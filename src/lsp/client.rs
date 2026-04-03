//! LSP client implementation

use crate::lsp::protocol::*;
use crate::lsp::LspCapabilities;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::process::{Child, Command};

use async_trait::async_trait;
use crate::agent::lsp::{LspBackend, Location};
use crate::error::CliError;

/// LSP client for communicating with language servers
pub struct LspClient {
    children: HashMap<String, Child>,
    #[allow(dead_code)]
    workspace_root: PathBuf,
    #[allow(dead_code)]
    capabilities: LspCapabilities,
}

impl LspClient {
    /// Create a new LSP client for the given workspace
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            children: HashMap::new(),
            workspace_root,
            capabilities: LspCapabilities::default(),
        }
    }

    /// Start an LSP server for a language
    pub async fn start_server(
        &mut self,
        language: &str,
        config: &crate::lsp::LspServerConfig,
    ) -> Result<(), CliError> {
        if self.children.contains_key(language) {
            return Ok(()); // Already running
        }

        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .stdin(std::process::Stdio::piped())
            .kill_on_drop(true);

        for (k, v) in &config.env {
            cmd.env(k, v);
        }

        let child = cmd
            .spawn()
            .map_err(|e| CliError::Other(format!("failed to start LSP server {}: {e}", config.command)))?;

        self.children.insert(language.to_string(), child);
        Ok(())
    }

    /// Stop an LSP server
    pub async fn stop_server(&mut self, language: &str) -> Result<(), CliError> {
        if let Some(mut child) = self.children.remove(language) {
            child.kill().await.map_err(|e| CliError::Other(format!("failed to kill LSP server: {e}")))?;
        }
        Ok(())
    }

    /// Stop all LSP servers
    pub async fn stop_all(&mut self) {
        for (_, mut child) in self.children.drain() {
            let _ = child.kill().await;
        }
    }

    /// Send an LSP request and wait for response
    pub async fn request<R: serde::Serialize + serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<R, CliError> {
        // TODO: Implement actual LSP communication
        let id = uuid::Uuid::new_v4().to_string();
        let request = LspRequest {
            jsonrpc: "2.0".to_string(),
            id: LspRequestId::String(id.clone()),
            method: method.to_string(),
            params,
        };

        let _json = serde_json::to_string(&request)
            .map_err(|e| CliError::Other(format!("LSP serialization error: {e}")))?;

        // Placeholder: In real implementation, send to child stdin and read from stdout
        eprintln!("LSP request: {method}");
        Err(CliError::Other("LSP communication not yet implemented".to_string()))
    }

    /// Get completion items at a position
    pub async fn completions(
        &self,
        _language: &str,
        file: &Path,
        position: LspPosition,
    ) -> Result<Vec<LspCompletionItem>, CliError> {
        let params = serde_json::json!({
            "textDocument": { "uri": format!("file://{}", file.display()) },
            "position": position,
        });
        self.request("textDocument/completion", params).await
    }

    /// Get hover information at a position
    pub async fn hover(
        &self,
        _language: &str,
        file: &Path,
        position: LspPosition,
    ) -> Result<Option<LspHover>, CliError> {
        let params = serde_json::json!({
            "textDocument": { "uri": format!("file://{}", file.display()) },
            "position": position,
        });
        self.request("textDocument/hover", params).await
    }

    /// Go to definition
    pub async fn goto_definition(
        &self,
        _language: &str,
        file: &Path,
        position: LspPosition,
    ) -> Result<Vec<LspLocation>, CliError> {
        let params = serde_json::json!({
            "textDocument": { "uri": format!("file://{}", file.display()) },
            "position": position,
        });
        self.request("textDocument/definition", params).await
    }

    /// Find all references
    pub async fn find_references(
        &self,
        _language: &str,
        file: &Path,
        position: LspPosition,
    ) -> Result<Vec<LspLocation>, CliError> {
        let params = serde_json::json!({
            "textDocument": { "uri": format!("file://{}", file.display()) },
            "position": position,
        });
        self.request("textDocument/references", params).await
    }
}

use std::sync::Arc;

/// Wrapper that makes LspClient shareable across threads via Mutex.
/// This is needed because LspClient holds Child processes which are not Sync.
pub struct LspBackendImpl(pub Arc<tokio::sync::Mutex<LspClient>>);

#[async_trait]
impl LspBackend for LspBackendImpl {
    async fn hover(&self, file: &str, _line: u32, _col: u32) -> Result<Option<String>, CliError> {
        // TODO: implement stdio JSON-RPC communication with LSP server
        let _ = file;
        Ok(None)
    }

    async fn goto_definition(&self, file: &str, _line: u32, _col: u32) -> Result<Option<Location>, CliError> {
        // TODO: implement stdio JSON-RPC communication with LSP server
        let _ = file;
        Ok(None)
    }

    async fn find_references(&self, file: &str, _line: u32, _col: u32) -> Result<Vec<Location>, CliError> {
        // TODO: implement stdio JSON-RPC communication with LSP server
        let _ = file;
        Ok(Vec::new())
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        // Kill all child processes
        for (_, child) in self.children.iter_mut() {
            let _ = child.start_kill();
        }
    }
}
