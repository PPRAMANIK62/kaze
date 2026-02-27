//! Command-line interface definition and dispatch for kaze.
//!
//! Uses [`clap`] for argument parsing with derive macros. Each subcommand is
//! currently a stub that will be replaced as features are implemented.

use anyhow::Result;
use clap::{Parser, Subcommand};

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
        Commands::Ask { prompt } => {
            let prompt = prompt.join(" ");
            println!("TODO: ask '{}'", prompt);
            Ok(())
        }
        Commands::Chat { session } => {
            println!("TODO: chat (session: {:?})", session);
            Ok(())
        }
        Commands::Config { action } => {
            match action {
                ConfigAction::Show => println!("TODO: show config"),
                ConfigAction::Set { key, value } => println!("TODO: set {} = {}", key, value),
            }
            Ok(())
        }
    }
}
