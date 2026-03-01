//! TUI application state for kaze.
//!
//! Holds the message history, current input buffer, and scroll position
//! that drive the terminal UI layout.

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
}

impl App {
    /// Creates a new empty application state.
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            scroll_offset: 0,
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
    }

    /// Scrolls the message history up by one line.
    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(1);
    }

    /// Scrolls the message history down by one line.
    pub fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }
}
