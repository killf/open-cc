//! Plugin tool wrapper — implements `dyn Tool` for plugin-defined tools

use async_trait::async_trait;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;

use crate::types::{
    ResultContentBlock, Tool as ToolTrait, ToolContext, ToolResult,
};
use crate::CliError;

/// A tool provided by a plugin, executed as a subprocess
pub struct PluginTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub plugin_path: std::path::PathBuf,
    /// Tool-specific env vars from plugin config
    pub env: std::collections::HashMap<String, String>,
    /// Working directory for the tool
    pub working_directory: Option<std::path::PathBuf>,
    /// Timeout in seconds
    pub timeout_secs: u64,
}

impl PluginTool {
    pub fn new(
        name: String,
        description: String,
        input_schema: serde_json::Value,
        plugin_path: std::path::PathBuf,
    ) -> Self {
        Self {
            name,
            description,
            input_schema,
            plugin_path,
            env: std::collections::HashMap::new(),
            working_directory: None,
            timeout_secs: 60,
        }
    }
}

#[async_trait]
impl ToolTrait for PluginTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> String {
        self.description.clone()
    }

    fn input_schema(&self) -> serde_json::Value {
        self.input_schema.clone()
    }

    fn is_read_only(&self) -> bool {
        false
    }

    async fn call(
        &self,
        args: serde_json::Value,
        context: ToolContext,
    ) -> Result<ToolResult, CliError> {
        let tool_exe = self.plugin_path.join("tools").join(&self.name);

        let mut cmd = tokio::process::Command::new(&tool_exe);
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        // Pass context via environment
        cmd.env("CLAUDE_SESSION_ID", &context.session_id);
        cmd.env("CLAUDE_TOOL_NAME", &self.name);
        cmd.env("CLAUDE_WORKING_DIR", context.working_directory.to_string_lossy().as_ref());

        for (k, v) in &self.env {
            cmd.env(k, v);
        }
        for (k, v) in &context.env {
            cmd.env(k, v);
        }

        if let Some(ref wd) = self.working_directory {
            cmd.current_dir(wd);
        } else {
            cmd.current_dir(&context.working_directory);
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| CliError::Other(format!("failed to start plugin tool '{}': {}", self.name, e)))?;

        // Send args JSON to stdin
        if let Some(mut stdin) = child.stdin.take() {
            let json = serde_json::to_string(&args)
                .map_err(|e| CliError::Other(format!("failed to serialise tool args: {e}")))?;
            stdin
                .write_all(json.as_bytes())
                .await
                .map_err(|e| CliError::Other(format!("failed to write to plugin tool stdin: {e}")))?;
            drop(stdin);
        }

        let timeout = std::time::Duration::from_secs(self.timeout_secs);
        let output = tokio::time::timeout(timeout, child.wait_with_output())
            .await
            .map_err(|_| CliError::Other(format!("plugin tool '{}' timed out", self.name)))?
            .map_err(|e| CliError::Other(format!("plugin tool '{}' failed: {e}", self.name)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(ToolResult {
                content: vec![ResultContentBlock::Text {
                    text: format!("Plugin tool error:\n{}", stderr),
                }],
                is_error: true,
                metrics: None,
            });
        }

        // Parse stdout as JSON ToolResult
        let stdout = String::from_utf8_lossy(&output.stdout);
        let result: ToolResult = match serde_json::from_str(&stdout) {
            Ok(r) => r,
            Err(_) => {
                // Fall back to raw stdout as text
                ToolResult {
                    content: vec![ResultContentBlock::Text { text: stdout.to_string() }],
                    is_error: false,
                    metrics: None,
                }
            }
        };

        Ok(result)
    }
}
