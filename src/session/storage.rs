//! Session storage management

use std::path::PathBuf;
use anyhow::Result;

use crate::error::CliError;
use crate::types::{Session, SessionConfig, SessionSummary, TokenUsage};

/// Session storage managing read/write to disk
pub struct SessionStorage {
    sessions_dir: PathBuf,
}

impl SessionStorage {
    pub fn new() -> Self {
        let sessions_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("claude")
            .join("sessions");
        Self { sessions_dir }
    }

    pub fn sessions_dir(&self) -> &PathBuf {
        &self.sessions_dir
    }

    /// Get the directory for a specific session
    fn session_dir(&self, session_id: &str) -> PathBuf {
        self.sessions_dir.join(session_id)
    }

    /// Create a new session
    pub async fn create(&self, session_id: &str, config: &SessionConfig) -> Result<Session, CliError> {
        let dir = self.session_dir(session_id);
        tokio::fs::create_dir_all(&dir).await?;

        let session = Session::new(session_id.to_string(), config.model.clone());
        self.save(&session).await?;
        Ok(session)
    }

    /// Load an existing session
    pub async fn load(&self, session_id: &str) -> Result<Session, CliError> {
        let path = self.session_dir(session_id).join("session.json");
        if !path.exists() {
            return Err(CliError::Session(format!("session not found: {session_id}")));
        }
        let content = tokio::fs::read_to_string(&path).await?;
        let session: Session = serde_json::from_str(&content)?;
        Ok(session)
    }

    /// Save a session to disk
    pub async fn save(&self, session: &Session) -> Result<(), CliError> {
        let dir = self.session_dir(&session.id);
        tokio::fs::create_dir_all(&dir).await?;
        let path = dir.join("session.json");
        let content = serde_json::to_string_pretty(session)?;
        tokio::fs::write(&path, content).await?;
        Ok(())
    }

    /// List all sessions
    pub async fn list(&self) -> Result<Vec<SessionSummary>, CliError> {
        let mut summaries = Vec::new();
        let entries = tokio::fs::read_dir(&self.sessions_dir).await;

        if entries.is_err() {
            return Ok(summaries);
        }

        let mut entries = entries.unwrap();
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                if let Ok(summary) = self.load_summary(path.file_name().and_then(|n| n.to_str()).unwrap_or("")).await {
                    summaries.push(summary);
                }
            }
        }

        summaries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(summaries)
    }

    /// Load a session summary without full message content
    async fn load_summary(&self, session_id: &str) -> Result<SessionSummary, CliError> {
        let session = self.load(session_id).await?;
        Ok(SessionSummary {
            id: session.id,
            model: session.model,
            created_at: session.created_at,
            updated_at: session.updated_at,
            cost: session.cost,
            message_count: session.messages.len(),
            active_worktree: None,
        })
    }

    /// Delete a session
    pub async fn delete(&self, session_id: &str) -> Result<(), CliError> {
        let dir = self.session_dir(session_id);
        if dir.exists() {
            tokio::fs::remove_dir_all(&dir).await?;
        }
        Ok(())
    }

    /// Add cost to a session
    pub async fn add_cost(&self, session_id: &str, cost: f64) -> Result<(), CliError> {
        let mut session = self.load(session_id).await?;
        session.cost += cost;
        self.save(&session).await?;
        Ok(())
    }

    /// Add token usage to a session
    pub async fn add_token_usage(&self, session_id: &str, usage: &TokenUsage) -> Result<(), CliError> {
        let mut session = self.load(session_id).await?;
        session.token_usage.add(usage);
        self.save(&session).await?;
        Ok(())
    }
}

impl Default for SessionStorage {
    fn default() -> Self {
        Self::new()
    }
}
