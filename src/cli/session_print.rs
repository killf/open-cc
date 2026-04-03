//! Print session command

use crate::error::CliError;
use crate::session::SessionStorage;

pub async fn run(session_id: &str) -> Result<(), CliError> {
    let storage = SessionStorage::new();
    let session = storage.load(session_id).await
        .map_err(|e| CliError::Session(format!("failed to load session: {e}")))?;

    println!("Session: {}", session.id);
    println!("Model: {}", session.model);
    println!("Messages:");
    for msg in &session.messages {
        println!("{:#?}", msg);
    }

    Ok(())
}
