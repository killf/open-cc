//! LSP backend trait for code intelligence
//!
//! Implemented by the binary to provide hover, goto-definition, and references
//! via the configured language servers.

use crate::error::CliError;
use async_trait::async_trait;

/// Represents a source code location
#[derive(Debug, Clone)]
pub struct Location {
    /// File path (absolute)
    pub file: String,
    /// 0-indexed line number
    pub line: u32,
    /// 0-indexed column number
    pub column: u32,
}

/// LSP-backed code intelligence
#[async_trait]
pub trait LspBackend: Send + Sync {
    /// Get hover information for a position (0-indexed line and column)
    async fn hover(&self, file: &str, line: u32, col: u32) -> Result<Option<String>, CliError>;

    /// Go to definition of a symbol at the given position
    async fn goto_definition(&self, file: &str, line: u32, col: u32) -> Result<Option<Location>, CliError>;

    /// Find all references to a symbol at the given position
    async fn find_references(&self, file: &str, line: u32, col: u32) -> Result<Vec<Location>, CliError>;
}
