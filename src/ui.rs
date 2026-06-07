//! Rendering. Styling is driven entirely by `app.theme`.

use std::time::Duration;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    symbols,
    text::{Line, Span},
    widgets::{Block, Borders, LineGauge, List, ListItem, Padding, Paragraph},
    Frame,
};

use crate::app::{App, Mode};
use crate::library::Track;

/// Vertical bar glyphs from empty to full (eighths).
const BARS: [&str; 9] = [" ", "▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];

pub fn render(f: &mut Frame, app: &mut App) {
    let area = f.area();

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
    let line = Line::from(vec![
        Span::styled(" ♪ melo ", Style::default().fg(t.bg).bg(t.accent).bold()),
        Span::raw("  "),
        Span::styled(app.root.display().to_string(), Style::default().fg(t.dim)),
    ]);
    f.render_widget(Paragraph::new(line), area);
}

fn render_library(f: &mut Frame, app: &mut App, area: Rect) {
    let t = app.theme.clone();

    let avail = area.width.saturating_sub(4) as usize;
    let title_w = (avail * 9 / 20).clamp(14, 64);
    let artist_w = (avail * 6 / 20).clamp(8, 40);

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

            let line = Line::from(vec![
                Span::styled(marker, Style::default().fg(t.accent)),
                Span::styled(pad_trunc(&track.title, title_w), base),
                Span::raw("  "),
                Span::styled(pad_trunc(&track.artist, artist_w), Style::default().fg(t.blue)),
                Span::raw("  "),
                Span::styled(format!("{:>6}", fmt_dur(track.duration)), Style::default().fg(t.dim)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.dim))
        .title(Span::styled(
            format!(" library · {} ", app.filtered.len()),
            Style::default().fg(t.accent),
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
        .border_style(Style::default().fg(t.dim))
        .title(Span::styled(" track ", Style::default().fg(t.accent)))
        .padding(Padding::horizontal(1));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let Some(idx) = app.display_idx() else {
        f.render_widget(
            Paragraph::new(Span::styled(
                "Nothing selected",
                Style::default().fg(t.dim),
            )),
            inner,
        );
        return;
    };
    let track = &app.tracks[idx];

    // Reserve the lower rows for text; give the rest to the cover art.
    let text_rows: u16 = 5;
    let art_rows = inner
        .height
        .saturating_sub(text_rows)
        .min(inner.width / 2);

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

    let dim = Style::default().fg(t.dim);
    let mut lines = vec![
        Line::from(Span::styled(
            trunc(&track.title, text_area.width as usize),
            Style::default().fg(t.fg).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            trunc(&track.artist, text_area.width as usize),
            Style::default().fg(t.blue),
        )),
        Line::from(Span::styled(
            trunc(&track.album, text_area.width as usize),
            dim,
        )),
        Line::from(Span::styled(trunc(&format_line(track), text_area.width as usize), dim)),
    ];
    if app.current == Some(idx) {
        lines.push(Line::from(Span::styled(
            format!("{} / {}", fmt_dur(app.player.position()), fmt_dur(track.duration)),
            Style::default().fg(t.green),
        )));
    } else {
        lines.push(Line::from(Span::styled(fmt_dur(track.duration), dim)));
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
                        Style::default().fg(t.dim).add_modifier(Modifier::DIM),
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
        .border_style(Style::default().fg(t.dim))
        .title(Span::styled(" visualizer ", Style::default().fg(t.accent)));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let w = inner.width as usize;
    let h = inner.height;
    if w == 0 || h == 0 {
        return;
    }

    let bands = &app.viz.bands;
    let n = bands.len().max(1);

    // Per-column bar height in eighth-row units.
    let mut col_eighths = Vec::with_capacity(w);
    for c in 0..w {
        let bi = c * n / w;
        let level = bands.get(bi).copied().unwrap_or(0.0).clamp(0.0, 1.0);
        col_eighths.push((level * h as f32 * 8.0).round() as u32);
    }

    let mut lines = Vec::with_capacity(h as usize);
    for y in 0..h {
        let row_from_bottom = (h - 1 - y) as u32;
        let frac = (row_from_bottom as f32 + 0.5) / h as f32;
        let color = if frac < 0.5 {
            t.green
        } else if frac < 0.8 {
            t.yellow
        } else {
            t.red
        };
        let style = Style::default().fg(color);

        let mut spans = Vec::with_capacity(w);
        for &eighths in &col_eighths {
            let cell_base = row_from_bottom * 8;
            let fill = eighths.saturating_sub(cell_base).min(8) as usize;
            spans.push(Span::styled(BARS[fill], style));
        }
        lines.push(Line::from(spans));
    }

    f.render_widget(Paragraph::new(lines), inner);
}

fn render_now_playing(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.dim))
        .title(Span::styled(" now playing ", Style::default().fg(t.accent)))
        .padding(Padding::horizontal(1));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    let (pos, dur, ratio, title_line) = match app.current {
        Some(idx) => {
            let track = &app.tracks[idx];
            let pos = app.player.position();
            let dur = track.duration;
            let ratio = if dur.as_secs_f64() > 0.0 {
                (pos.as_secs_f64() / dur.as_secs_f64()).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let icon = if app.player.is_paused() { "⏸" } else { "▶" };
            let line = Line::from(vec![
                Span::styled(format!("{icon} "), Style::default().fg(t.accent)),
                Span::styled(
                    track.title.clone(),
                    Style::default().fg(t.fg).add_modifier(Modifier::BOLD),
                ),
                Span::styled("  —  ", Style::default().fg(t.dim)),
                Span::styled(track.artist.clone(), Style::default().fg(t.blue)),
            ]);
            (pos, dur, ratio, line)
        }
        None => {
            let line = Line::from(Span::styled(
                "Nothing playing — press Enter to play the selected track",
                Style::default().fg(t.dim),
            ));
            (Duration::ZERO, Duration::ZERO, 0.0, line)
        }
    };

    f.render_widget(Paragraph::new(title_line), rows[0]);

    let label = format!(
        "{} / {}   vol {}%   shuffle {}   repeat {}",
        fmt_dur(pos),
        fmt_dur(dur),
        (app.player.volume * 100.0).round() as i32,
        if app.shuffle { "on" } else { "off" },
        app.repeat.label(),
    );

    let gauge = LineGauge::default()
        .filled_style(Style::default().fg(t.accent))
        .unfilled_style(Style::default().fg(t.dim))
        .line_set(symbols::line::THICK)
        .label(Span::styled(label, Style::default().fg(t.dim)))
        .ratio(ratio);
    f.render_widget(gauge, rows[1]);
}

fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let line = match app.mode {
        Mode::Search => Line::from(vec![
            Span::styled(" search ", Style::default().fg(t.bg).bg(t.yellow).bold()),
            Span::raw(" "),
            Span::styled(app.query.clone(), Style::default().fg(t.fg)),
            Span::styled("█", Style::default().fg(t.accent)),
        ]),
        Mode::Normal => {
            let keys = "↑↓ move  ⏎ play  space pause  n/b next/prev  ←→ seek  [ ] vol  s shuffle  r repeat  / search  q quit";
            Line::from(vec![
                Span::raw(" "),
                Span::styled(app.status.clone(), Style::default().fg(t.green)),
                Span::raw("   "),
                Span::styled(keys, Style::default().fg(t.dim)),
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
