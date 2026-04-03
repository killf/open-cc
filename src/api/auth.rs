//! Authentication utilities

use std::env;

use crate::config::ModelProvider;
use crate::error::CliError;

/// Resolve API key from various sources
pub async fn resolve_api_key(
    provider: ModelProvider,
    explicit_key: Option<&str>,
) -> Result<String, CliError> {
    // 1. Explicitly provided key takes precedence
    if let Some(key) = explicit_key {
        if !key.is_empty() {
            return Ok(key.to_string());
        }
    }

    // 2. Environment variable
    let env_var = match provider {
        ModelProvider::Anthropic => "ANTHROPIC_API_KEY",
        ModelProvider::AwsBedrock => "AWS_ACCESS_KEY_ID",
        ModelProvider::GcpVertex => "GOOGLE_API_KEY",
        ModelProvider::Azure => "AZURE_OPENAI_KEY",
        ModelProvider::OpenAi => "OPENAI_API_KEY",
        ModelProvider::Ollama => "OLLAMA_API_KEY",
        ModelProvider::Together => "TOGETHER_API_KEY",
    };

    if let Ok(key) = env::var(env_var) {
        if !key.is_empty() {
            return Ok(key);
        }
    }

    // 3. ANTHROPIC_API_KEY as fallback for all providers
    if provider != ModelProvider::Anthropic {
        if let Ok(key) = env::var("ANTHROPIC_API_KEY") {
            if !key.is_empty() {
                return Ok(key);
            }
        }
    }

    Err(CliError::ApiKeyNotFound)
}

/// Get base URL for a provider
pub fn get_base_url(provider: ModelProvider, explicit_url: Option<&str>) -> String {
    if let Some(url) = explicit_url {
        return url.to_string();
    }

    match provider {
        ModelProvider::Anthropic => "https://api.anthropic.com".to_string(),
        ModelProvider::AwsBedrock => {
            "https://bedrock.us-east-1.amazonaws.com".to_string()
        }
        ModelProvider::GcpVertex => {
            "https://us-central1-aiplatform.googleapis.com/v1".to_string()
        }
        ModelProvider::Azure => {
            "https://{resource}.openai.azure.com".to_string()
        }
        ModelProvider::OpenAi => "https://api.openai.com/v1".to_string(),
        ModelProvider::Ollama => "http://localhost:11434".to_string(),
        ModelProvider::Together => "https://api.together.xyz/v1".to_string(),
    }
}
