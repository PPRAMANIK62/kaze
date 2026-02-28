//! Output rendering abstraction for kaze.
//!
//! Defines the [`Renderer`] trait that decouples LLM output from the display
//! layer. [`StdoutRenderer`] prints tokens directly to the terminal; a future
//! `TuiRenderer` (Phase 7) will render to ratatui widgets instead.

use colored::Colorize;
use std::io::{self, Write};

/// Trait for rendering LLM output.
/// StdoutRenderer prints to terminal now.
/// TuiRenderer (Phase 7) will render to ratatui widgets.
pub trait Renderer {
    /// Render a single token as it arrives.
    fn render_token(&mut self, token: &str);

    /// Called when the full response is complete.
    fn render_done(&mut self);

    /// Called when an error occurs during streaming.
    fn render_error(&mut self, err: &str);
}

/// Renders streaming LLM output directly to stdout.
///
/// Each token is printed immediately with an explicit flush so the user
/// sees a "typing" effect. Tracks the total number of tokens received
/// and displays a summary when the stream completes.
pub struct StdoutRenderer {
    token_count: usize,
}

impl StdoutRenderer {
    pub fn new() -> Self {
        Self { token_count: 0 }
    }
}

impl Renderer for StdoutRenderer {
    fn render_token(&mut self, token: &str) {
        print!("{}", token);
        // Flush immediately so each token appears as it arrives
        io::stdout().flush().ok();
        self.token_count += 1;
    }

    fn render_done(&mut self) {
        println!(); // Final newline after stream ends
        println!();
        println!("{}", format!("[{} tokens]", self.token_count).dimmed());
    }

    fn render_error(&mut self, err: &str) {
        eprintln!();
        eprintln!("{} {}", "error:".red().bold(), err);
    }
}
