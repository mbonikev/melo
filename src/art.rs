//! Album-art extraction and terminal rendering via Unicode half-blocks.
//!
//! Half-blocks ("▀" with a foreground = top pixel and background = bottom pixel)
//! let one text cell show two vertical pixels, so cover art renders in *any*
//! terminal — including ones with no image protocol, like Alacritty.

use std::path::Path;

use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView};
use lofty::prelude::*;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

/// Load embedded cover art from a file's tags, if any.
pub fn load(path: &Path) -> Option<DynamicImage> {
    let tagged = lofty::read_from_path(path).ok()?;
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag())?;
    let pic = tag.pictures().first()?;
    image::load_from_memory(pic.data()).ok()
}

/// Render `img` into `cols` × `rows` cells of colored half-blocks.
///
/// A cell is roughly twice as tall as it is wide, and each cell holds two
/// stacked pixels, so a square image fills a region where `cols ≈ 2 * rows`.
pub fn render(img: &DynamicImage, cols: u16, rows: u16) -> Vec<Line<'static>> {
    if cols == 0 || rows == 0 {
        return Vec::new();
    }

    // Center-crop to a square so the cover isn't stretched.
    let (w, h) = img.dimensions();
    let side = w.min(h);
    let square = img.crop_imm((w - side) / 2, (h - side) / 2, side, side);

    let small = square
        .resize_exact(cols as u32, (rows as u32) * 2, FilterType::Triangle)
        .to_rgba8();

    let mut lines = Vec::with_capacity(rows as usize);
    for ry in 0..rows {
        let mut spans = Vec::with_capacity(cols as usize);
        for cx in 0..cols {
            let top = small.get_pixel(cx as u32, (ry as u32) * 2);
            let bot = small.get_pixel(cx as u32, (ry as u32) * 2 + 1);
            let style = Style::default()
                .fg(Color::Rgb(top[0], top[1], top[2]))
                .bg(Color::Rgb(bot[0], bot[1], bot[2]));
            spans.push(Span::styled("▀", style));
        }
        lines.push(Line::from(spans));
    }
    lines
}
