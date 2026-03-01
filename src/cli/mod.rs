//! Command-line interface definition and dispatch for kaze.
//!
//! Uses [`clap`] for argument parsing with derive macros. Each subcommand is
//! routed to its handler — session operations live in the [`session`] submodule.

mod session;

use crate::{agent, chat, config, message::Message, output, provider, tools::ToolRegistry};
use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::sync::Arc;

/// Top-level CLI structure for kaze.
///
/// Parsed from command-line arguments via [`clap::Parser`]. Contains a single
/// required subcommand that determines which action kaze performs.
#[derive(Parser)]
#[command(name = "kaze", about = "A memory-minimal AI coding agent")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Available subcommands for the kaze CLI.
///
/// Each variant maps to a top-level action. The `///` doc comments on variants
/// double as `--help` text rendered by clap.
#[derive(Subcommand)]
pub enum Commands {
    /// Ask a one-shot question
    Ask {
        /// The question to ask
        prompt: Vec<String>,
        /// Model to use (overrides config)
        #[arg(short, long)]
        model: Option<String>,
        /// Provider to use (anthropic, openai, openrouter, ollama)
        #[arg(short, long)]
        provider: Option<String>,
    },
    /// Start an interactive chat session
    Chat {
        /// Resume a specific session
        #[arg(short, long)]
        session: Option<String>,
        /// Provider to use (anthropic, openai, openrouter, ollama)
        #[arg(long)]
        provider: Option<String>,
        /// Model to use (overrides config)
        #[arg(short, long)]
        model: Option<String>,
        /// Open the terminal UI
        #[arg(long)]
        tui: bool,
    },
    /// List available models
    Models,
    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Manage chat sessions
    Session {
        #[command(subcommand)]
        action: SessionAction,
    },
}

/// Subcommands for the `config` command.
///
/// Controls reading and writing kaze's TOML configuration file
/// stored at the XDG config path (`~/.config/kaze/config.toml`).
#[derive(Subcommand)]
pub enum ConfigAction {
    /// Show current config
    Show,
    /// Set a config value
    Set { key: String, value: String },
}

/// Subcommands for the `session` command.
#[derive(Subcommand)]
pub enum SessionAction {
    /// Start a new chat session
    New,
    /// List all sessions
    List,
    /// Resume a session by ID (supports partial IDs)
    Resume { id: String },
    /// Delete a session by ID (supports partial IDs)
    Delete { id: String },
}

/// Parses command-line arguments into a [`Cli`] struct.
///
/// Delegates to [`clap::Parser::parse`], which exits the process on invalid input.
pub fn parse() -> Cli {
    Cli::parse()
}

/// Dispatches the parsed CLI command to its handler.
///
/// Routes each [`Commands`] variant to the appropriate implementation.
/// All handlers are currently stubs that print `TODO` messages.
pub async fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Ask {
            prompt,
            model,
            provider: provider_name,
        } => {
            let prompt = prompt.join(" ");
            if prompt.is_empty() {
                anyhow::bail!("No prompt provided. Usage: kaze ask \"your question here\"");
            }

            let config = config::Config::load()?;

            let selection =
                provider::resolve_model(provider_name.as_deref(), model.as_deref(), &config)?;

            println!(
                "{} [model: {}]",
                "kaze".bold().cyan(),
                selection.model.yellow(),
            );
            println!();
            println!("{} {}", ">".green().bold(), prompt);
            println!();

            let provider = provider::Provider::from_config(&config, &selection)?;
            let project_root = std::env::current_dir()?;
            let tools = ToolRegistry::with_builtins(project_root.clone());

            let mut messages = Vec::new();
            if let Some(ref sp) = config.system_prompt {
                messages.push(Message::system(sp.clone()));
            }
            messages.push(Message::user(&prompt));

            let permission_manager = Arc::new(crate::permissions::PermissionManager::new(
                config.permissions.clone(),
            ));
            let hook = crate::hooks::KazeHook::new(permission_manager, project_root);

            let mut renderer = output::StdoutRenderer::new();
            let response = agent::agent_loop(
                &provider,
                &mut messages,
                &tools,
                &mut renderer,
                crate::constants::MAX_AGENT_ITERATIONS,
                hook,
            )
            .await?;
            // Show token usage
            let token_count = crate::tokens::count_tokens(&response, &selection.model)?;
            let limit = 128_000;
            println!();
            println!(
                "{}",
                format!(
                    "Tokens: {}",
                    crate::tokens::format_token_usage(token_count, limit)
                )
                .dimmed()
            );

            Ok(())
        }
        Commands::Chat {
            session,
            provider: provider_name,
            model,
            tui,
        } => {
            if tui {
                crate::tui::run_tui().await
            } else {
                let mut config = config::Config::load()?;
                let selection =
                    provider::resolve_model(provider_name.as_deref(), model.as_deref(), &config)?;
                config.model = selection.model.clone();
                chat::run_chat(config, session, &selection).await
            }
        }
        Commands::Models => {
            let config = config::Config::load()?;
            crate::provider::list_models(&config).await
        }
        Commands::Config { action } => {
            let config = config::Config::load()?;
            match action {
                ConfigAction::Show => {
                    let path = config::Config::config_path()?;
                    println!("{} {}", "Config path:".bold(), path.display());
                    println!();
                    let toml_str = toml::to_string_pretty(&config)?;
                    println!("{}", toml_str);
                }
                ConfigAction::Set { key, value } => {
                    println!("TODO: set {} = {}", key, value);
                }
            }
            Ok(())
        }
        Commands::Session { action } => session::handle_session(action).await,
    }
}
