//! TUI-aware renderer that forwards LLM events to the TUI event loop.
//!
//! [`TuiRenderer`] implements the [`Renderer`] trait by sending
//! [`RenderEvent`] variants over a tokio mpsc channel. The TUI main
//! loop receives these events and updates the [`App`](super::App) state
//! accordingly.

use serde_json::Value;
use tokio::sync::mpsc;

use crate::output::Renderer;

/// Events sent from the renderer to the TUI event loop.
#[allow(dead_code)]
#[derive(Debug)]
pub enum RenderEvent {
    /// A single token arrived from the LLM stream.
    Token(String),
    /// The LLM response is complete.
    Done,
    /// An error occurred during streaming.
    Error(String),
    /// The agent started executing a tool.
    ToolStart {
        /// Tool name.
        name: String,
        /// Serialized tool arguments.
        args: String,
    },
    /// A tool execution completed.
    ToolResult {
        /// Tool name.
        name: String,
        /// Tool output text.
        result: String,
    },
    /// A warning to display.
    Warn(String),
}

/// Renderer that sends events to the TUI via an mpsc channel.
///
/// All trait methods are fire-and-forget: if the channel is full or
/// closed the event is silently dropped.
#[allow(dead_code)]
pub struct TuiRenderer {
    /// Channel sender for dispatching render events.
    tx: mpsc::Sender<RenderEvent>,
}

#[allow(dead_code)]
impl TuiRenderer {
    /// Creates a new [`TuiRenderer`] backed by the given channel sender.
    pub fn new(tx: mpsc::Sender<RenderEvent>) -> Self {
        Self { tx }
    }
}

impl Renderer for TuiRenderer {
    fn render_token(&mut self, token: &str) {
        let _ = self.tx.try_send(RenderEvent::Token(token.to_string()));
    }

    fn render_done(&mut self) {
        let _ = self.tx.try_send(RenderEvent::Done);
    }

    fn render_error(&mut self, err: &str) {
        let _ = self.tx.try_send(RenderEvent::Error(err.to_string()));
    }

    fn tool_start(&mut self, name: &str, args: &Value) {
        let _ = self.tx.try_send(RenderEvent::ToolStart {
            name: name.to_string(),
            args: args.to_string(),
        });
    }

    fn tool_result(&mut self, name: &str, result: &str) {
        let _ = self.tx.try_send(RenderEvent::ToolResult {
            name: name.to_string(),
            result: result.to_string(),
        });
    }

    fn warn(&mut self, message: &str) {
        let _ = self.tx.try_send(RenderEvent::Warn(message.to_string()));
    }
}
