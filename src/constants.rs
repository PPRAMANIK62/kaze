//! Centralized constants for kaze.
//!
//! All magic numbers, default strings, and configuration constants live here
//! so they can be changed in one place.

/// Application name used in CLI output and directory paths.
pub const APP_NAME: &str = "kaze";

/// Default LLM model identifier.
pub const DEFAULT_MODEL: &str = "claude-sonnet-4-6";

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
pub const DEFAULT_OPENAI_MODEL: &str = "gpt-4.1";

/// Default LLM model identifier for OpenRouter.
pub const DEFAULT_OPENROUTER_MODEL: &str = "arcee-ai/trinity-large-preview:free";

/// Default base URL for local Ollama server.
pub const OLLAMA_DEFAULT_BASE_URL: &str = "http://localhost:11434";

/// Default LLM model identifier for Ollama.
pub const OLLAMA_DEFAULT_MODEL: &str = "llama3";

// --- Provider defaults ---

/// Default provider when none is configured.
pub const DEFAULT_PROVIDER: &str = "anthropic";

// --- Context window ---

/// Default context window size for models not in the registry.
pub const DEFAULT_CONTEXT_WINDOW: usize = 8_192;

/// Context usage ratio at which a warning is shown (80%).
pub const CONTEXT_WARN_THRESHOLD: f64 = 0.80;

/// Context usage ratio at which truncation/compaction triggers (95%).
pub const CONTEXT_DANGER_THRESHOLD: f64 = 0.95;

// --- Token counting ---

/// Approximate token overhead per message (role markers, etc.).
pub const TOKENS_PER_MESSAGE_OVERHEAD: usize = 4;

/// Approximate token overhead for conversation framing.
pub const TOKENS_CONVERSATION_FRAMING: usize = 2;

// --- Compaction defaults ---

/// Default: auto-compaction enabled.
pub const COMPACTION_AUTO_DEFAULT: bool = true;

/// Default usage ratio to trigger auto-compaction (90%).
pub const COMPACTION_THRESHOLD_DEFAULT: f64 = 0.90;

/// Default number of recent messages to keep during compaction.
pub const COMPACTION_KEEP_RECENT_DEFAULT: usize = 4;

/// Default reserved token budget for compaction summary.
pub const COMPACTION_RESERVED_DEFAULT: usize = 10_000;

/// System prompt for LLM-based context compaction.
pub const COMPACTION_PROMPT: &str =
    "Summarize the following conversation context concisely. \
Preserve key decisions, code snippets, file paths, and technical details mentioned. \
Do not add commentary. Return only the summary.\n\n";

// --- Tool limits ---

/// Maximum file size (bytes) the read_file tool will read.
pub const READ_FILE_MAX_SIZE: u64 = 100 * 1024;

/// Byte threshold for binary file detection (check first N bytes for null).
pub const BINARY_DETECTION_BYTES: usize = 8192;

/// Maximum number of results the glob tool returns.
pub const GLOB_MAX_RESULTS: usize = 1000;

/// Maximum number of matching lines the grep tool returns.
pub const GREP_MAX_MATCHES: usize = 50;
