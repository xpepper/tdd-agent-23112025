use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

use crate::client::{LlmClient, LlmError, Message};
use crate::config::{LlmClientSettings, RoleModelConfig};

/// GitHub Copilot API client implementation.
/// Uses the same OpenAI-compatible endpoint structure but with GitHub-specific headers.
pub struct GitHubCopilotClient {
    client: reqwest::Client,
    settings: Arc<LlmClientSettings>,
    role_models: HashMap<String, RoleModelConfig>,
    api_version: String,
}

impl GitHubCopilotClient {
    pub fn new(settings: LlmClientSettings, api_version: Option<String>) -> Result<Self, LlmError> {
        let api_key = settings
            .resolve_api_key()
            .map_err(|_| LlmError::MissingApiKey(settings.api_key_env.clone()))?;

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {api_key}"))
                .map_err(|_| LlmError::MissingApiKey(settings.api_key_env.clone()))?,
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        // GitHub Copilot requires the X-GitHub-Api-Version header
        let version = api_version.unwrap_or_else(|| "2023-12-01".to_string());
        headers.insert(
            "X-GitHub-Api-Version",
            HeaderValue::from_str(&version)
                .map_err(|_| LlmError::MissingApiKey("invalid api_version header".to_string()))?,
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Self {
            role_models: settings.role_models.clone(),
            client,
            settings: Arc::new(settings),
            api_version: version,
        })
    }

    fn request_body(
        &self,
        role: &str,
        model: &RoleModelConfig,
        messages: &[Message],
    ) -> CopilotChatRequest {
        CopilotChatRequest {
            model: model.model.clone(),
            temperature: model.temperature,
            messages: messages.to_vec(),
            metadata: Some(CopilotMetadata {
                role: role.to_string(),
            }),
        }
    }
}

#[async_trait]
impl LlmClient for GitHubCopilotClient {
    async fn chat(&self, role: &str, messages: &[Message]) -> Result<String, LlmError> {
        let model_cfg = self
            .role_models
            .get(role)
            .ok_or_else(|| LlmError::MissingRoleConfig(role.to_string()))?;

        let payload = self.request_body(role, model_cfg, messages);
        let response: CopilotChatResponse = self
            .client
            .post(format!("{}/chat/completions", self.settings.base_url))
            .json(&payload)
            .send()
            .await?
            .json()
            .await?;

        Ok(response
            .choices
            .first()
            .map(|choice| choice.message.content.clone())
            .unwrap_or_default())
    }
}

#[derive(Debug, Serialize)]
struct CopilotChatRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<CopilotMetadata>,
}

#[derive(Debug, Serialize)]
struct CopilotMetadata {
    role: String,
}

#[derive(Debug, Deserialize)]
struct CopilotChatResponse {
    choices: Vec<CopilotChoice>,
}

#[derive(Debug, Deserialize)]
struct CopilotChoice {
    message: CopilotResponseMessage,
}

#[derive(Debug, Deserialize)]
struct CopilotResponseMessage {
    content: String,
}
