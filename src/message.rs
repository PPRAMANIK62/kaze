//! Message types for kaze's conversation history.
//!
//! Provides a structured [`Message`] type with [`Role`] and [`Content`] enums
//! that represent conversation turns. These are kaze's internal types,
//! converted to provider-specific formats (e.g. rig-core's `Message`) when
//! sent to the LLM.

use serde::{Deserialize, Serialize};

/// A single message in a conversation.
///
/// Contains a [`Role`] indicating who produced the message and [`Content`]
/// representing the message body. The `Content` enum is `untagged` for serde
/// to support future multimodal payloads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: Content,
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
        }
    }
    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: Content::Text(text.into()),
        }
    }
    pub fn system(text: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: Content::Text(text.into()),
        }
    }
    pub fn text(&self) -> &str {
        match &self.content {
            Content::Text(s) => s,
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
