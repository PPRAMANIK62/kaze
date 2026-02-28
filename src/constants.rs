//! Centralized constants for kaze.
//!
//! All magic numbers, default strings, and configuration constants live here
//! so they can be changed in one place.

/// Application name used in CLI output and directory paths.
pub const APP_NAME: &str = "kaze";

/// Default LLM model identifier.
pub const DEFAULT_MODEL: &str = "claude-sonnet-4-5";

/// Maximum tokens for LLM completions.
pub const MAX_TOKENS: u64 = 4096;

/// Default system prompt prepended to all conversations.
pub const DEFAULT_SYSTEM_PROMPT: &str =
    "You are kaze, a helpful AI coding assistant in the terminal. \
Be concise. Use code blocks with language tags when showing code.";

/// Configuration filename.
pub const CONFIG_FILENAME: &str = "config.toml";

/// Per-project configuration filename.
pub const PROJECT_CONFIG_FILENAME: &str = "kaze.toml";

/// Readline history filename.
pub const HISTORY_FILENAME: &str = "chat_history.txt";

/// Default LLM model identifier for OpenAI.
pub const DEFAULT_OPENAI_MODEL: &str = "gpt-4o";

/// Default LLM model identifier for OpenRouter.
pub const DEFAULT_OPENROUTER_MODEL: &str = "arcee-ai/trinity-large-preview:free";
