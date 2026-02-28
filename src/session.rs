//! Session persistence for kaze.
//!
//! Each session is stored as a JSONL file under `~/.local/share/kaze/sessions/`.
//! A `sessions/index.json` file maintains metadata for all sessions.
//! JSONL is crash-safe (append-only) and human-readable.

use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::Config;
use crate::message::{Message, Role};

/// Metadata for a single session, stored in the session index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub id: String,
    pub title: Option<String>,
    pub model: String,
    pub created_at: String,
    pub updated_at: String,
    pub message_count: usize,
}

/// Index of all sessions, persisted as `index.json`.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct SessionIndex {
    pub sessions: Vec<SessionMeta>,
}

/// An active conversation session.
///
/// Manages a JSONL file of messages and updates the session index
/// on each append for crash-safe persistence.
pub struct Session {
    pub id: String,
    pub messages: Vec<Message>,
    pub model: String,
    pub file_path: PathBuf,
}

impl Session {
    /// Creates a new session with a UUID v4 identifier.
    ///
    /// Ensures the sessions directory exists and sets up the file path.
    pub fn new(model: &str) -> Result<Self> {
        let id = Uuid::new_v4().to_string();
        let dir = Self::sessions_dir()?;
        fs::create_dir_all(&dir).context("Failed to create sessions directory")?;
        let file_path = Self::session_path(&id)?;

        Ok(Self {
            id,
            messages: Vec::new(),
            model: model.to_string(),
            file_path,
        })
    }

    /// Loads an existing session from its JSONL file.
    ///
    /// Reads the model from the session index and all messages from the JSONL file.
    pub fn load(id: &str) -> Result<Self> {
        let file_path = Self::session_path(id)?;
        let short = &id[..8.min(id.len())];
        anyhow::ensure!(file_path.exists(), "Session {} not found", short);

        // Read model from index
        let index = Self::load_index()?;
        let model = index
            .sessions
            .iter()
            .find(|s| s.id == id)
            .map(|s| s.model.clone())
            .unwrap_or_default();

        // Read messages from JSONL
        let file = fs::File::open(&file_path)
            .with_context(|| format!("Failed to open session file {:?}", file_path))?;
        let reader = BufReader::new(file);
        let mut messages = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let msg: Message = serde_json::from_str(&line)
                .with_context(|| "Failed to parse message from session file")?;
            messages.push(msg);
        }

        Ok(Self {
            id: id.to_string(),
            messages,
            model,
            file_path,
        })
    }

    /// Appends a message to the session.
    ///
    /// Writes the message as a JSON line to the JSONL file, flushes immediately
    /// for crash safety, and updates the session index.
    pub fn append(&mut self, msg: Message) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)
            .with_context(|| format!("Failed to open session file {:?}", self.file_path))?;

        let json = serde_json::to_string(&msg)?;
        writeln!(file, "{}", json)?;
        file.flush()?;

        self.messages.push(msg);
        self.update_index()?;

        Ok(())
    }

    /// Returns the session title derived from the first user message.
    ///
    /// Truncates to 50 characters. Returns `None` if no user message exists.
    pub fn title(&self) -> Option<String> {
        self.messages
            .iter()
            .find(|m| m.role == Role::User)
            .map(|m| {
                let text = m.text();
                if text.chars().count() > 50 {
                    let truncated: String = text.chars().take(50).collect();
                    format!("{}...", truncated)
                } else {
                    text.to_string()
                }
            })
    }

    /// Updates (or creates) this session's entry in the index file.
    fn update_index(&self) -> Result<()> {
        let mut index = Self::load_index()?;
        let now = Utc::now().to_rfc3339();

        if let Some(entry) = index.sessions.iter_mut().find(|s| s.id == self.id) {
            entry.title = self.title();
            entry.updated_at = now;
            entry.message_count = self.messages.len();
        } else {
            index.sessions.push(SessionMeta {
                id: self.id.clone(),
                title: self.title(),
                model: self.model.clone(),
                created_at: now.clone(),
                updated_at: now,
                message_count: self.messages.len(),
            });
        }

        let path = Self::index_path()?;
        let json = serde_json::to_string_pretty(&index)?;
        fs::write(&path, json).with_context(|| "Failed to write session index")?;

        Ok(())
    }

    /// Loads the session index, returning a default empty index if the file doesn't exist.
    fn load_index() -> Result<SessionIndex> {
        let path = Self::index_path()?;
        if !path.exists() {
            return Ok(SessionIndex::default());
        }
        let contents = fs::read_to_string(&path).with_context(|| "Failed to read session index")?;
        let index: SessionIndex =
            serde_json::from_str(&contents).with_context(|| "Failed to parse session index")?;
        Ok(index)
    }

    /// Returns the sessions directory path (`~/.local/share/kaze/sessions/`).
    fn sessions_dir() -> Result<PathBuf> {
        Ok(Config::data_dir()?.join("sessions"))
    }

    /// Returns the JSONL file path for a given session ID.
    fn session_path(id: &str) -> Result<PathBuf> {
        Ok(Self::sessions_dir()?.join(format!("{}.jsonl", id)))
    }

    /// Returns the path to the session index file.
    fn index_path() -> Result<PathBuf> {
        Ok(Self::sessions_dir()?.join("index.json"))
    }

    /// Returns metadata for all sessions.
    pub fn list_all() -> Result<Vec<SessionMeta>> {
        let index = Self::load_index()?;
        Ok(index.sessions)
    }

    /// Deletes a session's JSONL file and removes it from the index.
    pub fn delete(id: &str) -> Result<()> {
        let path = Self::session_path(id)?;
        if path.exists() {
            fs::remove_file(&path)
                .with_context(|| format!("Failed to delete session file {:?}", path))?;
        }

        let mut index = Self::load_index()?;
        index.sessions.retain(|s| s.id != id);

        let index_path = Self::index_path()?;
        let dir = Self::sessions_dir()?;
        if dir.exists() {
            let json = serde_json::to_string_pretty(&index)?;
            fs::write(&index_path, json).with_context(|| "Failed to update session index")?;
        }

        Ok(())
    }
}
