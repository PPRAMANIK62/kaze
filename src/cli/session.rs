//! Session management CLI operations for kaze.
//!
//! Handles listing, resuming, and deleting chat sessions through the
//! `kaze session` subcommand family. Provides table-formatted output
//! and partial session ID matching (git-style short IDs).

use anyhow::Result;
use colored::Colorize;

use crate::{chat, config, provider, session};
use super::SessionAction;

/// Dispatches a session subcommand to its handler.
pub(crate) async fn handle_session(action: SessionAction) -> Result<()> {
    match action {
        SessionAction::New => {
            let config = config::Config::load()?;
            let selection = provider::resolve_model(None, None, &config)?;
            let mut config = config;
            config.model = selection.model.clone();
            chat::run_chat(config, None, &selection).await
        }
        SessionAction::List => session_list(),
        SessionAction::Resume { id } => {
            let config = config::Config::load()?;
            let selection = provider::resolve_model(None, None, &config)?;
            let mut config = config;
            config.model = selection.model.clone();
            let full_id = resolve_session_id(&id)?;
            chat::run_chat(config, Some(full_id), &selection).await
        }
        SessionAction::Delete { id } => {
            let full_id = resolve_session_id(&id)?;
            session_delete(&full_id)
        }
    }
}

/// Resolves a partial session ID to a full ID.
///
/// Matches the given prefix against all known session IDs. Returns an error
/// if zero or multiple sessions match.
pub(crate) fn resolve_session_id(partial: &str) -> Result<String> {
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

/// Lists all saved sessions in a formatted table.
///
/// Displays session ID, title, message count, last-updated timestamp,
/// and model. Adapts column widths to the terminal size.
pub(crate) fn session_list() -> Result<()> {
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

/// Deletes a session by its full ID.
pub(crate) fn session_delete(id: &str) -> Result<()> {
    let sessions = session::Session::list_all()?;
    let meta = sessions.iter().find(|s| s.id == id)
        .ok_or_else(|| anyhow::anyhow!("Session not found: {}", id))?;
    let title = meta.title.as_deref().unwrap_or("(untitled)");
    println!("Deleting session {} (\"{}\")", &id[..8].cyan(), title);
    session::Session::delete(id)?;
    println!("{}", "Deleted.".green());
    Ok(())
}
