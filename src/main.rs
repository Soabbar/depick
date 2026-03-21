mod app;
mod domain;
mod infra;
mod ui;

use anyhow::Result;
use app::{AppState, Phase, TargetMode};
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use infra::pm::{self, PackageManager};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    Terminal,
};
use std::{
    io::{self, Write},
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Parser)]
#[command(
    name = "depick",
    about = "Selective dependency updates for npm and bun"
)]
struct Cli {
    #[arg(short, long)]
    dir: Option<PathBuf>,
    #[arg(long, value_parser = ["npm", "bun"])]
    pm: Option<String>,
    #[arg(long, value_parser = ["latest", "wanted"], default_value = "latest")]
    target: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let dir = cli
        .dir
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let detected_pm = match cli.pm.as_deref() {
        Some("bun") => PackageManager::Bun,
        Some("npm") => PackageManager::Npm,
        _ => pm::detect(&dir),
    };

    let target_mode = match cli.target.as_str() {
        "wanted" => TargetMode::Wanted,
        _ => TargetMode::Latest,
    };

    let project_name = dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_string());

    // ── Pre-TUI startup screen ────────────────────────────────────────────────
    print_startup_screen(&project_name, detected_pm.label())?;

    let items = match detected_pm {
        PackageManager::Npm => pm::npm_outdated(&dir, &target_mode),
        PackageManager::Bun => {
            pm::npm_outdated(&dir, &target_mode).or_else(|_| pm::bun_outdated(&dir, &target_mode))
        }
    }?;

    if items.is_empty() {
        // Clear the startup screen line and print a clean message
        eprint!("\r\x1b[2K");
        println!("No outdated direct dependencies found. Your project is up to date.");
        return Ok(());
    }

    // ── Preload all release contexts with shimmer animation ───────────────────
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()?;

    let total = items.len();

    // One background thread: sends progress ticks through `tx` and returns the
    // fully-populated store when done. The main thread animates while it waits.
    let (tx, rx) = std::sync::mpsc::channel::<usize>();
    let items_clone = items.clone();
    let http_clone = http.clone();

    let handle = std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio rt");
        rt.block_on(infra::release::fetch_all_release_contexts(
            &http_clone,
            &items_clone,
            |done, _total| {
                let _ = tx.send(done); // best-effort; ignore if main has moved on
            },
        ))
    });

    let shimmer_start = Instant::now();
    let mut last_done: usize = 0;

    let release_contexts = loop {
        // Drain all progress ticks that arrived since the last frame.
        while let Ok(n) = rx.try_recv() {
            last_done = n;
        }

        let elapsed = shimmer_start.elapsed().as_millis() as u64;
        animate_shimmer(elapsed, last_done, total)?;

        if handle.is_finished() {
            let elapsed = shimmer_start.elapsed().as_millis() as u64;
            animate_shimmer(elapsed, total, total)?;
            break handle.join().expect("fetch thread panicked");
        }

        std::thread::sleep(Duration::from_millis(80));
    };

    // Move cursor past the startup block before entering raw mode
    eprint!("\r\x1b[5B\x1b[2K");
    let _ = io::stderr().flush();

    // ── Hand off to the TUI ───────────────────────────────────────────────────
    let mut state = AppState::new(project_name, dir.clone(), detected_pm, items);
    state.release_contexts = release_contexts;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, &mut state);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {e}");
    }
    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut AppState,
) -> Result<()> {
    loop {
        // ── 1. Render ─────────────────────────────────────────────────────
        terminal.draw(|f| draw(f, state))?;

        // ── 2. Apply updates — redraw after each package so log is live ───
        if state.phase == Phase::Applying {
            apply_updates(terminal, state)?;
            state.phase = Phase::Done;
        }

        // ── 4. Poll for input ─────────────────────────────────────────────
        if !event::poll(Duration::from_millis(50))? {
            continue;
        }

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            // Modal intercepts all keys when open
            if state.show_notes_modal {
                handle_modal(state, key.code);
            } else {
                match state.phase {
                    Phase::Ready => handle_ready(state, key.code),
                    Phase::Reviewing => handle_reviewing(state, key.code),
                    Phase::Done => handle_done(state, key.code),
                    _ => {}
                }
            }
        }
    }
}

fn handle_ready(state: &mut AppState, code: KeyCode) {
    match code {
        KeyCode::Char('q') | KeyCode::Esc => std::process::exit(0),
        KeyCode::Up | KeyCode::Char('k') => state.move_up(),
        KeyCode::Down | KeyCode::Char('j') => state.move_down(),
        KeyCode::Char(' ') => state.toggle_current(),
        KeyCode::Char('a') => state.select_all(),
        KeyCode::Char('i') => state.invert_selection(),
        KeyCode::Char('t') => {
            state.target_mode = state.target_mode.toggle();
        }
        KeyCode::Char('w') => {
            state.write_mode = state.write_mode.toggle();
        }
        KeyCode::Char('o') => open_link(state),
        KeyCode::Char('m') => {
            if state.has_loaded_notes() {
                state.open_modal();
            }
        }
        KeyCode::Char('x') | KeyCode::Enter => {
            if state.selected_count() > 0 {
                state.phase = Phase::Reviewing;
            }
        }
        _ => {}
    }
}

fn handle_modal(state: &mut AppState, code: KeyCode) {
    const MAX_SCROLL: u16 = 500;
    match code {
        KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('m') => state.close_modal(),
        KeyCode::Up | KeyCode::Char('k') => state.modal_scroll_up(1),
        KeyCode::Down | KeyCode::Char('j') => state.modal_scroll_down(1, MAX_SCROLL),
        KeyCode::PageUp => state.modal_scroll_up(10),
        KeyCode::PageDown => state.modal_scroll_down(10, MAX_SCROLL),
        _ => {}
    }
}

fn handle_reviewing(state: &mut AppState, code: KeyCode) {
    match code {
        KeyCode::Char('q') | KeyCode::Esc => std::process::exit(0),
        KeyCode::Char('b') => state.phase = Phase::Ready,
        KeyCode::Enter => state.phase = Phase::Applying,
        _ => {}
    }
}

fn handle_done(_state: &mut AppState, _code: KeyCode) {
    std::process::exit(0);
}

fn apply_updates(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut AppState,
) -> Result<()> {
    let items: Vec<crate::domain::dependency::DependencyItem> =
        state.items.iter().filter(|i| i.checked).cloned().collect();
    let total = items.len();
    state.apply_log.clear();

    for (i, item) in items.iter().enumerate() {
        let cmd = item.install_command(&state.package_manager, &state.write_mode);

        // Push the "running…" line and redraw immediately so the user sees it.
        state.apply_log.push(format!(
            "[{}/{}]  {}  {}",
            i + 1,
            total,
            item.name,
            cmd
        ));
        terminal.draw(|f| draw(f, state))?;

        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }
        match std::process::Command::new(parts[0])
            .args(&parts[1..])
            .current_dir(&state.project_path)
            .output()
        {
            Ok(out) if out.status.success() => {
                state.apply_log.push("       ✓  done".into());
            }
            Ok(out) => {
                let e = String::from_utf8_lossy(&out.stderr);
                state.apply_log.push(format!(
                    "       ✗  failed: {}",
                    e.lines().next().unwrap_or("unknown error")
                ));
            }
            Err(e) => {
                state.apply_log.push(format!("       ✗  failed: {e}"));
            }
        }
        // Redraw again with the result line visible.
        terminal.draw(|f| draw(f, state))?;
    }
    Ok(())
}

// ── Pre-TUI startup screen ────────────────────────────────────────────────────

/// Hex colour → crossterm Color::Rgb
const fn rgb(hex: u32) -> Color {
    Color::Rgb {
        r: ((hex >> 16) & 0xFF) as u8,
        g: ((hex >> 8) & 0xFF) as u8,
        b: (hex & 0xFF) as u8,
    }
}

const PRIMARY: Color = rgb(0x2151BF);
const SHIMMER: [Color; 3] = [rgb(0x1A3D8F), rgb(0x2151BF), rgb(0x4B7BF5)];

/// Print the static startup layout to stderr using raw ANSI / crossterm.
///
/// ```
/// Depick
///
///   selective dependency updates
///   ─────────────────────────────
///   ◆ my-project   npm
///   ─────────────────────────────
///   Scanning dependencies…
/// ```
fn print_startup_screen(project: &str, pm: &str) -> Result<()> {
    let mut err = io::stderr();

    // "Depick" — bold + PRIMARY blue
    execute!(
        err,
        SetAttribute(Attribute::Bold),
        SetForegroundColor(PRIMARY),
        Print("Depick"),
        ResetColor,
        Print("\n\n"),
        Print("  selective dependency updates\n"),
        Print("  ─────────────────────────────\n"),
        SetForegroundColor(PRIMARY),
        SetAttribute(Attribute::Bold),
        Print(format!("  ◆ {}   {}", project, pm)),
        ResetColor,
        Print("\n"),
        Print("  ─────────────────────────────\n"),
    )?;

    // The shimmer line will be overwritten in place; write the initial frame.
    execute!(err, Print("  Scanning dependencies…\n"))?;
    err.flush()?;
    Ok(())
}

/// Overwrite the "Scanning dependencies…" line with a shimmering version.
///
/// The shimmer cycles three shades of blue across the text characters at
/// ~80 ms per frame. `elapsed_ms` drives the wave position.
/// `done` / `total` suffix shows fetch progress when > 0.
fn animate_shimmer(elapsed_ms: u64, done: usize, total: usize) -> Result<()> {
    let label: Vec<char> = "  Scanning dependencies…".chars().collect();
    let mut err = io::stderr();

    // Move cursor up 1 line and to column 0, then clear the line.
    eprint!("\x1b[1A\r\x1b[2K");

    // Wave position advances one character every 80 ms.
    let wave_pos = (elapsed_ms / 80) as usize;

    for (i, ch) in label.iter().enumerate() {
        // distance from the wave crest, mod the palette length
        let idx = (wave_pos + SHIMMER.len() * 2 - i) % SHIMMER.len();
        let colour = SHIMMER[idx];
        execute!(err, SetForegroundColor(colour), Print(ch))?;
    }

    execute!(err, ResetColor)?;

    if total > 0 && done < total {
        execute!(
            err,
            SetForegroundColor(SHIMMER[0]),
            Print(format!("  {done}/{total}")),
            ResetColor,
        )?;
    } else if done == total && total > 0 {
        execute!(
            err,
            SetForegroundColor(SHIMMER[2]),
            Print(format!("  {total}/{total}  ✓")),
            ResetColor,
        )?;
    }

    eprintln!();
    err.flush()?;
    Ok(())
}

fn open_link(state: &AppState) {
    use domain::release::{ReleaseCacheKey, ReleaseContextState};

    let Some(item) = state.selected_item() else {
        return;
    };
    let key = ReleaseCacheKey::from_item(item);
    let url = match state.release_contexts.entries.get(&key) {
        Some(ReleaseContextState::Loaded(ctx)) => {
            ctx.release_url.clone().or_else(|| ctx.compare_url.clone())
        }
        _ => Some(format!("https://www.npmjs.com/package/{}", item.name)),
    };
    if let Some(url) = url {
        #[cfg(target_os = "macos")]
        let _ = std::process::Command::new("open").arg(&url).spawn();
        #[cfg(target_os = "linux")]
        let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
        #[cfg(target_os = "windows")]
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", &url])
            .spawn();
    }
}

fn draw(f: &mut ratatui::Frame, state: &AppState) {
    match state.phase {
        Phase::Ready | Phase::Scanning => draw_main(f, f.area(), state),
        Phase::Reviewing => draw_review(f, f.area(), state),
        Phase::Applying => ui::apply::render(f, f.area(), state),
        Phase::Done => draw_done(f, f.area(), state),
    }
}

fn draw_main(f: &mut ratatui::Frame, area: ratatui::layout::Rect, state: &AppState) {
    // Vertical skeleton:
    //   [0] header        — 4 lines
    //   [1] body          — fills remaining space
    //   [2] detail bar    — 5 lines (name/change, command, links)
    //   [3] footer prompt — 1 line
    //   [4] footer keys   — 1 line
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // header
            Constraint::Min(6),    // body: table (left) + release notes (right)
            Constraint::Length(5), // package detail bar
            Constraint::Length(1), // footer prompt
            Constraint::Length(1), // footer keys
        ])
        .split(area);

    // Body splits horizontally: scrollable package list | release notes panel
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(rows[1]);

    ui::header::render(f, rows[0], state);
    ui::table::render(f, body[0], state);
    ui::details::render_notes(f, body[1], state);
    ui::details::render_detail_bar(f, rows[2], state);
    ui::footer::render(f, rows[3], state);
    ui::footer::render_keys(f, rows[4], state);

    // Modal rendered last so it paints on top of everything
    if state.show_notes_modal {
        ui::details::render_modal(f, state);
    }
}

fn draw_review(f: &mut ratatui::Frame, area: ratatui::layout::Rect, state: &AppState) {
    let c = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(10), Constraint::Length(1)])
        .split(area);
    ui::review::render(f, c[0], state);
    ui::review::render_footer(f, c[1]);
}

fn draw_done(f: &mut ratatui::Frame, area: ratatui::layout::Rect, state: &AppState) {
    let c = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(6), Constraint::Length(1)])
        .split(area);
    ui::apply::render_done(f, c[0], state);
    ui::footer::render(f, c[1], state);
}
