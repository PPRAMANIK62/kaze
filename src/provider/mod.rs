//! LLM provider abstraction for kaze.
//!
//! Wraps rig-core's provider clients behind a [`Provider`] struct with enum
//! dispatch, keeping provider-specific details out of the CLI layer. Supports
//! Anthropic, OpenAI, OpenRouter, and Ollama (local) via [`ProviderKind`].

mod client;
mod kind;
mod listing;
mod resolve;

pub use client::Provider;
#[allow(unused_imports)]
pub use kind::{default_model_for, ProviderKind};
pub use listing::list_models;
pub use resolve::{resolve_model, ModelSelection};
