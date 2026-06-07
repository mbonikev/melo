//! Local music library: recursively scan a directory and read tags.

use std::path::{Path, PathBuf};
use std::time::Duration;

use lofty::prelude::*;
use walkdir::WalkDir;

#[derive(Clone)]
pub struct Track {
    pub path: PathBuf,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: Duration,
    pub sample_rate: u32,
    pub channels: u8,
    pub bitrate: u32,
    pub ext: String,
}

const AUDIO_EXTS: &[&str] = &[
    "mp3", "flac", "ogg", "oga", "opus", "wav", "m4a", "mp4", "aac", "wma", "alac", "aiff",
    "aif",
];

/// Recursively scan `root` for audio files, reading metadata for each.
pub fn scan(root: &Path) -> Vec<Track> {
    let mut tracks = Vec::new();

    for entry in WalkDir::new(root)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let is_audio = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| AUDIO_EXTS.contains(&e.to_lowercase().as_str()))
            .unwrap_or(false);
        if !is_audio {
            continue;
        }
        tracks.push(read_track(path));
    }

    tracks.sort_by(|a, b| {
        let ka = (
            a.artist.to_lowercase(),
            a.album.to_lowercase(),
            a.title.to_lowercase(),
        );
        let kb = (
            b.artist.to_lowercase(),
            b.album.to_lowercase(),
            b.title.to_lowercase(),
        );
        ka.cmp(&kb)
    });
    tracks
}

fn read_track(path: &Path) -> Track {
    let fallback_title = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown")
        .to_string();

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let mut track = Track {
        path: path.to_path_buf(),
        title: fallback_title,
        artist: "Unknown Artist".to_string(),
        album: "Unknown Album".to_string(),
        duration: Duration::ZERO,
        sample_rate: 0,
        channels: 0,
        bitrate: 0,
        ext,
    };

    if let Ok(tagged) = lofty::read_from_path(path) {
        let props = tagged.properties();
        track.duration = props.duration();
        track.sample_rate = props.sample_rate().unwrap_or(0);
        track.channels = props.channels().unwrap_or(0);
        track.bitrate = props.audio_bitrate().unwrap_or(0);
        if let Some(tag) = tagged.primary_tag().or_else(|| tagged.first_tag()) {
            if let Some(t) = tag.title() {
                if !t.trim().is_empty() {
                    track.title = t.to_string();
                }
            }
            if let Some(a) = tag.artist() {
                if !a.trim().is_empty() {
                    track.artist = a.to_string();
                }
            }
            if let Some(al) = tag.album() {
                if !al.trim().is_empty() {
                    track.album = al.to_string();
                }
            }
        }
    }

    track
}
