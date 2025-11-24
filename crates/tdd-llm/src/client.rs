use async_trait::async_trait;
use serde::Serialize;
use std::sync::Arc;
use thiserror::Error;

use crate::config::LlmClientSettings;
use crate::providers::github::GitHubCopilotClient;
use crate::providers::openai::OpenAiClient;
use tdd_core::config::LlmProvider;

/// Role values accepted by OpenAI-compatible chat endpoints.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

/// Simple chat message representation shared across agents.
#[derive(Debug, Clone, Serialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
}

/// Errors emitted by LLM clients.
#[derive(Debug, Error)]
pub enum LlmError {
    #[error("missing role configuration for {0}")]
    MissingRoleConfig(String),
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("api key not configured in environment variable {0}")]
    MissingApiKey(String),
}

/// Trait describing the behavior required from LLM providers.
#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn chat(&self, role: &str, messages: &[Message]) -> Result<String, LlmError>;
}

/// Factory for creating LLM clients based on provider configuration.
pub fn create_client(
    provider: LlmProvider,
    settings: LlmClientSettings,
    api_version: Option<String>,
) -> Result<Arc<dyn LlmClient>, LlmError> {
    match provider {
        LlmProvider::Openai => {
            let client = OpenAiClient::new(settings)?;
            Ok(Arc::new(client))
        }
        LlmProvider::GithubCopilot => {
            let client = GitHubCopilotClient::new(settings, api_version)?;
            Ok(Arc::new(client))
        }
    }
}
