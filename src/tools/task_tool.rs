//! Task tool - create, list, update, and get sub-tasks

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::error::CliError;
use crate::types::{TaskState, TaskStatus, TaskType, Tool, ToolContext, ToolResult};

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TaskInput {
    action: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    status: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
struct TaskStore {
    tasks: Vec<TaskState>,
}

pub struct TaskTool {
    tasks_dir: PathBuf,
    /// In-memory cache to avoid repeated file reads on every call.
    /// We use Arc<Mutex<Option<TaskStore>>> so we can load once and re-use.
    cache: Arc<Mutex<Option<TaskStore>>>,
}

impl TaskTool {
    pub fn new(tasks_dir: PathBuf) -> Self {
        Self {
            tasks_dir,
            cache: Arc::new(Mutex::new(None)),
        }
    }

    fn tasks_file(&self) -> PathBuf {
        self.tasks_dir.join("tasks.json")
    }

    /// Load all tasks from disk into the cache.
    async fn load_store(&self) -> Result<TaskStore, CliError> {
        let mut guard = self.cache.lock().await;
        if let Some(ref store) = *guard {
            return Ok(store.clone());
        }
        let file = self.tasks_file();
        let store = if file.exists() {
            let content = fs::read_to_string(&file)?;
            serde_json::from_str(&content)?
        } else {
            TaskStore::default()
        };
        *guard = Some(store.clone());
        Ok(store)
    }

    /// Persist the in-memory cache to disk.
    async fn save_store(&self, store: &TaskStore) -> Result<(), CliError> {
        fs::create_dir_all(&self.tasks_dir)?;
        let content = serde_json::to_string_pretty(store)?;
        fs::write(self.tasks_file(), content)?;
        // update cache
        let mut guard = self.cache.lock().await;
        *guard = Some(store.clone());
        Ok(())
    }

    fn parse_status(s: &str) -> Result<TaskStatus, CliError> {
        match s {
            "pending" => Ok(TaskStatus::Pending),
            "running" => Ok(TaskStatus::Running),
            "completed" => Ok(TaskStatus::Completed),
            "failed" => Ok(TaskStatus::Failed),
            "killed" => Ok(TaskStatus::Killed),
            _ => Err(CliError::Other(format!("unknown status: {s}"))),
        }
    }

    fn format_status(status: TaskStatus) -> &'static str {
        match status {
            TaskStatus::Pending => "pending",
            TaskStatus::Running => "running",
            TaskStatus::Completed => "completed",
            TaskStatus::Failed => "failed",
            TaskStatus::Killed => "killed",
        }
    }

    async fn do_create(&self, description: String) -> Result<ToolResult, CliError> {
        let mut store = self.load_store().await?;
        let id = uuid::Uuid::new_v4().to_string();
        let task = TaskState::new(id.clone(), TaskType::InProcessTeammate, description);
        store.tasks.push(task);
        self.save_store(&store).await?;
        Ok(ToolResult::text(format!("Created task: {id}")))
    }

    async fn do_list(&self) -> Result<ToolResult, CliError> {
        let store = self.load_store().await?;
        if store.tasks.is_empty() {
            return Ok(ToolResult::text("No tasks found."));
        }
        let lines: Vec<String> = store
            .tasks
            .iter()
            .map(|t| {
                format!(
                    "[{}] ({}) {}",
                    t.id,
                    Self::format_status(t.status),
                    t.description
                )
            })
            .collect();
        Ok(ToolResult::text(lines.join("\n")))
    }

    async fn do_update(
        &self,
        id: String,
        status: String,
    ) -> Result<ToolResult, CliError> {
        let mut store = self.load_store().await?;
        let task = store
            .tasks
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| CliError::Other(format!("task not found: {id}")))?;
        let new_status = Self::parse_status(&status)?;
        task.status = new_status;
        // Set end_time for terminal statuses
        if matches!(new_status, TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Killed) {
            task.end_time = Some(chrono::Utc::now().timestamp_millis());
        }
        self.save_store(&store).await?;
        Ok(ToolResult::text(format!(
            "Updated task {} to status: {}",
            id, status
        )))
    }

    async fn do_get(&self, id: String) -> Result<ToolResult, CliError> {
        let store = self.load_store().await?;
        let task = store
            .tasks
            .iter()
            .find(|t| t.id == id)
            .ok_or_else(|| CliError::Other(format!("task not found: {id}")))?;
        let info = format!(
            "id: {}\ntype: {}\nstatus: {}\ndescription: {}\nstart_time: {}\nend_time: {:?}",
            task.id,
            task.task_type,
            Self::format_status(task.status),
            task.description,
            task.start_time,
            task.end_time,
        );
        Ok(ToolResult::text(info))
    }
}

impl Default for TaskTool {
    fn default() -> Self {
        Self::new(std::env::temp_dir())
    }
}

#[async_trait]
impl Tool for TaskTool {
    fn name(&self) -> &str {
        "Task"
    }

    fn aliases(&self) -> Vec<String> {
        vec!["task".to_string()]
    }

    fn description(&self) -> String {
        "Create, list, update, and get sub-tasks.".to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Action to perform: create, list, update, get",
                    "enum": ["create", "list", "update", "get"]
                },
                "id": {
                    "type": "string",
                    "description": "Task id (required for update and get actions)"
                },
                "description": {
                    "type": "string",
                    "description": "Task description (required for create action)"
                },
                "status": {
                    "type": "string",
                    "description": "New status for the task (required for update action)",
                    "enum": ["pending", "running", "completed", "failed", "killed"]
                }
            },
            "required": ["action"]
        })
    }

    async fn call(
        &self,
        args: serde_json::Value,
        _context: ToolContext,
    ) -> Result<ToolResult, CliError> {
        let input: TaskInput = serde_json::from_value(args)?;
        match input.action.as_str() {
            "create" => {
                let description = input
                    .description
                    .ok_or_else(|| CliError::Other("description is required for create".into()))?;
                self.do_create(description).await
            }
            "list" => self.do_list().await,
            "update" => {
                let id = input
                    .id
                    .ok_or_else(|| CliError::Other("id is required for update".into()))?;
                let status = input
                    .status
                    .ok_or_else(|| CliError::Other("status is required for update".into()))?;
                self.do_update(id, status).await
            }
            "get" => {
                let id = input
                    .id
                    .ok_or_else(|| CliError::Other("id is required for get".into()))?;
                self.do_get(id).await
            }
            _ => Err(CliError::Other(format!(
                "unknown action: {}. Use create, list, update, or get.",
                input.action
            ))),
        }
    }

    fn render_use_message(&self, args: &serde_json::Value) -> String {
        format!("Using Task tool with args: {}", args)
    }

    fn render_result_message(&self, result: &ToolResult) -> String {
        let preview = result
            .content
            .iter()
            .map(|b| b.preview())
            .collect::<Vec<_>>()
            .join("; ");
        if preview.len() > 200 {
            format!("{}...", &preview[..200])
        } else {
            preview
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_context() -> ToolContext {
        ToolContext {
            session_id: "test-session".to_string(),
            agent_id: "test-agent".to_string(),
            working_directory: std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
            can_use_tool: true,
            parent_message_id: None,
            env: Default::default(),
        }
    }

    fn make_tool() -> TaskTool {
        let dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        TaskTool::new(dir)
    }

    #[tokio::test]
    async fn test_create_and_get() {
        let tool = make_tool();
        let ctx = make_context();

        let result = tool
            .call(
                serde_json::json!({"action": "create", "description": "test task"}),
                ctx.clone(),
            )
            .await
            .unwrap();

        let text = match &result.content[0] {
            crate::types::ResultContentBlock::Text { text } => text,
            _ => panic!("expected Text block"),
        };
        assert!(text.starts_with("Created task: "));
        let task_id = text.strip_prefix("Created task: ").unwrap().trim();

        let result = tool
            .call(
                serde_json::json!({"action": "get", "id": task_id}),
                ctx,
            )
            .await
            .unwrap();

        let text = match &result.content[0] {
            crate::types::ResultContentBlock::Text { text } => text.as_str(),
            _ => panic!("expected Text block"),
        };
        assert!(text.contains("test task"));
        assert!(text.contains("pending"));
    }

    #[tokio::test]
    async fn test_list_empty() {
        let tool = make_tool();
        let ctx = make_context();

        let result = tool
            .call(serde_json::json!({"action": "list"}), ctx)
            .await
            .unwrap();

        let text = match &result.content[0] {
            crate::types::ResultContentBlock::Text { text } => text.as_str(),
            _ => panic!("expected Text block"),
        };
        assert_eq!(text, "No tasks found.");
    }

    #[tokio::test]
    async fn test_list_after_create() {
        let tool = make_tool();
        let ctx = make_context();

        tool.call(
            serde_json::json!({"action": "create", "description": "task A"}),
            ctx.clone(),
        )
        .await
        .unwrap();

        tool.call(
            serde_json::json!({"action": "create", "description": "task B"}),
            ctx.clone(),
        )
        .await
        .unwrap();

        let result = tool
            .call(serde_json::json!({"action": "list"}), ctx)
            .await
            .unwrap();

        let text = match &result.content[0] {
            crate::types::ResultContentBlock::Text { text } => text.as_str(),
            _ => panic!("expected Text block"),
        };
        assert!(text.contains("task A"));
        assert!(text.contains("task B"));
    }

    #[tokio::test]
    async fn test_update_to_completed() {
        let tool = make_tool();
        let ctx = make_context();

        let result = tool
            .call(
                serde_json::json!({"action": "create", "description": "update me"}),
                ctx.clone(),
            )
            .await
            .unwrap();

        let task_id = match &result.content[0] {
            crate::types::ResultContentBlock::Text { text } => {
                text.strip_prefix("Created task: ").unwrap().trim().to_string()
            }
            _ => panic!("expected Text block"),
        };

        let result = tool
            .call(
                serde_json::json!({"action": "update", "id": task_id, "status": "completed"}),
                ctx.clone(),
            )
            .await
            .unwrap();

        let text = match &result.content[0] {
            crate::types::ResultContentBlock::Text { text } => text.as_str(),
            _ => panic!("expected Text block"),
        };
        assert!(text.contains("completed"));

        // get and verify end_time is set
        let result = tool
            .call(serde_json::json!({"action": "get", "id": task_id}), ctx)
            .await
            .unwrap();

        let text = match &result.content[0] {
            crate::types::ResultContentBlock::Text { text } => text.as_str(),
            _ => panic!("expected Text block"),
        };
        assert!(text.contains("completed"));
        assert!(text.contains("end_time:"));
    }

    #[tokio::test]
    async fn test_update_unknown_task() {
        let tool = make_tool();
        let ctx = make_context();

        let result = tool
            .call(
                serde_json::json!({"action": "update", "id": "nonexistent-id", "status": "running"}),
                ctx,
            )
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_get_unknown_task() {
        let tool = make_tool();
        let ctx = make_context();

        let result = tool
            .call(serde_json::json!({"action": "get", "id": "bad-id"}), ctx)
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_create_missing_description() {
        let tool = make_tool();
        let ctx = make_context();

        let result = tool.call(serde_json::json!({"action": "create"}), ctx).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("description"));
    }

    #[tokio::test]
    async fn test_update_missing_status() {
        let tool = make_tool();
        let ctx = make_context();

        let result = tool
            .call(serde_json::json!({"action": "update", "id": "some-id"}), ctx)
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("status"));
    }

    #[tokio::test]
    async fn test_unknown_action() {
        let tool = make_tool();
        let ctx = make_context();

        let result = tool
            .call(serde_json::json!({"action": "foobar"}), ctx)
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown action"));
    }
}
