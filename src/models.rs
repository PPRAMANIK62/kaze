//! Centralized model registry for kaze.
//!
//! Defines known models with their context window sizes. This is the single
//! source of truth — both `provider.rs` (for model listing) and `tokens.rs`
//! (for context window lookup) consume from here.

/// Information about a known LLM model.
pub struct ModelInfo {
    /// The model identifier string (e.g., "claude-sonnet-4-6").
    pub name: &'static str,
    /// Context window size in tokens.
    pub context_window: usize,
}

/// Known Anthropic models.
pub const ANTHROPIC_MODELS: &[ModelInfo] = &[
    ModelInfo {
        name: "claude-opus-4-6",
        context_window: 200_000,
    },
    ModelInfo {
        name: "claude-sonnet-4-6",
        context_window: 200_000,
    },
    ModelInfo {
        name: "claude-haiku-4-5",
        context_window: 200_000,
    },
    ModelInfo {
        name: "claude-sonnet-4-5",
        context_window: 200_000,
    },
    ModelInfo {
        name: "claude-opus-4",
        context_window: 200_000,
    },
];

/// Known OpenAI models.
pub const OPENAI_MODELS: &[ModelInfo] = &[
    ModelInfo {
        name: "gpt-5.2",
        context_window: 1_047_576,
    },
    ModelInfo {
        name: "gpt-5-mini",
        context_window: 1_047_576,
    },
    ModelInfo {
        name: "gpt-5-nano",
        context_window: 1_047_576,
    },
    ModelInfo {
        name: "gpt-4.1",
        context_window: 1_047_576,
    },
    ModelInfo {
        name: "gpt-4.1-mini",
        context_window: 1_047_576,
    },
    ModelInfo {
        name: "gpt-4.1-nano",
        context_window: 1_047_576,
    },
    ModelInfo {
        name: "o3",
        context_window: 200_000,
    },
    ModelInfo {
        name: "o4-mini",
        context_window: 200_000,
    },
];

/// Common Ollama models with known context window sizes.
/// Ollama models are also queried dynamically; these provide context window
/// defaults for models we recognize.
pub const OLLAMA_MODELS: &[ModelInfo] = &[
    ModelInfo {
        name: "llama3",
        context_window: 8_192,
    },
    ModelInfo {
        name: "llama3:70b",
        context_window: 8_192,
    },
    ModelInfo {
        name: "codellama",
        context_window: 16_384,
    },
    ModelInfo {
        name: "mistral",
        context_window: 32_768,
    },
    ModelInfo {
        name: "mixtral",
        context_window: 32_768,
    },
];
