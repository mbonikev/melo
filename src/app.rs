//! Application state and behavior.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Result;
use image::DynamicImage;
use ratatui::widgets::ListState;

use crate::audio::Player;
use crate::library::{self, Track};
use crate::media::{MediaCommand, MediaKeys};
use crate::theme::Theme;
use crate::viz::Visualizer;

/// How often to re-read the theme file so theme switches show up live.
const THEME_REFRESH: Duration = Duration::from_millis(750);

/// Number of frequency bars the spectrum analyzer computes.
const VIZ_BANDS: usize = 48;

#[derive(PartialEq, Eq)]
pub enum Mode {
    Normal,
    Search,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Repeat {
    Off,
    All,
    One,
}

impl Repeat {
    pub fn label(self) -> &'static str {
        match self {
            Repeat::Off => "off",
            Repeat::All => "all",
            Repeat::One => "one",
        }
    }
    fn next(self) -> Self {
        match self {
            Repeat::Off => Repeat::All,
            Repeat::All => Repeat::One,
            Repeat::One => Repeat::Off,
        }
    }
}

pub struct App {
    pub root: PathBuf,
    pub theme: Theme,
    pub tracks: Vec<Track>,
    /// Indices into `tracks`, in display order (after search filtering).
    pub filtered: Vec<usize>,
    pub list_state: ListState,
    /// Index into `tracks` of the currently loaded song.
    pub current: Option<usize>,
    pub player: Player,
    pub mode: Mode,
    pub query: String,
    pub repeat: Repeat,
    pub shuffle: bool,
    pub should_quit: bool,
    pub confirm_quit: bool,
    pub status: String,
    pub viz: Visualizer,
    pub art_cache: HashMap<usize, Option<DynamicImage>>,
    media: Option<MediaKeys>,
    media_last: Option<usize>,
    media_paused: Option<bool>,
    notified: Option<usize>,
    last_theme_check: Instant,
}

impl App {
    pub fn new(root: PathBuf) -> Result<Self> {
        let theme = Theme::load();
        let tracks = library::scan(&root);
        let player = Player::new()?;

        let mut list_state = ListState::default();
        if !tracks.is_empty() {
            list_state.select(Some(0));
        }

        let filtered = (0..tracks.len()).collect();
        let status = if tracks.is_empty() {
            format!("No audio files found in {}", root.display())
        } else {
            format!("{} tracks · theme: {}", tracks.len(), theme.source)
        };

        Ok(Self {
            root,
            theme,
            tracks,
            filtered,
            list_state,
            current: None,
            player,
            mode: Mode::Normal,
            query: String::new(),
            repeat: Repeat::Off,
            shuffle: false,
            should_quit: false,
            confirm_quit: false,
            status,
            viz: Visualizer::new(VIZ_BANDS),
            art_cache: HashMap::new(),
            media: MediaKeys::new(),
            media_last: None,
            media_paused: None,
            notified: None,
            last_theme_check: Instant::now(),
        })
    }

    /// Send a desktop notification whenever the playing track changes, so the
    /// song name shows on next/prev/auto-advance (the OS OSD only covers play/pause).
    fn maybe_notify(&mut self) {
        match self.current {
            Some(idx) if self.notified != Some(idx) => {
                let t = &self.tracks[idx];
                crate::notify::track(&t.title, &t.artist, &t.album);
                self.notified = Some(idx);
            }
            None => self.notified = None,
            _ => {}
        }
    }

    /// Pending desktop media-key commands (empty if MPRIS is unavailable).
    pub fn poll_media(&self) -> Vec<MediaCommand> {
        self.media.as_ref().map(|m| m.poll()).unwrap_or_default()
    }

    pub fn handle_media(&mut self, cmd: MediaCommand) {
        match cmd {
            MediaCommand::Toggle => {
                self.toggle_pause();
                self.status = if self.player.is_paused() {
                    "Paused".into()
                } else {
                    "Playing".into()
                };
            }
            MediaCommand::Play => {
                if self.current.is_none() {
                    self.play_selected();
                } else {
                    self.player.play();
                    self.status = "Playing".into();
                }
            }
            MediaCommand::Pause => {
                self.player.pause();
                self.status = "Paused".into();
            }
            // next_track / prev_track already set the "Playing <title>" status.
            MediaCommand::Next => self.next_track(),
            MediaCommand::Prev => self.prev_track(),
            MediaCommand::Stop => {
                self.player.clear();
                self.current = None;
                self.status = "Stopped".into();
            }
        }
        // Push the new state to the desktop immediately (name, play/stop state).
        self.sync_media();
    }

    /// Push current playback state to the desktop (only when it changes).
    fn sync_media(&mut self) {
        let Some(media) = self.media.as_mut() else {
            return;
        };
        match self.current {
            Some(idx) => {
                let paused = self.player.is_paused();
                if self.media_last != Some(idx) || self.media_paused != Some(paused) {
                    let t = &self.tracks[idx];
                    media.set_now_playing(
                        &t.title,
                        &t.artist,
                        &t.album,
                        t.duration,
                        !paused,
                        self.player.position(),
                    );
                    self.media_last = Some(idx);
                    self.media_paused = Some(paused);
                }
            }
            None => {
                if self.media_last.is_some() {
                    media.set_stopped();
                    self.media_last = None;
                    self.media_paused = None;
                }
            }
        }
    }

    /// Track index (into `tracks`) currently shown in the info/art panel:
    /// the playing track, or the highlighted one when nothing is playing.
    pub fn selected_track_idx(&self) -> Option<usize> {
        self.list_state
            .selected()
            .and_then(|p| self.filtered.get(p).copied())
    }

    pub fn display_idx(&self) -> Option<usize> {
        self.current.or_else(|| self.selected_track_idx())
    }

    /// Re-scan the library folder without restarting playback.
    pub fn refresh(&mut self) {
        let playing_path = self.current.map(|i| self.tracks[i].path.clone());
        let selected_path = self.selected_track_idx().map(|i| self.tracks[i].path.clone());

        self.tracks = library::scan(&self.root);
        self.art_cache.clear();
        self.theme = Theme::load();
        self.apply_filter();

        // Keep the currently playing song highlighted/tracked across the rescan.
        self.current = playing_path.and_then(|p| self.tracks.iter().position(|t| t.path == p));
        if let Some(sel) = selected_path {
            if let Some(pos) = self.filtered.iter().position(|&i| self.tracks[i].path == sel) {
                self.list_state.select(Some(pos));
            }
        }

        self.status = format!("Refreshed · {} tracks", self.tracks.len());
    }

    // ---- quit confirmation --------------------------------------------------

    /// Ask before quitting, so a stray key doesn't kill playback.
    pub fn request_quit(&mut self) {
        self.confirm_quit = true;
    }

    pub fn cancel_quit(&mut self) {
        self.confirm_quit = false;
    }

    pub fn confirm_quit_now(&mut self) {
        self.should_quit = true;
    }

    // ---- selection ----------------------------------------------------------

    pub fn selected_pos(&self) -> Option<usize> {
        self.list_state.selected()
    }

    pub fn move_selection(&mut self, delta: isize) {
        if self.filtered.is_empty() {
            return;
        }
        let len = self.filtered.len() as isize;
        let cur = self.list_state.selected().unwrap_or(0) as isize;
        let next = (cur + delta).rem_euclid(len);
        self.list_state.select(Some(next as usize));
    }

    pub fn select_first(&mut self) {
        if !self.filtered.is_empty() {
            self.list_state.select(Some(0));
        }
    }

    pub fn select_last(&mut self) {
        if !self.filtered.is_empty() {
            self.list_state.select(Some(self.filtered.len() - 1));
        }
    }

    // ---- playback -----------------------------------------------------------

    pub fn play_selected(&mut self) {
        if let Some(pos) = self.selected_pos() {
            self.play_filtered_pos(pos);
        }
    }

    /// Play the track at position `pos` within the filtered list.
    fn play_filtered_pos(&mut self, pos: usize) {
        let Some(&track_idx) = self.filtered.get(pos) else {
            return;
        };
        let path = self.tracks[track_idx].path.clone();
        match self.player.play_file(&path) {
            Ok(()) => {
                self.current = Some(track_idx);
                let t = &self.tracks[track_idx];
                self.status = format!("Playing  {} — {}", t.title, t.artist);
            }
            Err(e) => {
                self.status = format!("Error: {e}");
            }
        }
    }

    pub fn toggle_pause(&mut self) {
        if self.current.is_none() {
            self.play_selected();
            return;
        }
        self.player.toggle_pause();
    }

    /// Position of the currently playing track within the filtered list.
    fn current_filtered_pos(&self) -> Option<usize> {
        let cur = self.current?;
        self.filtered.iter().position(|&i| i == cur)
    }

    pub fn next_track(&mut self) {
        if self.filtered.is_empty() {
            return;
        }
        if self.shuffle {
            let pos = fastrand::usize(..self.filtered.len());
            self.play_filtered_pos(pos);
            return;
        }
        let cur = self.current_filtered_pos().unwrap_or(usize::MAX);
        let next = if cur == usize::MAX {
            0
        } else {
            (cur + 1) % self.filtered.len()
        };
        self.play_filtered_pos(next);
    }

    pub fn prev_track(&mut self) {
        if self.filtered.is_empty() {
            return;
        }
        // If we're more than 3s into a song, restart it instead of going back.
        if self.current.is_some() && self.player.position() > Duration::from_secs(3) {
            let cur = self.current_filtered_pos();
            if let Some(pos) = cur {
                self.play_filtered_pos(pos);
                return;
            }
        }
        let cur = self.current_filtered_pos().unwrap_or(0);
        let len = self.filtered.len();
        let prev = (cur + len - 1) % len;
        self.play_filtered_pos(prev);
    }

    pub fn seek_relative(&mut self, secs: i64) {
        if self.current.is_none() {
            return;
        }
        let pos = self.player.position();
        let target = if secs >= 0 {
            pos + Duration::from_secs(secs as u64)
        } else {
            pos.saturating_sub(Duration::from_secs((-secs) as u64))
        };
        self.player.seek(target);
    }

    pub fn change_volume(&mut self, delta: f32) {
        let v = self.player.volume + delta;
        self.player.set_volume(v);
        self.status = format!("Volume {}%", (self.player.volume * 100.0).round() as i32);
    }

    pub fn toggle_shuffle(&mut self) {
        self.shuffle = !self.shuffle;
        self.status = format!("Shuffle {}", if self.shuffle { "on" } else { "off" });
    }

    pub fn cycle_repeat(&mut self) {
        self.repeat = self.repeat.next();
        self.status = format!("Repeat {}", self.repeat.label());
    }

    /// Called every frame: advance the queue, update the spectrum, preload art.
    pub fn on_tick(&mut self) {
        self.advance_if_finished();

        // Live-reload the theme so omarchy theme/light-dark switches apply at runtime.
        if self.last_theme_check.elapsed() >= THEME_REFRESH {
            self.theme = Theme::load();
            self.last_theme_check = Instant::now();
        }

        self.sync_media();
        self.maybe_notify();

        if self.player.is_active() {
            let samples = self.player.samples();
            self.viz.update(&samples);
        } else {
            self.viz.decay();
        }

        if let Some(idx) = self.display_idx() {
            if !self.art_cache.contains_key(&idx) {
                if self.art_cache.len() > 48 {
                    self.art_cache.clear();
                }
                let art = crate::art::load(&self.tracks[idx].path);
                self.art_cache.insert(idx, art);
            }
        }
    }

    fn advance_if_finished(&mut self) {
        if self.current.is_some() && self.player.track_finished() {
            match self.repeat {
                Repeat::One => {
                    if let Some(pos) = self.current_filtered_pos() {
                        self.play_filtered_pos(pos);
                    }
                }
                Repeat::All => self.next_track(),
                Repeat::Off => {
                    let cur = self.current_filtered_pos();
                    let is_last = cur.map(|p| p + 1 >= self.filtered.len()).unwrap_or(true);
                    if is_last {
                        self.player.clear();
                        self.current = None;
                        self.status = "End of library".to_string();
                    } else {
                        self.next_track();
                    }
                }
            }
        }
    }

    // ---- search -------------------------------------------------------------

    pub fn enter_search(&mut self) {
        self.mode = Mode::Search;
    }

    pub fn exit_search(&mut self, keep: bool) {
        self.mode = Mode::Normal;
        if !keep {
            self.query.clear();
            self.apply_filter();
        }
    }

    pub fn search_push(&mut self, c: char) {
        self.query.push(c);
        self.apply_filter();
    }

    pub fn search_backspace(&mut self) {
        self.query.pop();
        self.apply_filter();
    }

    fn apply_filter(&mut self) {
        let q = self.query.to_lowercase();
        self.filtered = self
            .tracks
            .iter()
            .enumerate()
            .filter(|(_, t)| {
                q.is_empty()
                    || t.title.to_lowercase().contains(&q)
                    || t.artist.to_lowercase().contains(&q)
                    || t.album.to_lowercase().contains(&q)
            })
            .map(|(i, _)| i)
            .collect();

        if self.filtered.is_empty() {
            self.list_state.select(None);
        } else {
            self.list_state.select(Some(0));
        }
    }
}
