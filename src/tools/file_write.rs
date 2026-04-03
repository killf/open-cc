//! FileWrite tool - write file contents

use async_trait::async_trait;
use serde::Deserialize;
use std::path::PathBuf;
use std::time::Instant;

use crate::error::CliError;
use crate::types::{ResultContentBlock, Tool, ToolContext, ToolMetrics, ToolResult};

#[derive(Debug, Deserialize)]
struct WriteInput {
    path: String,
    content: String,
}

pub struct FileWriteTool;

impl FileWriteTool {
    pub fn new() -> Self {
        Self
    }

    fn input_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to write"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                }
            },
            "required": ["path", "content"]
        })
    }
}

impl Default for FileWriteTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for FileWriteTool {
    fn name(&self) -> &str {
        "Write"
    }

    fn aliases(&self) -> Vec<String> {
        vec!["write".to_string(), "create".to_string(), "new".to_string()]
    }

    fn description(&self) -> String {
        "Write content to a file, creating it if it doesn't exist.".to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        Self::input_schema()
    }

    fn is_destructive(&self) -> bool {
        true
    }

    async fn call(
        &self,
        args: serde_json::Value,
        context: ToolContext,
    ) -> Result<ToolResult, CliError> {
        let input: WriteInput = serde_json::from_value(args)?;
        let path = PathBuf::from(&input.path);

        let abs_path = if path.is_absolute() {
            path.clone()
        } else {
            context.working_directory.join(&path)
        };

        let start = Instant::now();

        if let Some(parent) = abs_path.parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| CliError::ToolExecution(format!("cannot create directory: {e}")))?;
        }

        tokio::fs::write(&abs_path, &input.content).await
            .map_err(|e| CliError::ToolExecution(format!("cannot write file: {e}")))?;

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(ToolResult {
            content: vec![ResultContentBlock::Text {
                text: format!("File written: {} ({} bytes)", input.path, input.content.len()),
            }],
            is_error: false,
            metrics: Some(ToolMetrics {
                duration_ms,
                tokens_used: None,
            }),
        })
    }

    fn render_use_message(&self, args: &serde_json::Value) -> String {
        if let Ok(input) = serde_json::from_value::<WriteInput>(args.clone()) {
            format!("Writing {} bytes to: {}", input.content.len(), input.path)
        } else {
            "Writing file".to_string()
        }
    }
}
