//! Command-line interface definition and dispatch for kaze.
//!
//! Uses [`clap`] for argument parsing with derive macros. Each subcommand is
//! currently a stub that will be replaced as features are implemented.

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use crate::{chat, config, provider, output, session};

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
    },
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
        Commands::Ask { prompt, model, provider: provider_name } => {
            let prompt = prompt.join(" ");
            if prompt.is_empty() {
                anyhow::bail!("No prompt provided. Usage: kaze ask \"your question here\"");
            }

            let mut config = config::Config::load()?;

            let provider_kind = provider_name
                .map(|p| provider::ProviderKind::from_str(&p))
                .transpose()?;

            // Apply CLI overrides
            if let Some(m) = model {
                config.model = m;
            } else if matches!(provider_kind, Some(provider::ProviderKind::OpenAI)) {
                config.model = crate::constants::DEFAULT_OPENAI_MODEL.to_string();
            } else if matches!(provider_kind, Some(provider::ProviderKind::OpenRouter)) {
                config.model = crate::constants::DEFAULT_OPENROUTER_MODEL.to_string();
            } else if matches!(provider_kind, Some(provider::ProviderKind::Ollama)) {
                config.model = crate::constants::OLLAMA_DEFAULT_MODEL.to_string();
            }
            println!(
                "{} [model: {}]",
                "kaze".bold().cyan(),
                config.model.yellow(),
            );
            println!();
            println!("{} {}", ">".green().bold(), prompt);
            println!();

            let provider = provider::Provider::from_config(&config, provider_kind)?;
            let mut renderer = output::StdoutRenderer::new();
            let _response = provider
                .stream(&prompt, config.system_prompt.as_deref(), &mut renderer)
                .await?;

            Ok(())
        }
        Commands::Chat { session, provider: provider_name, model } => {
            let mut config = config::Config::load()?;
            let provider_kind = provider_name
                .map(|p| provider::ProviderKind::from_str(&p))
                .transpose()?;
            if let Some(m) = model {
                config.model = m;
            } else if matches!(provider_kind, Some(provider::ProviderKind::OpenAI)) {
                config.model = crate::constants::DEFAULT_OPENAI_MODEL.to_string();
            } else if matches!(provider_kind, Some(provider::ProviderKind::OpenRouter)) {
                config.model = crate::constants::DEFAULT_OPENROUTER_MODEL.to_string();
            } else if matches!(provider_kind, Some(provider::ProviderKind::Ollama)) {
                config.model = crate::constants::OLLAMA_DEFAULT_MODEL.to_string();
            }
            chat::run_chat(config, session, provider_kind).await
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
        Commands::Session { action } => {
            handle_session(action).await
        }
    }
}

async fn handle_session(action: SessionAction) -> Result<()> {
    match action {
        SessionAction::New => {
            let config = config::Config::load()?;
            chat::run_chat(config, None, None).await
        }
        SessionAction::List => session_list(),
        SessionAction::Resume { id } => {
            let config = config::Config::load()?;
            let full_id = resolve_session_id(&id)?;
            chat::run_chat(config, Some(full_id), None).await
        }
        SessionAction::Delete { id } => {
            let full_id = resolve_session_id(&id)?;
            session_delete(&full_id)
        }
    }
}

fn resolve_session_id(partial: &str) -> Result<String> {
    let sessions = session::Session::list_all()?;
    let matches: Vec<_> = sessions.iter().filter(|s| s.id.starts_with(partial)).collect();
    match matches.len() {
        0 => anyhow::bail!("No session found matching '{}'", partial),
        1 => Ok(matches[0].id.clone()),
        _ => {
            eprintln!("{} Multiple sessions match '{}':", "ambiguous:".yellow(), partial);
            for s in &matches {
                let title = s.title.as_deref().unwrap_or("(untitled)");
                eprintln!("  {} {}", &s.id[..8], title.dimmed());
            }
            anyhow::bail!("Provide more characters to disambiguate")
        }
    }
}

fn session_list() -> Result<()> {
    let mut sessions = session::Session::list_all()?;
    if sessions.is_empty() {
        println!("{}", "No sessions found.".dimmed());
        println!("Start one with: {}", "kaze chat".cyan());
        return Ok(());
    }
    sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    // Dynamic column layout based on terminal width
    let term_width = terminal_size::terminal_size()
        .map(|(w, _)| w.0 as usize)
        .unwrap_or(80);

    // Fixed column widths: ID=10, MSGS=6, UPDATED=18, MODEL~20, gaps between columns
    let fixed_cols = 10 + 6 + 18 + 20;
    // Find the longest actual title
    let max_title_len = sessions.iter()
        .map(|s| s.title.as_deref().unwrap_or("(untitled)").chars().count())
        .max()
        .unwrap_or(5);

    // Title width = actual content width, capped by terminal space and max 50
    let max_from_terminal = term_width.saturating_sub(fixed_cols).min(50);
    let title_width = max_title_len.max(5).min(max_from_terminal);
    let header_width = 10 + title_width + 2 + 6 + 18 + 20; // +2 for TITLE padding

    // Print header
    println!(
        "{:<10} {:<tw$} {:<6} {:<18} {}",
        format!("{:<10}", "ID").bold(),
        format!("{:<tw$}", "TITLE", tw = title_width + 2).bold(),
        format!("{:<6}", "MSGS").bold(),
        format!("{:<18}", "UPDATED").bold(),
        "MODEL".bold(),
        tw = title_width + 2,
    );
    println!("{}", "-".repeat(term_width.min(header_width)));

    for s in &sessions {
        let short_id = &s.id[..8];
        let title_str = s.title.as_deref().unwrap_or("(untitled)");
        let title = if title_str.chars().count() > title_width {
            let truncated: String = title_str.chars().take(title_width - 3).collect();
            format!("{}...", truncated)
        } else {
            title_str.to_string()
        };

        // Format timestamp: parse RFC3339 -> "YYYY-MM-DD HH:MM"
        let updated = chrono::DateTime::parse_from_rfc3339(&s.updated_at)
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|_| {
                if s.updated_at.len() > 16 {
                    s.updated_at[..16].to_string()
                } else {
                    s.updated_at.clone()
                }
            });

        // Pad first, then colorize to avoid ANSI escape code width issues
        let id_col = format!("{:<10}", short_id);
        let title_col = format!("{:<tw$}", title, tw = title_width + 2);
        let msgs_col = format!("{:<6}", s.message_count);
        let updated_col = format!("{:<18}", updated);

        println!(
            "{} {} {} {} {}",
            id_col.cyan(),
            title_col,
            msgs_col.yellow(),
            updated_col.dimmed(),
            s.model.dimmed(),
        );
    }
    println!();
    println!("{} {} sessions. Resume with: {}",
        "total:".dimmed(), sessions.len(), "kaze session resume <id>".cyan());
    Ok(())
}

fn session_delete(id: &str) -> Result<()> {
    let sessions = session::Session::list_all()?;
    let meta = sessions.iter().find(|s| s.id == id)
        .ok_or_else(|| anyhow::anyhow!("Session not found: {}", id))?;
    let title = meta.title.as_deref().unwrap_or("(untitled)");
    println!("Deleting session {} (\"{}\")", &id[..8].cyan(), title);
    session::Session::delete(id)?;
    println!("{}", "Deleted.".green());
    Ok(())
}
