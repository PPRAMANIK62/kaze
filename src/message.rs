//! Message types for kaze's conversation history.
//!
//! Provides a structured [`Message`] type with [`Role`] and [`Content`] enums
//! that represent conversation turns. These are kaze's internal types,
//! converted to provider-specific formats (e.g. rig-core's `Message`) when
//! sent to the LLM.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A tool invocation requested by the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique identifier for this tool call (used to match results).
    pub id: String,
    /// Name of the tool to invoke.
    pub name: String,
    /// JSON arguments to pass to the tool.
    pub arguments: Value,
}

/// A single message in a conversation.
///
/// Contains a [`Role`] indicating who produced the message and [`Content`]
/// representing the message body. The `Content` enum is `untagged` for serde
/// to support future multimodal payloads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: Content,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// The role of a message sender in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// Message content, currently text-only but structured for future multimodal support.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Content {
    Text(String),
}

impl Message {
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: Content::Text(text.into()),
            tool_calls: Vec::new(),
            tool_call_id: None,
        }
    }
    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: Content::Text(text.into()),
            tool_calls: Vec::new(),
            tool_call_id: None,
        }
    }
    pub fn system(text: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: Content::Text(text.into()),
            tool_calls: Vec::new(),
            tool_call_id: None,
        }
    }
    pub fn text(&self) -> &str {
        match &self.content {
            Content::Text(s) => s,
        }
    }

    // Part of public API, used in future phases
    #[allow(dead_code)]
    /// Creates a tool result message to feed back to the LLM.
    pub fn tool_result(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: Content::Text(content.into()),
            tool_calls: Vec::new(),
            tool_call_id: Some(tool_call_id.into()),
        }
    }

    // Part of public API, used in future phases
    #[allow(dead_code)]
    /// Returns the text content as an owned String.
    pub fn text_content(&self) -> String {
        match &self.content {
            Content::Text(s) => s.clone(),
        }
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::System => write!(f, "system"),
            Role::User => write!(f, "you"),
            Role::Assistant => write!(f, "kaze"),
            Role::Tool => write!(f, "tool"),
        }
    }
}
