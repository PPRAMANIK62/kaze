//! Output rendering abstraction for kaze.
//!
//! Defines the [`Renderer`] trait that decouples LLM output from the display
//! layer. [`StdoutRenderer`] prints tokens directly to the terminal; a future
//! `TuiRenderer` (Phase 7) will render to ratatui widgets instead.

use colored::Colorize;
use serde_json::Value;
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

    /// Called when the agent starts executing a tool.
    fn tool_start(&mut self, name: &str, args: &Value);

    /// Called when a tool execution completes with its result.
    fn tool_result(&mut self, name: &str, result: &str);

    // Part of public API, used in future phases
    #[allow(dead_code)]
    /// Display a warning message to the user.
    fn warn(&mut self, message: &str);
}

/// Renders streaming LLM output directly to stdout.
///
/// Each token is printed immediately with an explicit flush so the user
/// sees a "typing" effect. Tracks the total number of tokens received
/// and buffers the raw text for accurate visual line counting.
pub struct StdoutRenderer {
    token_count: usize,
    buffer: String,
}

impl StdoutRenderer {
    pub fn new() -> Self {
        Self {
            token_count: 0,
            buffer: String::new(),
        }
    }

    /// Calculates the number of cursor-up movements needed to erase
    /// all streamed output (raw text + render_done output).
    ///
    /// Accounts for terminal line wrapping by using the actual terminal width.
    pub fn visual_line_count(&self) -> usize {
        let width = terminal_size::terminal_size()
            .map(|(w, _)| w.0 as usize)
            .unwrap_or(80)
            .max(1); // prevent division by zero

        // Count visual lines the raw text occupies (including wrapping)
        let content_lines: usize = self
            .buffer
            .split('\n')
            .map(|line| {
                let len = line.len();
                if len == 0 {
                    1
                } else {
                    len.div_ceil(width)
                }
            })
            .sum();

        // content_lines is the number of visual lines.
        // Cursor-up count = (content_lines - 1) + 3
        //   -1 because the first line doesn't need a cursor-up to reach
        //   +3 for render_done's 3 println! calls
        content_lines.saturating_sub(1) + 3
    }
}

impl Renderer for StdoutRenderer {
    fn render_token(&mut self, token: &str) {
        self.buffer.push_str(token);
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

    fn tool_start(&mut self, name: &str, args: &Value) {
        let args_str = args.to_string();
        let truncated = if args_str.len() > 80 {
            let end = args_str.floor_char_boundary(77);
            format!("{}...", &args_str[..end])
        } else {
            args_str
        };
        eprintln!("⚡ {} {}", name.yellow(), truncated.dimmed());
    }

    fn tool_result(&mut self, name: &str, result: &str) {
        let truncated = if result.len() > 200 {
            let end = result.floor_char_boundary(197);
            format!("{}...", &result[..end])
        } else {
            result.to_string()
        };
        eprintln!("{} {}", format!("✓ {}", name).green(), truncated.dimmed());
    }

    fn warn(&mut self, message: &str) {
        eprintln!("{} {}", "warning:".yellow().bold(), message);
    }
}
