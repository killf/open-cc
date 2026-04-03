//! Glob tool - find files matching patterns

use async_trait::async_trait;
use glob::Pattern;
use serde::Deserialize;
use std::path::PathBuf;
use std::time::Instant;
use walkdir::WalkDir;

use crate::error::CliError;
use crate::types::{ResultContentBlock, Tool, ToolContext, ToolMetrics, ToolResult};

#[derive(Debug, Deserialize)]
struct GlobInput {
    pattern: String,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    #[serde(rename = "maxResults")]
    max_results: Option<usize>,
}

pub struct GlobTool;

impl GlobTool {
    pub fn new() -> Self {
        Self
    }

    fn input_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern (e.g., '**/*.rs', 'src/**/*.ts')"
                },
                "path": {
                    "type": "string",
                    "description": "Root directory to search from"
                },
                "maxResults": {
                    "type": "number",
                    "description": "Maximum number of results"
                }
            },
            "required": ["pattern"]
        })
    }
}

impl Default for GlobTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &str {
        "Glob"
    }

    fn aliases(&self) -> Vec<String> {
        vec!["glob".to_string(), "find".to_string(), "list".to_string()]
    }

    fn description(&self) -> String {
        "Find files matching a glob pattern.".to_string()
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
        let input: GlobInput = serde_json::from_value(args)?;
        let start = Instant::now();

        let search_path = input
            .path
            .map(PathBuf::from)
            .unwrap_or_else(|| context.working_directory.clone());

        let glob_pattern = Pattern::new(&input.pattern)
            .map_err(|e| CliError::ToolExecution(format!("invalid glob pattern: {e}")))?;

        let max_results = input.max_results.unwrap_or(100);
        let mut matches = Vec::new();

        for entry in WalkDir::new(&search_path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
            .take(10_000)
        {
            if matches.len() >= max_results {
                break;
            }
            if entry.file_type().is_file() {
                let path_str = entry.path().display().to_string();
                if let Ok(relative) = entry.path().strip_prefix(&search_path) {
                    let rel_str = relative.display().to_string();
                    if glob_pattern.matches(&rel_str) || glob_pattern.matches(&path_str) {
                        matches.push(rel_str);
                    }
                }
            }
        }

        matches.sort();
        let duration_ms = start.elapsed().as_millis() as u64;

        let text = if matches.is_empty() {
            "No files found matching the pattern.".to_string()
        } else {
            format!("{} files found:\n{}", matches.len(), matches.join("\n"))
        };

        Ok(ToolResult {
            content: vec![ResultContentBlock::Text { text }],
            is_error: false,
            metrics: Some(ToolMetrics {
                duration_ms,
                tokens_used: None,
            }),
        })
    }

    fn render_use_message(&self, args: &serde_json::Value) -> String {
        if let Ok(input) = serde_json::from_value::<GlobInput>(args.clone()) {
            format!("Finding files matching: {}", input.pattern)
        } else {
            "Finding files".to_string()
        }
    }
}
