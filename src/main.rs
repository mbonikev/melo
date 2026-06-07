//! melo — a stylish TUI music player for your local library.

mod app;
mod art;
mod audio;
mod library;
mod theme;
mod ui;
mod viz;

use std::io::{self, Stdout};
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use app::{App, Mode};

/// Frame interval while audio is playing (smooth ~33fps spectrum).
const TICK_ACTIVE: Duration = Duration::from_millis(33);
/// Frame interval when idle/paused — easy on the CPU.
const TICK_IDLE: Duration = Duration::from_millis(200);

fn main() -> Result<()> {
    let root = resolve_root();

    if let Some(arg) = std::env::args().nth(1) {
        if arg == "-h" || arg == "--help" {
            print_help();
            return Ok(());
        }
        if arg == "--scan" {
            return scan_report(root);
        }
    }

    let mut app = App::new(root)?;

    let mut terminal = setup_terminal()?;
    let res = run(&mut terminal, &mut app);
    restore_terminal(&mut terminal)?;
    res
}

fn resolve_root() -> PathBuf {
    // Accept `melo DIR`, `melo --scan DIR`, or no positional (use XDG music dir).
    let positional = std::env::args().skip(1).find(|a| !a.starts_with('-'));
    if let Some(dir) = positional {
        return PathBuf::from(dir);
    }
    dirs::audio_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn scan_report(root: PathBuf) -> Result<()> {
    let theme = theme::Theme::load();
    let tracks = library::scan(&root);
    println!("root:  {}", root.display());
    println!("theme: {}", theme.source);
    println!("found: {} tracks\n", tracks.len());
    for t in &tracks {
        let secs = t.duration.as_secs();
        println!(
            "  {:>2}:{:02}  {}  —  {}  [{}]",
            secs / 60,
            secs % 60,
            t.title,
            t.artist,
            t.album
        );
    }
    Ok(())
}

fn run(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui::render(f, app))?;

        let tick = if app.player.is_active() {
            TICK_ACTIVE
        } else {
            TICK_IDLE
        };

        if event::poll(tick)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.mode {
                        Mode::Normal => handle_normal(app, key.code, key.modifiers),
                        Mode::Search => handle_search(app, key.code),
                    }
                }
            }
        }

        app.on_tick();

        if app.should_quit {
            return Ok(());
        }
    }
}

fn handle_normal(app: &mut App, code: KeyCode, mods: KeyModifiers) {
    match code {
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Char('c') if mods.contains(KeyModifiers::CONTROL) => app.should_quit = true,
        KeyCode::Down | KeyCode::Char('j') => app.move_selection(1),
        KeyCode::Up | KeyCode::Char('k') => app.move_selection(-1),
        KeyCode::Char('g') => app.select_first(),
        KeyCode::Char('G') => app.select_last(),
        KeyCode::PageDown => app.move_selection(10),
        KeyCode::PageUp => app.move_selection(-10),
        KeyCode::Enter => app.play_selected(),
        KeyCode::Char(' ') => app.toggle_pause(),
        KeyCode::Char('n') => app.next_track(),
        KeyCode::Char('b') | KeyCode::Char('p') => app.prev_track(),
        KeyCode::Right | KeyCode::Char('l') => app.seek_relative(5),
        KeyCode::Left | KeyCode::Char('h') => app.seek_relative(-5),
        KeyCode::Char('+') | KeyCode::Char('=') | KeyCode::Char(']') => app.change_volume(0.05),
        KeyCode::Char('-') | KeyCode::Char('_') | KeyCode::Char('[') => app.change_volume(-0.05),
        KeyCode::Char('s') => app.toggle_shuffle(),
        KeyCode::Char('r') => app.cycle_repeat(),
        KeyCode::Char('/') => app.enter_search(),
        _ => {}
    }
}

fn handle_search(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => app.exit_search(false),
        KeyCode::Enter => app.exit_search(true),
        KeyCode::Backspace => app.search_backspace(),
        KeyCode::Char(c) => app.search_push(c),
        _ => {}
    }
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn print_help() {
    println!(
        "melo — a stylish TUI music player for local libraries\n\n\
         USAGE:\n    melo [MUSIC_DIR]\n\n\
         If no directory is given, melo uses your XDG music dir (~/Music).\n\n\
         KEYS:\n\
         \t↑/↓ or j/k   move          ⏎  play selected\n\
         \tspace        play/pause    n  next        b/p  previous\n\
         \t←/→ or h/l   seek ∓5s      [ / ] or +/- volume\n\
         \ts            shuffle       r  repeat (off/all/one)\n\
         \t/            search        q  quit\n"
    );
}
