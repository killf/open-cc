//! List sessions command

use crate::error::CliError;
use crate::session::SessionStorage;

pub async fn run() -> Result<(), CliError> {
    let storage = SessionStorage::new();
    let sessions = storage.list().await
        .map_err(|e| CliError::Session(format!("failed to list sessions: {e}")))?;

    if sessions.is_empty() {
        println!("No sessions found.");
    } else {
        for session in sessions {
            let ts = chrono::DateTime::from_timestamp_millis(session.updated_at)
                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_default();
            println!("{}  {}  {} messages", session.id, ts, session.message_count);
        }
    }

    Ok(())
}
