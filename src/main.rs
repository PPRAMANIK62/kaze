//! Entry point for kaze, a memory-minimal AI coding agent for the terminal.
//!
//! This binary loads environment variables, parses CLI arguments via [`cli`],
//! and dispatches to the appropriate subcommand handler.

mod agent;
mod chat;
mod cli;
mod compaction;
mod config;
mod constants;
mod format;
mod hooks;
mod message;
mod models;
mod output;
mod permissions;
mod provider;
mod session;
mod tokens;
mod tools;

use anyhow::Result;

/// Runs the kaze CLI.
///
/// Loads `.env` files (silently ignored if absent), parses command-line
/// arguments into a [`cli::Cli`] struct, and dispatches the chosen
/// subcommand via [`cli::run`].
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let cli = cli::parse();
    cli::run(cli).await
}
