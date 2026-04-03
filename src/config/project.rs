//! Project-specific configuration types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::McpServerConfig;

/// Project configuration (stored in .claude/settings.json)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectConfig {
    pub allowed_tools: Vec<String>,
    pub mcp_servers: Option<HashMap<String, McpServerConfig>>,
    pub last_api_duration_ms: Option<u64>,
    pub last_cost: Option<f64>,
    pub has_trust_dialog_accepted: Option<bool>,
    pub active_worktree_session: Option<String>,
    pub custom_commands: Vec<CommandDefinition>,
}

/// Custom command definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandDefinition {
    pub name: String,
    pub description: String,
    pub script: Option<String>,
    pub env: Option<HashMap<String, String>>,
    pub working_directory: Option<String>,
    pub timeout_secs: Option<u64>,
}
