//! Desktop media-key integration via MPRIS (D-Bus).
//!
//! Keyboard media keys (play/pause, next, prev) are captured by the desktop and
//! delivered over MPRIS, not to the terminal — so melo registers as an MPRIS
//! player. If there's no D-Bus session bus, this degrades to `None` silently.

use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

use souvlaki::{
    MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig,
};

/// Commands translated from desktop media-key events.
pub enum MediaCommand {
    Toggle,
    Play,
    Pause,
    Next,
    Prev,
    Stop,
}

pub struct MediaKeys {
    controls: MediaControls,
    rx: Receiver<MediaControlEvent>,
}

impl MediaKeys {
    pub fn new() -> Option<Self> {
        let config = PlatformConfig {
            dbus_name: "melo",
            display_name: "melo",
            hwnd: None,
        };
        let mut controls = MediaControls::new(config).ok()?;

        let (tx, rx) = channel();
        controls
            .attach(move |event: MediaControlEvent| {
                let _ = tx.send(event);
            })
            .ok()?;

        Some(Self { controls, rx })
    }

    /// Drain pending media-key events into our command enum.
    pub fn poll(&self) -> Vec<MediaCommand> {
        let mut cmds = Vec::new();
        while let Ok(event) = self.rx.try_recv() {
            let cmd = match event {
                MediaControlEvent::Toggle => MediaCommand::Toggle,
                MediaControlEvent::Play => MediaCommand::Play,
                MediaControlEvent::Pause => MediaCommand::Pause,
                MediaControlEvent::Next => MediaCommand::Next,
                MediaControlEvent::Previous => MediaCommand::Prev,
                MediaControlEvent::Stop | MediaControlEvent::Quit => MediaCommand::Stop,
                _ => continue,
            };
            cmds.push(cmd);
        }
        cmds
    }

    pub fn set_now_playing(
        &mut self,
        title: &str,
        artist: &str,
        album: &str,
        duration: Duration,
        playing: bool,
        position: Duration,
    ) {
        let _ = self.controls.set_metadata(MediaMetadata {
            title: Some(title),
            artist: Some(artist),
            album: Some(album),
            cover_url: None,
            duration: Some(duration),
        });
        let progress = Some(MediaPosition(position));
        let playback = if playing {
            MediaPlayback::Playing { progress }
        } else {
            MediaPlayback::Paused { progress }
        };
        let _ = self.controls.set_playback(playback);
    }

    pub fn set_stopped(&mut self) {
        let _ = self.controls.set_playback(MediaPlayback::Stopped);
    }
}
