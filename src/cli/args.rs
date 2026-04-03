//! CLI argument parsing

#![allow(dead_code)]

use clap::{Parser, ValueHint};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "claude-code",
    about = "Claude Code CLI - AI coding assistant",
    version,
    author = "Anthropic"
)]
pub struct CliArgs {
    /// The prompt to send to Claude
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub prompt: Vec<String>,

    /// Print response only (non-interactive mode)
    #[arg(short, long)]
    pub print: bool,

    /// Output response to file instead of terminal
    #[arg(short, long, value_hint = ValueHint::FilePath)]
    pub output: Option<PathBuf>,

    /// Resume an existing session by ID
    #[arg(long, value_name = "session-id")]
    pub resume: Option<String>,

    /// Skip permission prompts (dangerous)
    #[arg(long, value_name = "mode")]
    pub dangerously_skip_permission: Option<String>,

    /// Permission mode: accept-edits, bypass-permissions, plan, dont-ask, auto
    #[arg(long, env = "CLAUDE_PERMISSION_MODE")]
    pub permission_mode: Option<String>,

    /// Skip confirmation prompts
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// Additional environment variables (KEY=VALUE)
    #[arg(long, value_name = "KEY=VALUE")]
    pub add_env: Vec<String>,

    /// Specify model to use
    #[arg(long, env = "CLAUDE_MODEL")]
    pub model: Option<String>,

    /// Specify maximum tokens
    #[arg(long)]
    pub max_tokens: Option<u32>,

    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Omit additional metadata from output
    #[arg(long)]
    pub additive_slash_enabled: bool,

    /// Headless mode (no terminal)
    #[arg(long)]
    pub headless: bool,

    /// MCP server config (JSON)
    #[arg(long, value_name = "JSON")]
    pub mcp_config: Option<String>,

    /// Specify custom instructions
    #[arg(long, value_name = "text")]
    pub system_prompt: Option<String>,

    /// Disable automatic context
    #[arg(long)]
    pub disable_auto_context: bool,

    /// List previous sessions
    #[arg(long)]
    pub list_sessions: bool,

    /// Print session transcript
    #[arg(long)]
    pub print_sessions: Option<String>,

    /// Spawn a sub-agent
    #[arg(long)]
    pub spawn: Option<String>,
}

impl CliArgs {
    /// Build a combined prompt string from the prompt args
    pub fn combined_prompt(&self) -> Option<String> {
        if self.prompt.is_empty() {
            None
        } else {
            Some(self.prompt.join(" "))
        }
    }

    /// Parse additional env vars into a HashMap
    pub fn parse_env_vars(&self) -> std::collections::HashMap<String, String> {
        self.add_env
            .iter()
            .filter_map(|kv| {
                let mut parts = kv.splitn(2, '=');
                match (parts.next(), parts.next()) {
                    (Some(k), Some(v)) => Some((k.to_string(), v.to_string())),
                    _ => None,
                }
            })
            .collect()
    }
}
