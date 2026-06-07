//! Theme loading. Prefers the active omarchy palette so `melo` matches your
//! desktop exactly; falls back to terminal ANSI colors on any other system,
//! which means it still tracks whatever theme your terminal is set to.

use std::collections::HashMap;
use std::fs;

use ratatui::style::Color;

#[derive(Clone)]
pub struct Theme {
    pub accent: Color,
    pub fg: Color,
    pub bg: Color,
    /// Secondary text — dimmer than `fg` but still clearly readable.
    pub muted: Color,
    /// Subtle chrome (borders, gauge troughs). Lower contrast than `muted`.
    pub faint: Color,
    pub red: Color,
    pub green: Color,
    pub yellow: Color,
    pub blue: Color,
    pub magenta: Color,
    pub cyan: Color,
    pub source: &'static str,
}

impl Default for Theme {
    fn default() -> Self {
        // ANSI fallback: these map to your terminal's palette, so the player
        // automatically follows whatever theme the terminal is using.
        Self {
            accent: Color::LightMagenta,
            fg: Color::Reset,
            bg: Color::Reset,
            muted: Color::Gray,
            faint: Color::DarkGray,
            red: Color::Red,
            green: Color::Green,
            yellow: Color::Yellow,
            blue: Color::Blue,
            magenta: Color::Magenta,
            cyan: Color::Cyan,
            source: "ansi",
        }
    }
}

impl Theme {
    pub fn load() -> Self {
        Self::from_omarchy().unwrap_or_default()
    }

    fn from_omarchy() -> Option<Self> {
        let home = dirs::home_dir()?;
        let path = home.join(".config/omarchy/current/theme/colors.toml");
        let text = fs::read_to_string(path).ok()?;

        let mut map: HashMap<String, String> = HashMap::new();
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with('#') || line.starts_with('[') {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                let v = v.trim().trim_matches('"').trim_matches('\'');
                map.insert(k.trim().to_string(), v.to_string());
            }
        }

        let get = |k: &str| map.get(k).and_then(|h| hex(h));
        let d = Theme::default();

        let fg = get("foreground").unwrap_or(d.fg);
        let bg = get("background").unwrap_or(d.bg);

        // Blend fg toward bg so secondary text is readable in light AND dark themes.
        let muted = blend(fg, bg, 0.32).unwrap_or(d.muted);
        let faint = blend(fg, bg, 0.55).unwrap_or(d.faint);

        Some(Self {
            accent: get("accent").unwrap_or(d.accent),
            fg,
            bg,
            muted,
            faint,
            red: get("color1").unwrap_or(d.red),
            green: get("color2").unwrap_or(d.green),
            yellow: get("color3").unwrap_or(d.yellow),
            blue: get("color4").unwrap_or(d.blue),
            magenta: get("color5").unwrap_or(d.magenta),
            cyan: get("color6").unwrap_or(d.cyan),
            source: "omarchy",
        })
    }
}

/// Linear blend between two RGB colors; `t` is the weight toward `b`.
/// Returns `None` if either color isn't RGB (e.g. ANSI `Reset`).
fn blend(a: Color, b: Color, t: f32) -> Option<Color> {
    match (a, b) {
        (Color::Rgb(ar, ag, ab), Color::Rgb(br, bg, bb)) => {
            let mix = |x: u8, y: u8| (x as f32 * (1.0 - t) + y as f32 * t).round() as u8;
            Some(Color::Rgb(mix(ar, br), mix(ag, bg), mix(ab, bb)))
        }
        _ => None,
    }
}

fn hex(s: &str) -> Option<Color> {
    let s = s.trim().trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}
