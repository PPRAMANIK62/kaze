//! Command-line interface definition and dispatch for kaze.
//!
//! Uses [`clap`] for argument parsing with derive macros. Each subcommand is
//! currently a stub that will be replaced as features are implemented.

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use crate::{chat, config, provider, output};

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
        /// Provider to use (openai, anthropic, ollama)
        #[arg(short, long)]
        provider: Option<String>,
    },
    /// Start an interactive chat session
    Chat {
        /// Resume a specific session
        #[arg(short, long)]
        session: Option<String>,
    },
    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
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
        Commands::Ask { prompt, model, provider: _provider_name } => {
            let prompt = prompt.join(" ");
            if prompt.is_empty() {
                anyhow::bail!("No prompt provided. Usage: kaze ask \"your question here\"");
            }

            let mut config = config::Config::load()?;

            // Apply CLI overrides
            if let Some(m) = model {
                config.model = m;
            }

            println!(
                "{} [model: {}]",
                "kaze".bold().cyan(),
                config.model.yellow(),
            );
            println!();
            println!("{} {}", ">".green().bold(), prompt);
            println!();

            let provider = provider::Provider::from_config(&config)?;
            let mut renderer = output::StdoutRenderer::new();
            let _response = provider
                .stream(&prompt, config.system_prompt.as_deref(), &mut renderer)
                .await?;

            Ok(())
        }
        Commands::Chat { session } => {
            let config = config::Config::load()?;
            chat::run_chat(config, session).await
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
    }
}
