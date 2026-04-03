//! Transcript management (NDJSON format)

use std::path::PathBuf;
use anyhow::Result;

use crate::error::CliError;
use crate::types::Message;

/// Transcript file manager
pub struct TranscriptManager {
    session_dir: PathBuf,
}

impl TranscriptManager {
    pub fn new(session_id: &str) -> Self {
        let session_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("claude")
            .join("sessions")
            .join(session_id);
        Self { session_dir }
    }

    pub fn transcript_path(&self) -> PathBuf {
        self.session_dir.join("transcript.ndjson")
    }

    /// Append a message to the transcript (NDJSON format)
    pub async fn append(&self, message: &Message) -> Result<(), CliError> {
        tokio::fs::create_dir_all(&self.session_dir).await?;
        let path = self.transcript_path();

        let json = serde_json::to_string(message)?;
        let line = format!("{json}\n");

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await?;
        tokio::io::AsyncWriteExt::write_all(&mut file, line.as_bytes()).await?;
        Ok(())
    }

    /// Read all messages from the transcript
    pub async fn read_all(&self) -> Result<Vec<Message>, CliError> {
        let path = self.transcript_path();
        if !path.exists() {
            return Ok(Vec::new());
        }

        let content = tokio::fs::read_to_string(&path).await?;
        let mut messages = Vec::new();

        for line in content.lines() {
            if !line.trim().is_empty() {
                let msg: Message = serde_json::from_str(line)?;
                messages.push(msg);
            }
        }

        Ok(messages)
    }

    /// Clear the transcript
    pub async fn clear(&self) -> Result<(), CliError> {
        let path = self.transcript_path();
        if path.exists() {
            tokio::fs::remove_file(&path).await?;
        }
        Ok(())
    }
}
