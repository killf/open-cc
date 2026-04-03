//! Multi-agent coordinator for parallel task execution

#![allow(dead_code)]

pub mod agent;

use crate::agent::context::AgentContext;
use crate::agent::engine::{AgentEngine, AgentOutcome};
use crate::api::client::ApiClient;
use crate::config::{GlobalConfig, ProjectConfig};
use crate::error::CliError;
use crate::types::{Session, SessionConfig, Tool};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// A sub-agent task
#[derive(Debug, Clone)]
pub struct SubAgentTask {
    pub id: String,
    pub prompt: String,
    pub model: Option<String>,
    pub max_tokens: Option<u32>,
    pub status: TaskStatus,
    pub result: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// The coordinator manages multiple sub-agents
pub struct Coordinator {
    tasks: Arc<RwLock<HashMap<String, SubAgentTask>>>,
    result_tx: mpsc::UnboundedSender<TaskResult>,
}

pub struct TaskResult {
    pub task_id: String,
    pub output: String,
    pub error: Option<String>,
}

impl Coordinator {
    /// Create a new coordinator
    pub fn new() -> (Self, mpsc::UnboundedReceiver<TaskResult>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (
            Self {
                tasks: Arc::new(RwLock::new(HashMap::new())),
                result_tx: tx,
            },
            rx,
        )
    }

    /// Spawn a sub-agent task
    pub async fn spawn(&self, task: SubAgentTask) -> String {
        let id = task.id.clone();
        let mut tasks = self.tasks.write().await;
        tasks.insert(id.clone(), task);
        id
    }

    /// Get task status
    pub async fn get_task(&self, id: &str) -> Option<SubAgentTask> {
        let tasks = self.tasks.read().await;
        tasks.get(id).cloned()
    }

    /// List all tasks
    pub async fn list_tasks(&self) -> Vec<SubAgentTask> {
        let tasks = self.tasks.read().await;
        tasks.values().cloned().collect()
    }

    /// Cancel a task
    pub async fn cancel(&self, id: &str) -> bool {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(id) {
            if task.status == TaskStatus::Pending || task.status == TaskStatus::Running {
                task.status = TaskStatus::Cancelled;
                return true;
            }
        }
        false
    }

    /// Wait for all tasks to complete
    pub async fn wait_all(&self) -> Vec<TaskResult> {
        let mut results = Vec::new();
        loop {
            let pending = {
                let tasks = self.tasks.read().await;
                tasks.values().filter(|t| {
                    t.status == TaskStatus::Pending || t.status == TaskStatus::Running
                }).count()
            };
            if pending == 0 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        let tasks = self.tasks.read().await;
        for task in tasks.values() {
            results.push(TaskResult {
                task_id: task.id.clone(),
                output: task.result.clone().unwrap_or_default(),
                error: task.error.clone(),
            });
        }
        results
    }

    /// Spawn a sub-agent task and return its output as a string.
    ///
    /// Creates a fresh session and AgentEngine, runs the prompt, and returns
    /// the final assistant text from the session.
    pub async fn spawn_agent(
        &self,
        api_client: ApiClient,
        model: String,
        tools: Vec<Arc<dyn Tool>>,
        working_directory: PathBuf,
        env: HashMap<String, String>,
        prompt: String,
    ) -> Result<String, CliError> {
        let session_config = SessionConfig { model: model.clone(), ..Default::default() };
        let session = Session::new(uuid::Uuid::new_v4().to_string(), model);

        let context = AgentContext::new(
            session,
            session_config,
            tools,
            working_directory,
            GlobalConfig::default(),
            ProjectConfig::default(),
            env,
            None,
        );

        let mut engine = AgentEngine::new(api_client, context);

        match engine.run(prompt).await {
            Ok(AgentOutcome::Completed) => {
                let output = engine
                    .session()
                    .messages
                    .iter()
                    .filter_map(|msg| {
                        if let crate::types::Message::Assistant { content: Some(ref c) } = msg {
                            Some(c.content.iter().filter_map(|b| {
                                if let crate::types::ContentBlock::Text { ref text } = b {
                                    Some(text.as_str())
                                } else {
                                    None
                                }
                            }).collect::<Vec<_>>().join(""))
                        } else {
                            None
                        }
                    })
                    .next_back()
                    .unwrap_or_default();
                Ok(output)
            }
            Ok(AgentOutcome::Error(msg)) => Err(CliError::Other(msg)),
            Ok(AgentOutcome::Interrupted) => Err(CliError::Other("agent interrupted".to_string())),
            Err(e) => Err(e),
        }
    }

    /// Run multiple sub-agents in parallel and collect their outputs.
    ///
    /// Each prompt gets its own session and agent engine. Results are returned
    /// in the same order as the input prompts.
    pub async fn run_multi(
        &self,
        api_client: ApiClient,
        model: String,
        tools: Vec<Arc<dyn Tool>>,
        working_directory: PathBuf,
        env: HashMap<String, String>,
        prompts: Vec<String>,
    ) -> Vec<Result<String, CliError>> {
        let mut handles = Vec::new();

        for prompt in prompts {
            let api = api_client.clone();
            let model = model.clone();
            let tools = tools.clone();
            let wd = working_directory.clone();
            let env = env.clone();
            let result_tx = self.result_tx.clone();

            let handle = tokio::spawn(async move {
                let session_config = SessionConfig { model: model.clone(), ..Default::default() };
                let session = Session::new(uuid::Uuid::new_v4().to_string(), model.clone());

                let context = AgentContext::new(
                    session,
                    session_config,
                    tools,
                    wd,
                    GlobalConfig::default(),
                    ProjectConfig::default(),
                    env,
                    None,
                );

                let mut engine = AgentEngine::new(api, context);

                let output = match engine.run(prompt).await {
                    Ok(AgentOutcome::Completed) => {
                        engine.session().messages.iter()
                            .filter_map(|msg| {
                                if let crate::types::Message::Assistant { content: Some(ref c) } = msg {
                                    Some(c.content.iter().filter_map(|b| {
                                        if let crate::types::ContentBlock::Text { ref text } = b {
                                            Some(text.as_str())
                                        } else { None }
                                    }).collect::<Vec<_>>().join(""))
                                } else { None }
                            })
                            .next_back()
                            .unwrap_or_default()
                    }
                    Ok(AgentOutcome::Error(msg)) => return Err(CliError::Other(msg)),
                    Ok(AgentOutcome::Interrupted) => return Err(CliError::Other("agent interrupted".to_string())),
                    Err(e) => return Err(e),
                };

                let _ = result_tx.send(TaskResult {
                    task_id: uuid::Uuid::new_v4().to_string(),
                    output: output.clone(),
                    error: None,
                });
                Ok(output)
            });

            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(r) => results.push(r),
                Err(e) => results.push(Err(CliError::Other(format!("task panicked: {e}")))),
            }
        }
        results
    }

    /// Run multiple agents in parallel
    pub async fn run_parallel<F>(
        &self,
        prompts: Vec<String>,
        spawn_fn: F,
    ) -> Result<Vec<String>, CliError>
    where
        F: Fn(String, usize) -> tokio::task::JoinHandle<Result<String, CliError>> + Send + Sync,
    {
        let handles: Vec<_> = prompts
            .into_iter()
            .enumerate()
            .map(|(i, prompt)| spawn_fn(prompt, i))
            .collect();

        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(Ok(output)) => results.push(output),
                Ok(Err(e)) => return Err(e),
                Err(e) => {
                    return Err(CliError::Other(format!("task panicked: {e}")))
                }
            }
        }

        Ok(results)
    }
}

impl Default for Coordinator {
    fn default() -> Self {
        let (this, _) = Self::new();
        this
    }
}

/// Task assignment strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentStrategy {
    /// Assign tasks to agents round-robin
    RoundRobin,
    /// Assign based on agent specialization
    Specialized,
    /// Let the coordinator decide dynamically
    Dynamic,
}

/// Coordinator configuration
#[derive(Debug, Clone)]
pub struct CoordinatorConfig {
    pub max_parallel: usize,
    pub strategy: AssignmentStrategy,
    pub timeout_secs: u64,
    pub retry_on_failure: bool,
    pub max_retries: usize,
}

impl Default for CoordinatorConfig {
    fn default() -> Self {
        Self {
            max_parallel: 4,
            strategy: AssignmentStrategy::Dynamic,
            timeout_secs: 300,
            retry_on_failure: true,
            max_retries: 2,
        }
    }
}
