//! REPL tool — interact with a persistent Read-Eval-Print-Loop session.
//
// Input: command (String), context (Option<String>)
// name: "REPL"
// is_read_only: false

use async_trait::async_trait;
use serde::Deserialize;

use crate::error::CliError;
use crate::types::{Tool, ToolContext, ToolResult};

/// Input schema for the REPL tool.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ReplInput {
    /// The REPL command to execute.
    command: String,
    /// Optional language/session context hint (e.g. "python", "node", "rust").
    #[serde(default)]
    context: Option<String>,
}

pub struct ReplTool;

impl ReplTool {
    pub fn new() -> Self {
        Self
    }

    fn input_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The REPL command to execute"
                },
                "context": {
                    "type": "string",
                    "description": "Optional language or session context hint (e.g. 'python', 'node', 'rust')"
                }
            },
            "required": ["command"]
        })
    }
}

impl Default for ReplTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ReplTool {
    fn name(&self) -> &str {
        "REPL"
    }

    fn description(&self) -> String {
        "Executes a command in a persistent REPL (Read-Eval-Print-Loop) session. \
         Maintains state between calls within the same session. Use the context \
         field to select or create a language-specific REPL session."
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
        let _input: ReplInput = serde_json::from_value(args)
            .map_err(|e| CliError::ToolExecution(format!("Invalid input: {e}")))?;

        Err(CliError::ToolExecution(
            "This tool requires the TUI/system integration.".to_string(),
        ))
    }

    fn render_use_message(&self, args: &serde_json::Value) -> String {
        if let Ok(input) = serde_json::from_value::<ReplInput>(args.clone()) {
            let ctx = input.context.as_deref().unwrap_or("default");
            format!("REPL [{}]: {}", ctx, &input.command[..input.command.len().min(60)])
        } else {
            "Running REPL command".to_string()
        }
    }

    fn render_result_message(&self, result: &ToolResult) -> String {
        if result.is_error {
            "REPL command failed.".to_string()
        } else {
            "REPL command completed.".to_string()
        }
    }
}
