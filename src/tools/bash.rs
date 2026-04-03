//! Bash tool - execute shell commands

use async_trait::async_trait;
use serde::Deserialize;
use std::process::Stdio;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Instant;

use crate::error::CliError;
use crate::types::{ResultContentBlock, Tool, ToolContext, ToolMetrics, ToolResult};

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct BashInput {
    command: String,
    #[serde(default)]
    timeout: Option<u64>,
    #[serde(default)]
    working_directory: Option<String>,
    #[serde(default)]
    environment: Option<std::collections::HashMap<String, String>>,
}

pub struct BashTool {
    timeout_secs: u64,
}

impl BashTool {
    pub fn new() -> Self {
        Self { timeout_secs: 300 }
    }

    fn input_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                },
                "timeout": {
                    "type": "number",
                    "description": "Timeout in seconds (default: 300)"
                },
                "working_directory": {
                    "type": "string",
                    "description": "Working directory for the command"
                },
                "environment": {
                    "type": "object",
                    "description": "Additional environment variables"
                }
            },
            "required": ["command"]
        })
    }
}

impl Default for BashTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "Bash"
    }

    fn aliases(&self) -> Vec<String> {
        vec!["bash".to_string(), "shell".to_string(), "exec".to_string()]
    }

    fn description(&self) -> String {
        "Execute a shell command in the terminal. Use this for running programs, \
         git commands, npm scripts, and other shell operations."
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        Self::input_schema()
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn is_destructive(&self) -> bool {
        false
    }

    async fn call(
        &self,
        args: serde_json::Value,
        context: ToolContext,
    ) -> Result<ToolResult, CliError> {
        let input: BashInput = serde_json::from_value(args)?;
        let timeout = input.timeout.unwrap_or(self.timeout_secs);
        let cwd = input
            .working_directory
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| context.working_directory.clone());

        let start = Instant::now();

        let mut cmd = tokio::process::Command::new("sh");
        cmd.args(["-c", &input.command])
            .current_dir(&cwd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .envs(&context.env);

        let spawned = cmd.spawn().map_err(|e| CliError::ToolExecution(e.to_string()))?;

        let child = Arc::new(Mutex::new(Some(spawned)));
        let child_for_timeout = Arc::clone(&child);

        let timeout_duration = std::time::Duration::from_secs(timeout);

        let output_result = tokio::time::timeout(timeout_duration, async {
            let mut c = child_for_timeout.lock().await;
            if let Some(child) = c.take() {
                child.wait_with_output().await.map(Some)
            } else {
                Ok(None)
            }
        }).await;

        let output = match output_result {
            Ok(Ok(Some(output))) => output,
            Ok(Ok(None)) => return Ok(ToolResult::error("child already taken".to_string())),
            Ok(Err(e)) => return Ok(ToolResult::error(format!("failed to wait: {e}"))),
            Err(_) => {
                let mut c = child.lock().await;
                if let Some(mut child) = c.take() {
                    let _ = child.start_kill();
                }
                return Ok(ToolResult::error(format!(
                    "Command timed out after {timeout} seconds"
                )));
            }
        };

        let duration_ms = start.elapsed().as_millis() as u64;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let mut content = Vec::new();

        if !output.status.success() {
            if !stderr.is_empty() {
                content.push(ResultContentBlock::Text {
                    text: format!("stderr:\n{stderr}"),
                });
            }
            if !stdout.is_empty() {
                content.push(ResultContentBlock::Text { text: stdout.to_string() });
            }
        }
        if stdout.is_empty() && !stderr.is_empty() {
            content.push(ResultContentBlock::Text { text: stderr.to_string() });
        }

        if content.is_empty() {
            content.push(ResultContentBlock::Text {
                text: "(no output)".to_string(),
            });
        }

        Ok(ToolResult {
            content,
            is_error: !output.status.success(),
            metrics: Some(ToolMetrics {
                duration_ms,
                tokens_used: None,
            }),
        })
    }

    fn render_use_message(&self, args: &serde_json::Value) -> String {
        if let Ok(input) = serde_json::from_value::<BashInput>(args.clone()) {
            let cmd = if input.command.len() > 60 {
                format!("{}...", &input.command[..60])
            } else {
                input.command.clone()
            };
            format!("Executing: {cmd}")
        } else {
            "Executing bash command".to_string()
        }
    }
}
