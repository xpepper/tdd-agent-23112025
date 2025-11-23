use std::{collections::HashMap, env};

use tdd_core::config::{RoleConfig, TddConfig};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct RoleModelConfig {
    pub model: String,
    pub temperature: f32,
}

impl From<&RoleConfig> for RoleModelConfig {
    fn from(value: &RoleConfig) -> Self {
        Self {
            model: value.model.clone(),
            temperature: value.temperature,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LlmClientSettings {
    pub base_url: String,
    pub api_key_env: String,
    pub role_models: HashMap<String, RoleModelConfig>,
}

impl LlmClientSettings {
    pub fn from_core_config(config: &TddConfig) -> Self {
        let mut role_models = HashMap::new();
        role_models.insert("tester".into(), RoleModelConfig::from(&config.roles.tester));
        role_models.insert(
            "implementor".into(),
            RoleModelConfig::from(&config.roles.implementor),
        );
        role_models.insert(
            "refactorer".into(),
            RoleModelConfig::from(&config.roles.refactorer),
        );

        Self {
            base_url: config.llm.base_url.clone(),
            api_key_env: config.llm.api_key_env.clone(),
            role_models,
        }
    }

    pub fn role(&self, key: &str) -> Option<&RoleModelConfig> {
        self.role_models.get(key)
    }

    pub fn resolve_api_key(&self) -> Result<String, LlmConfigError> {
        env::var(&self.api_key_env)
            .map_err(|_| LlmConfigError::MissingEnvVar(self.api_key_env.clone()))
    }
}

#[derive(Debug, Error)]
pub enum LlmConfigError {
    #[error("environment variable {0} is not set")]
    MissingEnvVar(String),
}
