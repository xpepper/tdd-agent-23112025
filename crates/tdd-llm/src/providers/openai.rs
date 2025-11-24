use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

use crate::client::{LlmClient, LlmError, Message};
use crate::config::{LlmClientSettings, RoleModelConfig};

/// Concrete implementation that talks to OpenAI-compatible endpoints.
pub struct OpenAiClient {
    client: reqwest::Client,
    settings: Arc<LlmClientSettings>,
    role_models: HashMap<String, RoleModelConfig>,
}

impl OpenAiClient {
    pub fn new(settings: LlmClientSettings) -> Result<Self, LlmError> {
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

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Self {
            role_models: settings.role_models.clone(),
            client,
            settings: Arc::new(settings),
        })
    }

    fn request_body(
        &self,
        role: &str,
        model: &RoleModelConfig,
        messages: &[Message],
    ) -> OpenAiChatRequest {
        OpenAiChatRequest {
            model: model.model.clone(),
            temperature: model.temperature,
            messages: messages.to_vec(),
            metadata: Some(OpenAiMetadata {
                role: role.to_string(),
            }),
        }
    }
}

#[async_trait]
impl LlmClient for OpenAiClient {
    async fn chat(&self, role: &str, messages: &[Message]) -> Result<String, LlmError> {
        let model_cfg = self
            .role_models
            .get(role)
            .ok_or_else(|| LlmError::MissingRoleConfig(role.to_string()))?;

        let payload = self.request_body(role, model_cfg, messages);
        let response: OpenAiChatResponse = self
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
struct OpenAiChatRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<OpenAiMetadata>,
}

#[derive(Debug, Serialize)]
struct OpenAiMetadata {
    role: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiChatResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiResponseMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponseMessage {
    content: String,
}
