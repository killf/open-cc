//! Permission system types

use serde::{Deserialize, Serialize};

/// Permission mode for tool execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PermissionMode {
    AcceptEdits,
    BypassPermissions,
    #[default]
    Default,
    DontAsk,
    Plan,
    Auto,
    Bubble,
}

impl std::fmt::Display for PermissionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AcceptEdits => write!(f, "acceptEdits"),
            Self::BypassPermissions => write!(f, "bypassPermissions"),
            Self::Default => write!(f, "default"),
            Self::DontAsk => write!(f, "dontAsk"),
            Self::Plan => write!(f, "plan"),
            Self::Auto => write!(f, "auto"),
            Self::Bubble => write!(f, "bubble"),
        }
    }
}

impl std::str::FromStr for PermissionMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "acceptEdits" => Ok(Self::AcceptEdits),
            "bypassPermissions" => Ok(Self::BypassPermissions),
            "default" => Ok(Self::Default),
            "dontAsk" => Ok(Self::DontAsk),
            "plan" => Ok(Self::Plan),
            "auto" => Ok(Self::Auto),
            "bubble" => Ok(Self::Bubble),
            _ => Err(format!("unknown permission mode: {s}")),
        }
    }
}

/// Permission evaluation result
#[derive(Debug, Clone)]
pub enum PermissionDecision {
    Allow,
    Deny(String),
    Ask {
        message: String,
        suggestions: Vec<PermissionUpdate>,
    },
    Passthrough(String),
}

/// Suggested permission update
#[derive(Debug, Clone)]
pub struct PermissionUpdate {
    pub path: String,
    pub value: serde_json::Value,
}

/// Permission rule for tool-specific permissions
#[derive(Debug, Clone)]
pub struct PermissionRule {
    pub tool_name: String,
    pub path_pattern: Option<String>,
    pub content_pattern: Option<String>,
    pub decision: PermissionDecision,
}

impl PermissionRule {
    pub fn allow_all(tool_name: impl Into<String>) -> Self {
        Self {
            tool_name: tool_name.into(),
            path_pattern: None,
            content_pattern: None,
            decision: PermissionDecision::Allow,
        }
    }
}
