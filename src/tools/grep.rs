//! Grep tool - search file contents with regex

use async_trait::async_trait;
use regex::Regex;
use serde::Deserialize;
use std::path::PathBuf;
use std::time::Instant;
use walkdir::WalkDir;

use crate::error::CliError;
use crate::types::{ResultContentBlock, Tool, ToolContext, ToolMetrics, ToolResult};

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GrepInput {
    pattern: String,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    #[serde(rename = "caseSensitive")]
    case_sensitive: bool,
    #[serde(default)]
    #[serde(rename = "includeFiles")]
    include_files: Option<String>,
    #[serde(default)]
    max_results: Option<usize>,
    #[serde(default)]
    context: Option<usize>,
}

pub struct GrepTool;

impl GrepTool {
    pub fn new() -> Self {
        Self
    }

    fn input_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regex pattern to search for"
                },
                "path": {
                    "type": "string",
                    "description": "Directory or file to search in (default: current directory)"
                },
                "caseSensitive": {
                    "type": "boolean",
                    "description": "Case sensitive search (default: false)"
                },
                "includeFiles": {
                    "type": "string",
                    "description": "Only search in files matching this pattern"
                },
                "maxResults": {
                    "type": "number",
                    "description": "Maximum number of results"
                },
                "context": {
                    "type": "number",
                    "description": "Number of context lines around matches"
                }
            },
            "required": ["pattern"]
        })
    }
}

impl Default for GrepTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "Grep"
    }

    fn aliases(&self) -> Vec<String> {
        vec!["grep".to_string(), "search".to_string(), "find".to_string()]
    }

    fn description(&self) -> String {
        "Search for a pattern in files using regex.".to_string()
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
        let input: GrepInput = serde_json::from_value(args)?;
        let start = Instant::now();

        let pattern = if input.case_sensitive {
            input.pattern.clone()
        } else {
            format!("(?i){}", input.pattern)
        };

        let regex = match Regex::new(&pattern) {
            Ok(r) => r,
            Err(e) => return Ok(ToolResult::error(format!("invalid regex: {e}"))),
        };

        let search_path = input
            .path
            .map(PathBuf::from)
            .unwrap_or_else(|| context.working_directory.clone());

        let max_results = input.max_results.unwrap_or(100);
        let context_lines = input.context.unwrap_or(0);

        let mut results = Vec::new();
        let mut count = 0;

        let entries = WalkDir::new(&search_path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file());

        for entry in entries {
            if count >= max_results {
                break;
            }

            let file_path = entry.path();
            if let Ok(content) = tokio::fs::read_to_string(file_path).await {
                let all_lines: Vec<&str> = content.lines().collect();
                for (line_num, line) in all_lines.iter().enumerate() {
                    if regex.is_match(line) {
                        let path_str = file_path.display().to_string();
                        let ctx_start = line_num.saturating_sub(context_lines);

                        let mut text = format!("{}:{}: {}", path_str, line_num + 1, line);

                        if ctx_start < line_num {
                            let before_lines: Vec<&str> = all_lines[ctx_start..line_num].to_vec();
                            if !before_lines.is_empty() {
                                let before_text: String = before_lines
                                    .iter()
                                    .rev()
                                    .enumerate()
                                    .map(|(i, l)| format!("{}: {}", line_num - i, l))
                                    .collect::<Vec<_>>()
                                    .join("\n");
                                text = format!("{}\n{}", before_text, text);
                            }
                        }

                        results.push(ResultContentBlock::Text { text });
                        count += 1;

                        if count >= max_results {
                            break;
                        }
                    }
                }
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        if results.is_empty() {
            return Ok(ToolResult {
                content: vec![ResultContentBlock::Text {
                    text: "No matches found.".to_string(),
                }],
                is_error: false,
                metrics: Some(ToolMetrics {
                    duration_ms,
                    tokens_used: None,
                }),
            });
        }

        Ok(ToolResult {
            content: results,
            is_error: false,
            metrics: Some(ToolMetrics {
                duration_ms,
                tokens_used: None,
            }),
        })
    }

    fn render_use_message(&self, args: &serde_json::Value) -> String {
        if let Ok(input) = serde_json::from_value::<GrepInput>(args.clone()) {
            format!("Searching for: {}", input.pattern)
        } else {
            "Searching files".to_string()
        }
    }
}
