//! Audio playback engine built on rodio + symphonia (pure Rust, no runtime deps).
//!
//! While playing, the decoded PCM stream is "tapped": each frame is downmixed
//! to mono and pushed into a shared ring buffer that the visualizer reads.

use std::collections::VecDeque;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result};
use rodio::source::SeekError;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};

/// How many mono samples to keep available for the visualizer.
const VIZ_CAP: usize = 4096;

pub type VizBuffer = Arc<Mutex<VecDeque<f32>>>;

pub struct Player {
    _stream: OutputStream,
    handle: OutputStreamHandle,
    sink: Sink,
    pub volume: f32,
    has_track: bool,
    viz: VizBuffer,
}

impl Player {
    pub fn new() -> Result<Self> {
        let (stream, handle) =
            OutputStream::try_default().context("could not open an audio output device")?;
        let sink = Sink::try_new(&handle)?;
        let volume = 1.0;
        sink.set_volume(volume);
        Ok(Self {
            _stream: stream,
            handle,
            sink,
            volume,
            has_track: false,
            viz: Arc::new(Mutex::new(VecDeque::with_capacity(VIZ_CAP))),
        })
    }

    pub fn play_file(&mut self, path: &Path) -> Result<()> {
        let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
        let decoder = Decoder::new(BufReader::new(file))
            .with_context(|| format!("decoding {}", path.display()))?;
        let source = decoder.convert_samples::<f32>();
        let channels = source.channels().max(1);

        if let Ok(mut b) = self.viz.lock() {
            b.clear();
        }

        let tap = Tap {
            inner: source,
            buf: Arc::clone(&self.viz),
            channels,
            ch_idx: 0,
            acc: 0.0,
        };

        let sink = Sink::try_new(&self.handle)?;
        sink.set_volume(self.volume);
        sink.append(tap);
        self.sink = sink;
        self.has_track = true;
        Ok(())
    }

    pub fn toggle_pause(&self) {
        if self.sink.is_paused() {
            self.sink.play();
        } else {
            self.sink.pause();
        }
    }

    pub fn is_paused(&self) -> bool {
        self.sink.is_paused()
    }

    /// Actively producing audio right now (used to drive the visualizer / frame rate).
    pub fn is_active(&self) -> bool {
        self.has_track && !self.sink.is_paused() && !self.sink.empty()
    }

    pub fn track_finished(&self) -> bool {
        self.has_track && self.sink.empty()
    }

    pub fn clear(&mut self) {
        self.sink.stop();
        self.has_track = false;
        if let Ok(mut b) = self.viz.lock() {
            b.clear();
        }
    }

    pub fn position(&self) -> Duration {
        self.sink.get_pos()
    }

    pub fn set_volume(&mut self, v: f32) {
        self.volume = v.clamp(0.0, 2.0);
        self.sink.set_volume(self.volume);
    }

    pub fn seek(&self, pos: Duration) {
        let _ = self.sink.try_seek(pos);
    }

    /// Snapshot of the most recent mono samples for the visualizer.
    pub fn samples(&self) -> Vec<f32> {
        self.viz
            .lock()
            .map(|b| b.iter().copied().collect())
            .unwrap_or_default()
    }
}

/// Source wrapper that copies a downmixed-to-mono copy of every frame into a
/// shared ring buffer as the audio is pulled by the output device.
struct Tap<S> {
    inner: S,
    buf: VizBuffer,
    channels: u16,
    ch_idx: u16,
    acc: f32,
}

impl<S> Iterator for Tap<S>
where
    S: Source<Item = f32>,
{
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let s = self.inner.next()?;
        self.acc += s;
        self.ch_idx += 1;
        if self.ch_idx >= self.channels {
            let mono = self.acc / self.channels as f32;
            self.acc = 0.0;
            self.ch_idx = 0;
            if let Ok(mut b) = self.buf.lock() {
                if b.len() >= VIZ_CAP {
                    b.pop_front();
                }
                b.push_back(mono);
            }
        }
        Some(s)
    }
}

impl<S> Source for Tap<S>
where
    S: Source<Item = f32>,
{
    fn current_frame_len(&self) -> Option<usize> {
        self.inner.current_frame_len()
    }
    fn channels(&self) -> u16 {
        self.inner.channels()
    }
    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }
    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }
    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        if let Ok(mut b) = self.buf.lock() {
            b.clear();
        }
        self.ch_idx = 0;
        self.acc = 0.0;
        self.inner.try_seek(pos)
    }
}
