//! Language Server Protocol (LSP) integration

#![allow(dead_code)]

pub mod client;
pub mod protocol;

pub use client::{LspBackendImpl, LspClient};

/// LSP capabilities for code intelligence
#[derive(Debug, Clone)]
pub struct LspCapabilities {
    /// Supported languages
    pub languages: Vec<String>,
    /// Whether hover is supported
    pub hover: bool,
    /// Whether goto-definition is supported
    pub goto_definition: bool,
    /// Whether find-references is supported
    pub find_references: bool,
    /// Whether completion is supported
    pub completion: bool,
    /// Whether workspace symbols are supported
    pub workspace_symbols: bool,
}

impl Default for LspCapabilities {
    fn default() -> Self {
        Self {
            languages: vec![
                "rust".to_string(),
                "typescript".to_string(),
                "javascript".to_string(),
                "python".to_string(),
                "go".to_string(),
            ],
            hover: true,
            goto_definition: true,
            find_references: true,
            completion: true,
            workspace_symbols: false,
        }
    }
}

/// Configuration for LSP servers
#[derive(Debug, Clone)]
pub struct LspConfig {
    /// Language ID -> server command
    pub servers: std::collections::HashMap<String, LspServerConfig>,
    /// Auto-start servers
    pub auto_start: bool,
    /// Trace LSP communication
    pub trace: bool,
}

impl Default for LspConfig {
    fn default() -> Self {
        Self {
            servers: Self::default_servers(),
            auto_start: true,
            trace: false,
        }
    }
}

impl LspConfig {
    fn default_servers() -> std::collections::HashMap<String, LspServerConfig> {
        let mut servers = std::collections::HashMap::new();
        servers.insert(
            "rust".to_string(),
            LspServerConfig {
                command: "rust-analyzer".to_string(),
                args: vec![],
                env: std::collections::HashMap::new(),
            },
        );
        servers.insert(
            "typescript".to_string(),
            LspServerConfig {
                command: "typescript-language-server".to_string(),
                args: vec!["--stdio".to_string()],
                env: std::collections::HashMap::new(),
            },
        );
        servers
    }
}

#[derive(Debug, Clone)]
pub struct LspServerConfig {
    pub command: String,
    pub args: Vec<String>,
    pub env: std::collections::HashMap<String, String>,
}
