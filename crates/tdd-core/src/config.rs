use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error;

/// Top-level configuration loaded from `tdd.yaml`.
#[derive(Debug, Clone, Deserialize)]
pub struct TddConfig {
    pub workspace: WorkspaceConfig,
    pub roles: RoleConfigs,
    pub llm: LlmConfig,
    pub ci: CiConfig,
    pub commit_author: CommitAuthor,
}

impl TddConfig {
    /// Load configuration from a YAML file, returning a validated structure.
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let raw = fs::read_to_string(path).map_err(|err| ConfigError::Io {
            path: path.to_path_buf(),
            source: err,
        })?;
        let mut config: TddConfig =
            serde_yaml::from_str(&raw).map_err(|source| ConfigError::Parse {
                path: path.to_path_buf(),
                source,
            })?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&mut self) -> Result<(), ConfigError> {
        self.workspace.normalize();
        self.ci.ensure_all_commands_present()?;
        self.roles.validate()?;
        self.llm.validate()?;
        self.commit_author.validate()?;
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceConfig {
    pub kata_file: PathBuf,
    pub plan_dir: PathBuf,
    pub log_dir: PathBuf,
    #[serde(default = "WorkspaceConfig::default_max_steps")]
    pub max_steps: u32,
    #[serde(default = "WorkspaceConfig::default_attempts")]
    pub max_attempts_per_agent: u32,
}

impl WorkspaceConfig {
    fn default_max_steps() -> u32 {
        10
    }

    fn default_attempts() -> u32 {
        2
    }

    fn normalize(&mut self) {
        if self.max_steps == 0 {
            self.max_steps = Self::default_max_steps();
        }
        if self.max_attempts_per_agent == 0 {
            self.max_attempts_per_agent = Self::default_attempts();
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RoleConfigs {
    pub tester: RoleConfig,
    pub implementor: RoleConfig,
    pub refactorer: RoleConfig,
}

impl RoleConfigs {
    fn validate(&self) -> Result<(), ConfigError> {
        self.tester.validate("tester")?;
        self.implementor.validate("implementor")?;
        self.refactorer.validate("refactorer")?;
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RoleConfig {
    pub model: String,
    #[serde(default = "RoleConfig::default_temperature")]
    pub temperature: f32,
}

impl RoleConfig {
    fn default_temperature() -> f32 {
        0.2
    }

    fn validate(&self, role: &str) -> Result<(), ConfigError> {
        if self.model.trim().is_empty() {
            return Err(ConfigError::InvalidField {
                field: format!("roles.{role}.model"),
                reason: "model cannot be empty".into(),
            });
        }

        if !(0.0..=2.0).contains(&self.temperature) {
            return Err(ConfigError::InvalidField {
                field: format!("roles.{role}.temperature"),
                reason: "temperature must be between 0.0 and 2.0".into(),
            });
        }

        Ok(())
    }
}

/// LLM provider selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LlmProvider {
    #[default]
    Openai,
    GithubCopilot,
}

impl LlmProvider {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Openai => "openai",
            Self::GithubCopilot => "github_copilot",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmConfig {
    #[serde(default)]
    pub provider: LlmProvider,
    pub base_url: String,
    pub api_key_env: String,
    #[serde(default)]
    pub api_version: Option<String>,
}

impl LlmConfig {
    fn validate(&self) -> Result<(), ConfigError> {
        if self.base_url.trim().is_empty() {
            return Err(ConfigError::InvalidField {
                field: "llm.base_url".into(),
                reason: "base_url cannot be empty".into(),
            });
        }
        if self.api_key_env.trim().is_empty() {
            return Err(ConfigError::InvalidField {
                field: "llm.api_key_env".into(),
                reason: "api_key_env cannot be empty".into(),
            });
        }
        // For GitHub Copilot, api_version should be provided (default to standard version if not)
        if self.provider == LlmProvider::GithubCopilot && self.api_version.is_none() {
            // This is acceptable; we'll use a default in the client
        }
        Ok(())
    }

    /// Get the API version, providing a default for GitHub Copilot if not specified.
    pub fn effective_api_version(&self) -> Option<String> {
        match self.provider {
            LlmProvider::GithubCopilot => Some(
                self.api_version
                    .clone()
                    .unwrap_or_else(|| "2023-12-01".to_string()),
            ),
            LlmProvider::Openai => self.api_version.clone(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CiConfig {
    pub fmt: Vec<String>,
    pub check: Vec<String>,
    pub test: Vec<String>,
}

impl CiConfig {
    fn ensure_all_commands_present(&self) -> Result<(), ConfigError> {
        for (field, cmds) in [
            ("ci.fmt", &self.fmt),
            ("ci.check", &self.check),
            ("ci.test", &self.test),
        ] {
            if cmds.is_empty() {
                return Err(ConfigError::InvalidField {
                    field: field.into(),
                    reason: "command list cannot be empty".into(),
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CommitAuthor {
    pub name: String,
    pub email: String,
}

impl CommitAuthor {
    fn validate(&self) -> Result<(), ConfigError> {
        if self.name.trim().is_empty() {
            return Err(ConfigError::InvalidField {
                field: "commit_author.name".into(),
                reason: "name cannot be empty".into(),
            });
        }
        if self.email.trim().is_empty() {
            return Err(ConfigError::InvalidField {
                field: "commit_author.email".into(),
                reason: "email cannot be empty".into(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse YAML config {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },
    #[error("invalid configuration at {field}: {reason}")]
    InvalidField { field: String, reason: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn loads_valid_config() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(
            r#"workspace:
    kata_file: kata.md
    plan_dir: .tdd/plan
    log_dir: .tdd/logs
roles:
    tester:
        model: gpt-4o-mini
        temperature: 0.1
    implementor:
        model: gpt-4o-mini
        temperature: 0.2
    refactorer:
        model: gpt-4o-mini
        temperature: 0.15
llm:
    provider: openai
    base_url: https://api.example.com
    api_key_env: API_KEY
ci:
    fmt: ["cargo", "fmt"]
    check: ["cargo", "clippy", "-D", "warnings"]
    test: ["cargo", "test"]
commit_author:
    name: Example
    email: example@example.com
"#
            .as_bytes(),
        )
        .unwrap();

        let config = TddConfig::load_from_file(file.path()).unwrap();
        assert_eq!(config.workspace.max_steps, 10);
        assert_eq!(config.roles.tester.model, "gpt-4o-mini");
        assert_eq!(config.llm.provider, LlmProvider::Openai);
    }

    #[test]
    fn rejects_empty_model() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(
            br#"workspace:
  kata_file: kata.md
  plan_dir: .tdd/plan
  log_dir: .tdd/logs
roles:
  tester:
    model: ""
  implementor:
    model: gpt
  refactorer:
    model: gpt
llm:
  provider: openai
  base_url: https://api.example.com
  api_key_env: API_KEY
ci:
  fmt: ["cargo", "fmt"]
  check: ["cargo", "clippy"]
  test: ["cargo", "test"]
commit_author:
  name: Example
  email: example@example.com
"#,
        )
        .unwrap();

        let err = TddConfig::load_from_file(file.path()).unwrap_err();
        matches!(err, ConfigError::InvalidField { .. });
    }

    #[test]
    fn loads_github_copilot_provider() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(
            r#"workspace:
  kata_file: kata.md
  plan_dir: .tdd/plan
  log_dir: .tdd/logs
roles:
  tester:
    model: gpt-4o
  implementor:
    model: gpt-4o
  refactorer:
    model: gpt-4o
llm:
  provider: github_copilot
  base_url: https://api.githubcopilot.com/v1
  api_key_env: GITHUB_COPILOT_TOKEN
  api_version: 2023-12-01
ci:
  fmt: ["cargo", "fmt"]
  check: ["cargo", "clippy"]
  test: ["cargo", "test"]
commit_author:
  name: Bot
  email: bot@example.com
"#
            .as_bytes(),
        )
        .unwrap();

        let config = TddConfig::load_from_file(file.path()).unwrap();
        assert_eq!(config.llm.provider, LlmProvider::GithubCopilot);
        assert_eq!(config.llm.api_version, Some("2023-12-01".to_string()));
        assert_eq!(config.llm.base_url, "https://api.githubcopilot.com/v1");
    }

    #[test]
    fn defaults_to_openai_when_provider_omitted() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(
            r#"workspace:
  kata_file: kata.md
  plan_dir: .tdd/plan
  log_dir: .tdd/logs
roles:
  tester:
    model: gpt-4o
  implementor:
    model: gpt-4o
  refactorer:
    model: gpt-4o
llm:
  base_url: https://api.openai.com/v1
  api_key_env: OPENAI_API_KEY
ci:
  fmt: ["cargo", "fmt"]
  check: ["cargo", "clippy"]
  test: ["cargo", "test"]
commit_author:
  name: Bot
  email: bot@example.com
"#
            .as_bytes(),
        )
        .unwrap();

        let config = TddConfig::load_from_file(file.path()).unwrap();
        assert_eq!(config.llm.provider, LlmProvider::Openai);
    }
}
