//! Task types for Claude Code CLI

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Task type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    LocalBash,
    LocalAgent,
    RemoteAgent,
    InProcessTeammate,
    LocalWorkflow,
    MonitorMcp,
    Dream,
}

impl std::fmt::Display for TaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LocalBash => write!(f, "local_bash"),
            Self::LocalAgent => write!(f, "local_agent"),
            Self::RemoteAgent => write!(f, "remote_agent"),
            Self::InProcessTeammate => write!(f, "in_process_teammate"),
            Self::LocalWorkflow => write!(f, "local_workflow"),
            Self::MonitorMcp => write!(f, "monitor_mcp"),
            Self::Dream => write!(f, "dream"),
        }
    }
}

/// Task status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Killed,
}

/// Full task state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskState {
    pub id: String,
    pub task_type: TaskType,
    pub status: TaskStatus,
    pub description: String,
    pub tool_use_id: Option<String>,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub output_file: PathBuf,
    pub notified: bool,
}

impl TaskState {
    pub fn new(id: String, task_type: TaskType, description: String) -> Self {
        Self {
            id,
            task_type,
            status: TaskStatus::Pending,
            description,
            tool_use_id: None,
            start_time: chrono::Utc::now().timestamp_millis(),
            end_time: None,
            output_file: PathBuf::new(),
            notified: false,
        }
    }

    pub fn start(&mut self) {
        self.status = TaskStatus::Running;
        self.start_time = chrono::Utc::now().timestamp_millis();
    }

    pub fn complete(&mut self) {
        self.status = TaskStatus::Completed;
        self.end_time = Some(chrono::Utc::now().timestamp_millis());
    }

    pub fn fail(&mut self) {
        self.status = TaskStatus::Failed;
        self.end_time = Some(chrono::Utc::now().timestamp_millis());
    }

    pub fn kill(&mut self) {
        self.status = TaskStatus::Killed;
        self.end_time = Some(chrono::Utc::now().timestamp_millis());
    }
}
