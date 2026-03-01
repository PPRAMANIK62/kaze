//! Terminal UI module for kaze.
//!
//! Provides an alternative ratatui-based interface launched via `kaze chat --tui`.
//! The event loop runs asynchronously using [`tokio::select!`] with a 60 fps
//! render tick and crossterm's async [`EventStream`](crossterm::event::EventStream).

mod app;
mod renderer;
mod ui;

pub use app::App;
pub use renderer::RenderEvent;
#[allow(unused_imports)]
pub use renderer::TuiRenderer;
pub use ui::draw;

use std::io;

use anyhow::Result;
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use futures::StreamExt;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};

/// Render tick interval (~60 fps).
const TICK_DURATION: Duration = Duration::from_millis(16);

/// Launches the TUI event loop.
///
/// Enters raw mode and the alternate screen, then loops at ~60 fps:
/// - Redraws the UI each tick
/// - Handles crossterm key events (typing, scrolling, submit, quit)
///
/// On exit (Ctrl+C), restores the terminal to its normal state.
pub async fn run_tui() -> Result<()> {
    // --- Terminal setup ---
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let mut events = EventStream::new();
    let mut tick = interval(TICK_DURATION);

    // Channel for streaming LLM events into the TUI.
    let (tx, mut rx) = mpsc::channel::<RenderEvent>(1000);
    let _tx = tx; // keep sender alive so rx doesn't immediately close

    // --- Main event loop ---
    loop {
        tokio::select! {
            _ = tick.tick() => {
                app.tick_spinner();
                terminal.draw(|f| draw(f, &app))?;
            }
            event = events.next() => {
                match event {
                    Some(Ok(Event::Key(key))) => {
                        if !handle_key(&mut app, key) {
                            break;
                        }
                    }
                    Some(Err(_)) | None => break,
                    _ => {} // ignore mouse / resize for now
                }
            }
            Some(render_event) = rx.recv() => {
                app.handle_render_event(render_event);
            }
        }
    }

    // --- Terminal teardown ---
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

/// Processes a single key event, returning `false` when the loop should exit.
fn handle_key(app: &mut App, key: KeyEvent) -> bool {
    // Ctrl+C → quit
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return false;
    }

    match key.code {
        KeyCode::Enter => app.submit_input(),
        KeyCode::Char(c) => app.input.push(c),
        KeyCode::Backspace => {
            app.input.pop();
        }
        KeyCode::Up => app.scroll_up(),
        KeyCode::Down => app.scroll_down(),
        _ => {}
    }
    true
}
