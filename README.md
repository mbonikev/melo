# melo

A stylish, keyboard-driven **TUI music player** for your **local** library.
No accounts, no streaming, no online fetching — point it at a folder of music
and play. It automatically follows your **omarchy** theme (and falls back to
your terminal's ANSI palette on any other system), so it always matches your
setup.

```
 ♪ melo   ~/Music
┌ library · 128 ───────────────────────────────────────────────┐
│▌ Midnight City            M83                Hurry Up    4:03 │
│  ▶ Outro                  M83                Hurry Up    8:04 │
│  Wait                     M83                Hurry Up    3:49 │
└──────────────────────────────────────────────────────────────┘
┌ now playing ─────────────────────────────────────────────────┐
│ ▶ Outro  —  M83                                              │
│ ━━━━━━━━━━━━━━━━━╸················  3:21 / 8:04  vol 100% …    │
└──────────────────────────────────────────────────────────────┘
```

## Features

- Pure local playback — `mp3`, `flac`, `ogg`/`oga`, `opus`, `wav`, `m4a`, `aac`, and more
- Reads title / artist / album / duration tags
- Play / pause, next / previous, seek, volume
- Shuffle and repeat (off / all / one)
- Fuzzy-ish search filter across title, artist, album
- Follows your omarchy theme automatically; ANSI fallback elsewhere
- Single self-contained binary — the only shared dependency is ALSA

## Usage

```sh
melo                 # play your XDG music dir (~/Music)
melo ~/path/to/music # play a specific folder
melo --scan ~/Music  # list the detected library and exit (no UI)
melo --help
```

### Keys

| Key | Action | Key | Action |
| --- | --- | --- | --- |
| `↑`/`↓` or `j`/`k` | move | `Enter` | play selected |
| `Space` | play / pause | `n` | next |
| `b` / `p` | previous | `←`/`→` or `h`/`l` | seek ∓5s |
| `[` / `]` or `+` / `-` | volume down / up | `s` | shuffle |
| `r` | repeat mode | `/` | search |
| `g` / `G` | top / bottom | `q` / `Esc` | quit |

## Install

### Arch (yay / AUR)

```sh
yay -S melo
```

### Debian / Ubuntu (.deb)

Download the `.deb` from the [releases page](https://github.com/mbonikev/melo/releases) and:

```sh
sudo apt install ./melo_0.1.0_amd64.deb
```

### From source (any Linux)

Requires the Rust toolchain and ALSA headers (`libasound2-dev` on Debian,
`alsa-lib` on Arch).

```sh
git clone https://github.com/mbonikev/melo
cd melo
cargo install --path .
```

## Theming

`melo` reads `~/.config/omarchy/current/theme/colors.toml` if present and maps
`accent`, `foreground`, and `color0..15` onto the UI. Switch your omarchy theme
and `melo` matches it the next time you launch. With no omarchy config it uses
the terminal's 16 ANSI colors, so it still tracks whatever your terminal theme
is.

## License

MIT
