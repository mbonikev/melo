//! Desktop notifications on track change.
//!
//! The OS media OSD only pops on play/pause (PlaybackStatus changes), so it
//! misses next/prev. We send our own notification via `notify-send` (libnotify)
//! on every new track so the song name always shows. Fire-and-forget; if
//! `notify-send` isn't installed it just does nothing.

use std::process::Command;

pub fn track(title: &str, artist: &str, album: &str) {
    let summary = format!("♪ {title}");
    let body = if album.is_empty() || album == "Unknown Album" {
        artist.to_string()
    } else {
        format!("{artist} — {album}")
    };

    // Run in a detached thread so we never block rendering and the child is reaped.
    std::thread::spawn(move || {
        let _ = Command::new("notify-send")
            .arg("-a")
            .arg("melo")
            .arg("-i")
            .arg("multimedia-player")
            // Replace the previous melo popup instead of stacking on rapid skips.
            .arg("-h")
            .arg("string:x-canonical-private-synchronous:melo")
            .arg("-t")
            .arg("3000")
            .arg(&summary)
            .arg(&body)
            .status();
    });
}
