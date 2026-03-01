//! TUI application state for kaze.
//!
//! Holds the message history, current input buffer, and scroll position
//! that drive the terminal UI layout.

use super::renderer::RenderEvent;

/// A single chat message displayed in the TUI message history.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    /// The role of the message sender (e.g. "user", "assistant", "system").
    pub role: String,
    /// The text content of the message.
    pub content: String,
}

/// Core application state for the TUI.
///
/// Tracks all messages, the current input line, and vertical scroll
/// offset for the message history pane.
pub struct App {
    /// Ordered list of chat messages displayed in the history pane.
    pub messages: Vec<ChatMessage>,
    /// Current text in the input box.
    pub input: String,
    /// Vertical scroll offset for the message history (in lines).
    pub scroll_offset: u16,
    /// Whether tokens are currently arriving from the LLM.
    pub streaming: bool,
    /// Whether we are waiting for the first token (shows spinner).
    pub waiting: bool,
    /// Current animation frame for the spinner.
    pub spinner_frame: usize,
}

impl App {
    /// Creates a new empty application state.
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            scroll_offset: 0,
            streaming: false,
            waiting: false,
            spinner_frame: 0,
        }
    }

    /// Submits the current input as a user message.
    ///
    /// If the input is empty, this is a no-op. Otherwise, the input text
    /// is moved into a new [`ChatMessage`] with role "user", appended to
    /// the message history, and the scroll offset is reset to zero.
    pub fn submit_input(&mut self) {
        if self.input.is_empty() {
            return;
        }
        let text = std::mem::take(&mut self.input);
        self.messages.push(ChatMessage {
            role: "user".to_string(),
            content: text,
        });
        self.scroll_offset = 0;
        self.waiting = true;
    }

    /// Scrolls the message history up by one line.
    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(1);
    }

    /// Scrolls the message history down by one line.
    pub fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    /// Handles a render event from the LLM streaming channel.
    pub fn handle_render_event(&mut self, event: RenderEvent) {
        match event {
            RenderEvent::Token(token) => {
                self.waiting = false;
                self.streaming = true;
                if let Some(last) = self.messages.last_mut() {
                    if last.role == "assistant" && self.streaming {
                        last.content.push_str(&token);
                        self.scroll_offset = 0;
                        return;
                    }
                }
                self.messages.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: token,
                });
                self.scroll_offset = 0;
            }
            RenderEvent::ToolStart { name, args: _ } => {
                self.messages.push(ChatMessage {
                    role: "tool".to_string(),
                    content: format!("⚡ Calling {}...", name),
                });
            }
            RenderEvent::ToolResult { name: _, result } => {
                if let Some(last) = self.messages.last_mut() {
                    if last.role == "tool" {
                        if result.len() > 200 {
                            let end = result.floor_char_boundary(197);
                            last.content = format!("{}...", &result[..end]);
                        } else {
                            last.content = result;
                        }
                    }
                }
            }
            RenderEvent::Done => {
                self.streaming = false;
                self.waiting = false;
            }
            RenderEvent::Error(err) => {
                self.streaming = false;
                self.waiting = false;
                self.messages.push(ChatMessage {
                    role: "error".to_string(),
                    content: err,
                });
            }
            RenderEvent::Warn(msg) => {
                self.messages.push(ChatMessage {
                    role: "warning".to_string(),
                    content: msg,
                });
            }
        }
    }

    /// Advances the spinner animation frame when waiting.
    pub fn tick_spinner(&mut self) {
        if self.waiting {
            self.spinner_frame = (self.spinner_frame + 1) % 4;
        }
    }
}
