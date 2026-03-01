//! Model listing and discovery.
//!
//! Displays available models grouped by provider, including dynamically
//! queried Ollama models. Isolates display/UI concerns from the provider core.

use anyhow::Result;

use super::resolve::resolve_model;
use crate::config::Config;

/// List all available models, grouped by provider.
pub async fn list_models(config: &Config) -> Result<()> {
    let selection = resolve_model(None, None, config)?;
    let current = &selection.model;

    println!("Available models:\n");

    // Anthropic
    println!("  anthropic:");
    for info in crate::models::ANTHROPIC_MODELS {
        let marker = if info.name == current { " (default)" } else { "" };
        println!("    {}{marker}", info.name);
    }

    // OpenAI
    println!("\n  openai:");
    for info in crate::models::OPENAI_MODELS {
        let marker = if info.name == current { " (default)" } else { "" };
        println!("    {}{marker}", info.name);
    }

    // Ollama (dynamic)
    println!("\n  ollama:");
    match list_ollama_models(config).await {
        Ok(models) if models.is_empty() => {
            println!("    (no models found -- run `ollama pull llama3`)");
        }
        Ok(models) => {
            for model in &models {
                let marker = if model == current { " (default)" } else { "" };
                println!("    {model}{marker}");
            }
        }
        Err(_) => {
            println!("    (ollama not running)");
        }
    }

    Ok(())
}

/// Query Ollama's local API for available models.
async fn list_ollama_models(config: &Config) -> Result<Vec<String>> {
    let base_url = config
        .provider
        .ollama
        .as_ref()
        .and_then(|o| o.base_url.as_deref())
        .unwrap_or(crate::constants::OLLAMA_DEFAULT_BASE_URL);

    let url = format!("{base_url}/api/tags");

    let resp: serde_json::Value = reqwest::get(&url).await?.json().await?;

    let models = resp["models"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m["name"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    Ok(models)
}
