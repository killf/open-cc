//! CoordinatorAgentBackend — implements AgentBackend using Coordinator

use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::coordinator::Coordinator;
use crate::tools::agent_tool::AgentBackend;
use crate::api::client::ApiClient;
use crate::error::CliError;
use crate::types::Tool;

/// Backend that delegates to Coordinator::spawn_agent
pub struct CoordinatorAgentBackend {
    pub coordinator: Arc<Coordinator>,
    pub api_client: Arc<ApiClient>,
    pub model: String,
    pub tools: Arc<Vec<Arc<dyn Tool>>>,
    pub working_directory: PathBuf,
    pub env: HashMap<String, String>,
}

#[async_trait]
impl AgentBackend for CoordinatorAgentBackend {
    async fn run_agent(
        &self,
        prompt: String,
        _system_prompt: Option<String>,
    ) -> Result<String, CliError> {
        self.coordinator
            .spawn_agent(
                (*self.api_client).clone(),
                self.model.clone(),
                (*self.tools).clone(),
                self.working_directory.clone(),
                self.env.clone(),
                prompt,
            )
            .await
    }
}
