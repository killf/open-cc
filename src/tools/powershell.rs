//! PowerShell tool — execute PowerShell commands on Windows.
//
// Input: command (String), description (Option<String>)
// name: "PowerShell"
// is_read_only: false
//
// This is a stub implementation. The full tool executes PowerShell commands
// with sandbox support, background task management, permission checks, and
// output formatting (image detection, stderr handling, etc.).

use async_trait::async_trait;
use serde::Deserialize;

use crate::error::CliError;
use crate::types::{Tool, ToolContext, ToolResult};

/// Maximum timeout in milliseconds for a PowerShell command.
const MAX_TIMEOUT_MS: u64 = 600_000; // 10 minutes

/// Input schema for the PowerShell tool.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PowerShellInput {
    /// The PowerShell command to execute.
    command: String,
    /// Optional timeout in milliseconds (max 600_000 / 10 minutes).
    #[serde(default)]
    timeout: Option<u64>,
    /// Clear, concise description of what this command does in active voice.
    #[serde(default)]
    description: Option<String>,
    /// Set to true to run this command in the background.
    #[serde(default)]
    run_in_background: Option<bool>,
    /// Set to true to dangerously override sandbox mode.
    #[serde(default)]
    dangerously_disable_sandbox: Option<bool>,
}

pub struct PowerShellTool;

impl PowerShellTool {
    pub fn new() -> Self {
        Self
    }

    fn input_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The PowerShell command to execute"
                },
                "timeout": {
                    "type": "integer",
                    "description": format!("Optional timeout in milliseconds (max {})", MAX_TIMEOUT_MS)
                },
                "description": {
                    "type": "string",
                    "description": "Clear, concise description of what this command does in active voice."
                },
                "run_in_background": {
                    "type": "boolean",
                    "description": "Set to true to run this command in the background. Use Read to read the output later."
                },
                "dangerously_disable_sandbox": {
                    "type": "boolean",
                    "description": "Set to true to dangerously override sandbox mode and run commands without sandboxing."
                }
            },
            "required": ["command"]
        })
    }
}

impl Default for PowerShellTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for PowerShellTool {
    fn name(&self) -> &str {
        "PowerShell"
    }

    fn description(&self) -> String {
        "Executes PowerShell commands on Windows. Supports sandboxed execution, \
         background tasks, permission checks, image output detection, and large \
         output persistence. Use the description field to explain what the command \
         does in active voice."
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        Self::input_schema()
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn is_concurrency_safe(&self) -> bool {
        false
    }

    async fn call(
        &self,
        args: serde_json::Value,
        _context: ToolContext,
    ) -> Result<ToolResult, CliError> {
        let input: PowerShellInput = serde_json::from_value(args)
            .map_err(|e| CliError::ToolExecution(format!("Invalid input: {e}")))?;

        if let Some(timeout) = input.timeout {
            if timeout > MAX_TIMEOUT_MS {
                return Err(CliError::ToolExecution(format!(
                    "Timeout {}ms exceeds maximum allowed {}ms",
                    timeout, MAX_TIMEOUT_MS
                )));
            }
        }

        Err(CliError::ToolExecution(
            "This tool requires the TUI/system integration.".to_string(),
        ))
    }

    fn render_use_message(&self, args: &serde_json::Value) -> String {
        if let Ok(input) = serde_json::from_value::<PowerShellInput>(args.clone()) {
            let preview = if input.command.len() > 60 {
                format!("{}...", &input.command[..60])
            } else {
                input.command.clone()
            };
            format!(
                "PowerShell{}: {}",
                input
                    .description
                    .as_ref()
                    .map(|d| format!(" ({d})"))
                    .unwrap_or_default(),
                preview
            )
        } else {
            "Running PowerShell command".to_string()
        }
    }

    fn render_result_message(&self, result: &ToolResult) -> String {
        if result.is_error {
            "PowerShell command failed.".to_string()
        } else {
            "PowerShell command completed.".to_string()
        }
    }
}
