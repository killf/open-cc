//! FileRead tool - read file contents

use async_trait::async_trait;
use serde::Deserialize;
use std::path::PathBuf;
use std::time::Instant;

use crate::error::CliError;
use crate::types::{ResultContentBlock, Tool, ToolContext, ToolMetrics, ToolResult};

#[derive(Debug, Deserialize)]
struct ReadInput {
    path: String,
    #[serde(default)]
    start_line: Option<usize>,
    #[serde(default)]
    end_line: Option<usize>,
}

pub struct FileReadTool {
    max_file_size: usize,
}

impl FileReadTool {
    pub fn new() -> Self {
        Self { max_file_size: 10 * 1024 * 1024 }
    }

    fn input_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to read"
                },
                "start_line": {
                    "type": "number",
                    "description": "Starting line number (1-indexed)"
                },
                "end_line": {
                    "type": "number",
                    "description": "Ending line number (inclusive)"
                }
            },
            "required": ["path"]
        })
    }
}

impl Default for FileReadTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &str {
        "Read"
    }

    fn aliases(&self) -> Vec<String> {
        vec!["read".to_string(), "cat".to_string()]
    }

    fn description(&self) -> String {
        "Read the contents of a file. Can read specific line ranges.".to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        Self::input_schema()
    }

    fn is_read_only(&self) -> bool {
        true
    }

    async fn call(
        &self,
        args: serde_json::Value,
        context: ToolContext,
    ) -> Result<ToolResult, CliError> {
        let input: ReadInput = serde_json::from_value(args)?;
        let path = PathBuf::from(&input.path);

        let abs_path = if path.is_absolute() {
            path.clone()
        } else {
            context.working_directory.join(&path)
        };

        let metadata = tokio::fs::metadata(&abs_path).await
            .map_err(|e| CliError::ToolExecution(format!("cannot read file {}: {}", path.display(), e)))?;

        if metadata.len() as usize > self.max_file_size {
            return Ok(ToolResult::error(format!(
                "File too large ({} bytes). Maximum is {} bytes.",
                metadata.len(),
                self.max_file_size
            )));
        }

        let start = Instant::now();
        let content = tokio::fs::read_to_string(&abs_path).await
            .map_err(|e| CliError::ToolExecution(format!("cannot read file {}: {}", path.display(), e)))?;

        let duration_ms = start.elapsed().as_millis() as u64;

        let output = if let (Some(start), Some(end)) = (input.start_line, input.end_line) {
            let lines: Vec<&str> = content.lines().collect();
            let start_idx = start.saturating_sub(1);
            let end_idx = (end.min(lines.len())).saturating_sub(1);

            if start_idx > end_idx || start_idx >= lines.len() {
                return Ok(ToolResult::error("Invalid line range".to_string()));
            }

            lines[start_idx..=end_idx].join("\n")
        } else {
            content
        };

        Ok(ToolResult {
            content: vec![ResultContentBlock::Text { text: output }],
            is_error: false,
            metrics: Some(ToolMetrics {
                duration_ms,
                tokens_used: None,
            }),
        })
    }

    fn render_use_message(&self, args: &serde_json::Value) -> String {
        if let Ok(input) = serde_json::from_value::<ReadInput>(args.clone()) {
            format!("Reading file: {}", input.path)
        } else {
            "Reading file".to_string()
        }
    }
}
