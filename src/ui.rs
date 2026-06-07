//! Rendering. Styling is driven entirely by `app.theme`.

use std::time::Duration;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Padding, Paragraph},
    Frame,
};

use crate::app::{App, Mode};
use crate::library::Track;

/// Vertical bar glyphs from empty to full (eighths).
const BARS: [&str; 9] = [" ", "▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];

/// Cap on cover-art height (cells); width is ~2× this. Keeps the poster small.
const MAX_ART_ROWS: u16 = 6;

/// Visualizer bar geometry: each bar is this many cells wide, with a gap between.
const BAR_W: usize = 2;
const BAR_GAP: usize = 1;

pub fn render(f: &mut Frame, app: &mut App) {
    let area = f.area();

    // Guard against terminals too small to lay out without clipping.
    if area.width < 24 || area.height < 8 {
        let msg = Paragraph::new("melo\n\nwindow too small —\nzoom out / enlarge")
            .alignment(Alignment::Center)
            .style(Style::default().fg(app.theme.muted));
        f.render_widget(msg, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // header
            Constraint::Min(3),    // library + side panel
            Constraint::Length(4), // now playing
            Constraint::Length(1), // footer / search
        ])
        .split(area);

    render_header(f, app, chunks[0]);
    render_body(f, app, chunks[1]);
    render_now_playing(f, app, chunks[2]);
    render_footer(f, app, chunks[3]);
}

fn render_body(f: &mut Frame, app: &mut App, area: Rect) {
    // Show the side panel only when there's comfortable horizontal room.
    let panel_w = if area.width >= 100 {
        42
    } else if area.width >= 78 {
        34
    } else {
        0
    };

    if panel_w == 0 {
        render_library(f, app, area);
        return;
    }

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(panel_w)])
        .split(area);

    render_library(f, app, cols[0]);
    render_side_panel(f, app, cols[1]);
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let version = format!("v{} ", env!("CARGO_PKG_VERSION"));

    // Title + library path on the left, version pinned to the right.
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(version.chars().count() as u16)])
        .split(area);

    let left = Line::from(vec![
        Span::styled(" ♪ melo ", Style::default().fg(t.bg).bg(t.accent).bold()),
        Span::raw("  "),
        Span::styled(app.root.display().to_string(), Style::default().fg(t.muted)),
    ]);
    f.render_widget(Paragraph::new(left), cols[0]);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(version, Style::default().fg(t.muted))))
            .alignment(Alignment::Right),
        cols[1],
    );
}

fn render_library(f: &mut Frame, app: &mut App, area: Rect) {
    let t = app.theme.clone();

    // Usable text width per row: inner width minus the 2 borders and the
    // 1-cell highlight symbol the List reserves on every row.
    let usable = (area.width as usize).saturating_sub(3);

    // Responsive columns: drop the duration, then the artist, as space shrinks
    // so text never spills past the edge and gets clipped.
    let show_dur = usable >= 30;
    let show_artist = usable >= 22;
    let marker_w = 2;
    let dur_w = if show_dur { 8 } else { 0 }; // "  m:ss" right-aligned + gap

    let (title_w, artist_w) = if show_artist {
        let rest = usable.saturating_sub(marker_w + dur_w + 2); // 2 = gap before artist
        let artist_w = (rest * 6 / 20).clamp(8, 30);
        (rest.saturating_sub(artist_w), artist_w)
    } else {
        (usable.saturating_sub(marker_w + dur_w), 0)
    };

    let items: Vec<ListItem> = app
        .filtered
        .iter()
        .map(|&idx| {
            let track = &app.tracks[idx];
            let is_current = app.current == Some(idx);
            let marker = if is_current {
                if app.player.is_paused() {
                    "⏸ "
                } else {
                    "▶ "
                }
            } else {
                "  "
            };

            let base = if is_current {
                Style::default().fg(t.accent).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.fg)
            };

            let mut spans = vec![
                Span::styled(marker, Style::default().fg(t.accent)),
                Span::styled(pad_trunc(&track.title, title_w), base),
            ];
            if show_artist {
                spans.push(Span::raw("  "));
                spans.push(Span::styled(
                    pad_trunc(&track.artist, artist_w),
                    Style::default().fg(t.blue),
                ));
            }
            if show_dur {
                spans.push(Span::styled(
                    format!("  {:>6}", fmt_dur(track.duration)),
                    Style::default().fg(t.muted),
                ));
            }
            ListItem::new(Line::from(spans))
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.blue))
        .title(Span::styled(
            format!(" library · {} ", app.filtered.len()),
            Style::default().fg(t.blue).add_modifier(Modifier::BOLD),
        ));

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .fg(t.accent)
                .add_modifier(Modifier::REVERSED | Modifier::BOLD),
        )
        .highlight_symbol("▌");

    f.render_stateful_widget(list, area, &mut app.list_state);
}

fn render_side_panel(f: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    render_info(f, app, rows[0]);
    render_visualizer(f, app, rows[1]);
}

fn render_info(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.magenta))
        .title(Span::styled(
            " track ",
            Style::default().fg(t.magenta).add_modifier(Modifier::BOLD),
        ))
        .padding(Padding::horizontal(1));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let Some(idx) = app.display_idx() else {
        f.render_widget(
            Paragraph::new(Span::styled("Nothing selected", Style::default().fg(t.muted))),
            inner,
        );
        return;
    };
    let track = &app.tracks[idx];

    // Reserve the lower rows for text; give a small, capped square to the cover.
    let text_rows: u16 = 5;
    let art_rows = inner
        .height
        .saturating_sub(text_rows)
        .min(inner.width / 2)
        .min(MAX_ART_ROWS);

    let art_area = Rect {
        height: art_rows,
        ..inner
    };
    let text_area = Rect {
        y: inner.y + art_rows,
        height: inner.height.saturating_sub(art_rows),
        ..inner
    };

    render_cover(f, app, idx, art_area);

    let w = text_area.width as usize;
    let mut lines = vec![
        Line::from(Span::styled(
            trunc(&track.title, w),
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            trunc(&track.artist, w),
            Style::default().fg(t.blue),
        )),
        Line::from(Span::styled(
            trunc(&track.album, w),
            Style::default().fg(t.magenta),
        )),
        Line::from(Span::styled(
            trunc(&format_line(track), w),
            Style::default().fg(t.cyan),
        )),
    ];
    if app.current == Some(idx) {
        lines.push(Line::from(Span::styled(
            format!("{} / {}", fmt_dur(app.player.position()), fmt_dur(track.duration)),
            Style::default().fg(t.green),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            fmt_dur(track.duration),
            Style::default().fg(t.muted),
        )));
    }

    f.render_widget(Paragraph::new(lines), text_area);
}

fn render_cover(f: &mut Frame, app: &App, idx: usize, area: Rect) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let t = &app.theme;

    let art = app.art_cache.get(&idx).and_then(|o| o.as_ref());
    let art_cols = (area.height * 2).min(area.width);
    let centered = Rect {
        x: area.x + (area.width - art_cols) / 2,
        width: art_cols,
        ..area
    };

    match art {
        Some(img) => {
            let lines = crate::art::render(img, centered.width, centered.height);
            f.render_widget(Paragraph::new(lines), centered);
        }
        None => {
            // Placeholder: a faint frame with a centered note.
            let mut lines = Vec::with_capacity(area.height as usize);
            let mid = area.height / 2;
            for r in 0..area.height {
                if r == mid {
                    lines.push(Line::from(Span::styled(
                        "♪",
                        Style::default().fg(t.muted),
                    )));
                } else {
                    lines.push(Line::from(""));
                }
            }
            let para = Paragraph::new(lines).alignment(ratatui::layout::Alignment::Center);
            f.render_widget(para, area);
        }
    }
}

fn render_visualizer(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.cyan))
        .title(Span::styled(
            " visualizer ",
            Style::default().fg(t.cyan).add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let w = inner.width as usize;
    let h = inner.height as usize;
    if w == 0 || h == 0 {
        return;
    }

    // Bigger bars: BAR_W cells wide, separated by a gap.
    let unit = BAR_W + BAR_GAP;
    let n_bars = ((w + BAR_GAP) / unit).max(1);

    // Downsample the analyzer bands into the (fewer, wider) bar slots.
    let bands = &app.viz.bands;
    let nb = bands.len().max(1);
    let mut levels = vec![0.0f32; n_bars];
    for (i, slot) in levels.iter_mut().enumerate() {
        let lo = i * nb / n_bars;
        let hi = ((i + 1) * nb / n_bars).max(lo + 1).min(nb);
        let sum: f32 = bands[lo..hi].iter().sum();
        *slot = sum / (hi - lo) as f32;
    }

    // 3-tap spatial smoothing → rounded hills instead of jagged spikes.
    let eighths: Vec<u32> = (0..n_bars)
        .map(|i| {
            let l = levels[i.saturating_sub(1)];
            let c = levels[i];
            let r = levels[(i + 1).min(n_bars - 1)];
            let v = (l * 0.25 + c * 0.5 + r * 0.25).clamp(0.0, 1.0);
            (v * h as f32 * 8.0).round() as u32
        })
        .collect();

    // Single color — the accent, matching the library selection highlight.
    let style = Style::default().fg(t.accent);

    let mut lines = Vec::with_capacity(h);
    for y in 0..h {
        let row_from_bottom = (h - 1 - y) as u32;

        let mut spans = Vec::with_capacity(w);
        for (i, &e) in eighths.iter().enumerate() {
            let cell_base = row_from_bottom * 8;
            let fill = e.saturating_sub(cell_base).min(8) as usize;
            for _ in 0..BAR_W {
                spans.push(Span::styled(BARS[fill], style));
            }
            if i + 1 < n_bars {
                for _ in 0..BAR_GAP {
                    spans.push(Span::raw(" "));
                }
            }
        }
        lines.push(Line::from(spans));
    }

    f.render_widget(Paragraph::new(lines), inner);
}

fn render_now_playing(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.accent))
        .title(Span::styled(
            " now playing ",
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        ))
        .padding(Padding::horizontal(1));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    let (pos, dur, ratio) = match app.current {
        Some(idx) => {
            let dur = app.tracks[idx].duration;
            let pos = app.player.position();
            let ratio = if dur.as_secs_f64() > 0.0 {
                (pos.as_secs_f64() / dur.as_secs_f64()).clamp(0.0, 1.0)
            } else {
                0.0
            };
            (pos, dur, ratio)
        }
        None => (Duration::ZERO, Duration::ZERO, 0.0),
    };

    // Title row: track on the left, playback status on the right.
    // Labels are muted; the values (volume %, on/off, repeat mode) are bold accent
    // so they're always clearly distinguishable.
    let lbl = Style::default().fg(t.muted);
    let val = Style::default().fg(t.accent).add_modifier(Modifier::BOLD);
    let vol = format!("{}%", (app.player.volume * 100.0).round() as i32);
    let shuf = if app.shuffle { "on" } else { "off" };
    let rep = app.repeat.label();
    let status_spans = vec![
        Span::styled("vol ", lbl),
        Span::styled(vol, val),
        Span::styled("   shuffle ", lbl),
        Span::styled(shuf, val),
        Span::styled("   repeat ", lbl),
        Span::styled(rep, val),
    ];
    let status_w = status_spans.iter().map(|s| s.width()).sum::<usize>() as u16;
    let title_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(status_w)])
        .split(rows[0]);

    // Build the title now that we know its available width, truncating with an
    // ellipsis so it never collides with the status indicators.
    let title_avail = title_cols[0].width as usize;
    let title_line = match app.current {
        Some(idx) => {
            let track = &app.tracks[idx];
            let icon = if app.player.is_paused() { "⏸ " } else { "▶ " };
            now_title_line(icon, &track.title, &track.artist, title_avail, t)
        }
        None => Line::from(Span::styled(
            trunc(
                "Nothing playing — press Enter to play the selected track",
                title_avail,
            ),
            Style::default().fg(t.muted),
        )),
    };

    f.render_widget(Paragraph::new(title_line), title_cols[0]);
    f.render_widget(
        Paragraph::new(Line::from(status_spans)).alignment(Alignment::Right),
        title_cols[1],
    );

    // Progress row: elapsed time | bar | total time.
    let cur_str = format!("{} ", fmt_dur(pos));
    let tot_str = format!(" {}", fmt_dur(dur));
    let prog_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(cur_str.chars().count() as u16),
            Constraint::Min(1),
            Constraint::Length(tot_str.chars().count() as u16),
        ])
        .split(rows[1]);

    f.render_widget(
        Paragraph::new(Span::styled(
            cur_str,
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        )),
        prog_cols[0],
    );
    // Custom bar: a solid accent line for the played part, dots for the rest.
    let bar_w = prog_cols[1].width as usize;
    let filled = ((ratio * bar_w as f64).round() as usize).min(bar_w);
    let bar_spans = vec![
        Span::styled("━".repeat(filled), Style::default().fg(t.accent)),
        Span::styled(
            "·".repeat(bar_w.saturating_sub(filled)),
            Style::default().fg(t.faint),
        ),
    ];
    f.render_widget(Paragraph::new(Line::from(bar_spans)), prog_cols[1]);

    f.render_widget(
        Paragraph::new(Span::styled(tot_str, Style::default().fg(t.muted))),
        prog_cols[2],
    );
}

fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;

    if app.confirm_quit {
        let line = Line::from(vec![
            Span::styled(" quit? ", Style::default().fg(t.bg).bg(t.red).bold()),
            Span::raw(" "),
            Span::styled(
                "press y / Enter to quit, any other key to cancel",
                Style::default().fg(t.fg),
            ),
        ]);
        f.render_widget(Paragraph::new(line), area);
        return;
    }

    let line = match app.mode {
        Mode::Search => Line::from(vec![
            Span::styled(" search ", Style::default().fg(t.bg).bg(t.yellow).bold()),
            Span::raw(" "),
            Span::styled(app.query.clone(), Style::default().fg(t.fg)),
            Span::styled("█", Style::default().fg(t.accent)),
        ]),
        Mode::Normal => {
            let keys = "↑↓ move  ⏎ play  space pause  n/b next/prev  ←→ seek  [ ] vol  s shuffle  r repeat  / search  R refresh  q quit";
            Line::from(vec![
                Span::raw(" "),
                Span::styled(app.status.clone(), Style::default().fg(t.green)),
                Span::raw("   "),
                Span::styled(keys, Style::default().fg(t.muted)),
            ])
        }
    };
    f.render_widget(Paragraph::new(line), area);
}

// ---- helpers ----------------------------------------------------------------

fn format_line(t: &Track) -> String {
    let mut parts = Vec::new();
    if !t.ext.is_empty() {
        parts.push(t.ext.to_uppercase());
    }
    if t.sample_rate > 0 {
        parts.push(format!("{:.1} kHz", t.sample_rate as f32 / 1000.0));
    }
    if t.channels > 0 {
        parts.push(format!("{}ch", t.channels));
    }
    if t.bitrate > 0 {
        parts.push(format!("{} kbps", t.bitrate));
    }
    parts.join(" · ")
}

fn fmt_dur(d: Duration) -> String {
    let secs = d.as_secs();
    format!("{}:{:02}", secs / 60, secs % 60)
}

/// Build the now-playing title line (`▶ Title — Artist`), fitting it into
/// `avail` cells. Drops/truncates the artist first, then ellipsizes the title.
fn now_title_line<'a>(
    icon: &'a str,
    title: &'a str,
    artist: &'a str,
    avail: usize,
    t: &crate::theme::Theme,
) -> Line<'a> {
    let title_style = Style::default().fg(t.fg).add_modifier(Modifier::BOLD);
    let mut spans = vec![Span::styled(icon.to_string(), Style::default().fg(t.accent))];

    let icon_w = icon.chars().count();
    let budget = avail.saturating_sub(icon_w);
    if budget == 0 {
        return Line::from(spans);
    }

    const SEP: &str = "  —  ";
    let sep_w = SEP.chars().count();
    let tw = title.chars().count();
    let aw = artist.chars().count();

    if tw + sep_w + aw <= budget {
        // Everything fits.
        spans.push(Span::styled(title.to_string(), title_style));
        spans.push(Span::styled(SEP, Style::default().fg(t.muted)));
        spans.push(Span::styled(artist.to_string(), Style::default().fg(t.blue)));
    } else if budget < sep_w + 6 {
        // Too tight for an artist column — show just the (truncated) title.
        spans.push(Span::styled(trunc(title, budget), title_style));
    } else {
        // Share the space: artist gets up to ~40%, title keeps the rest.
        let artist_w = (budget * 2 / 5).min(aw).max(3);
        let title_w = budget.saturating_sub(sep_w + artist_w);
        spans.push(Span::styled(trunc(title, title_w), title_style));
        spans.push(Span::styled(SEP, Style::default().fg(t.muted)));
        spans.push(Span::styled(trunc(artist, artist_w), Style::default().fg(t.blue)));
    }

    Line::from(spans)
}

/// Truncate to `width` chars with an ellipsis; no padding.
fn trunc(s: &str, width: usize) -> String {
    let count = s.chars().count();
    if width == 0 {
        return String::new();
    }
    if count > width {
        let take = width.saturating_sub(1);
        let mut out: String = s.chars().take(take).collect();
        out.push('…');
        out
    } else {
        s.to_string()
    }
}

/// Truncate (with ellipsis) or right-pad with spaces to exactly `width` chars.
fn pad_trunc(s: &str, width: usize) -> String {
    let count = s.chars().count();
    if count > width {
        trunc(s, width)
    } else {
        let mut out = s.to_string();
        out.extend(std::iter::repeat(' ').take(width - count));
        out
    }
}
