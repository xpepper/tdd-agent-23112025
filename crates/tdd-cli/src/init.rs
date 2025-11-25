use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use tdd_core::config::TddConfig;
use tdd_exec::vcs::{CommitSignature, GitVcs, Vcs};

const DEFAULT_KATA_CONTENT: &str = r#"# Kata Description

Write a clear description of the kata you want to practice here.
The autonomous TDD machine will use this as context for generating tests and implementations.

## Example

Implement a string calculator that:
- Takes a string of comma-separated numbers and returns their sum
- Returns 0 for an empty string
- Handles newlines between numbers
- Supports custom delimiters
"#;

const DEFAULT_TDD_YAML: &str = r#"# Autonomous Multi-Agent TDD Machine configuration
workspace:
  kata_file: "kata.md"
  plan_dir: ".tdd/plan"
  log_dir: ".tdd/logs"
  max_steps: 10
  max_attempts_per_agent: 2
roles:
  tester:
    model: "gpt-4o-mini"
    temperature: 0.1
  implementor:
    model: "gpt-4o-mini"
    temperature: 0.2
  refactorer:
    model: "gpt-4o-mini"
    temperature: 0.15
llm:
  # Provider selection: openai or github_copilot (defaults to openai)
  provider: "openai"
  base_url: "https://api.openai.com/v1"
  api_key_env: "OPENAI_API_KEY"
  # Optional: API version (required for GitHub Copilot, auto-defaults to 2023-12-01)
  # api_version: "2023-12-01"

# Example GitHub Copilot configuration (uncomment to use):
# llm:
#   provider: "github_copilot"
#   base_url: "https://api.githubcopilot.com/v1"
#   api_key_env: "GITHUB_COPILOT_TOKEN"
#   api_version: "2023-12-01"

ci:
  fmt: ["cargo", "fmt"]
  check: ["cargo", "clippy", "-D", "warnings"]
  test: ["cargo", "test"]
commit_author:
  name: "Autonomous TDD Machine"
  email: "tdd-machine@example.com"
"#;

#[derive(Debug)]
pub struct InitResult {
    pub workspace_exists: bool,
    pub config_created: bool,
    pub kata_created: bool,
    pub directories_created: bool,
    pub git_initialized: bool,
}

/// Initialize workspace for TDD machine, detecting existing Rust projects.
pub fn initialize_workspace(config_path: &str) -> Result<InitResult> {
    let root = PathBuf::from(".");
    let config_file = root.join(config_path);

    // Check if this is an existing Rust project
    let has_cargo_toml = root.join("Cargo.toml").exists();
    let has_src = root.join("src").exists();
    let workspace_exists = has_cargo_toml || has_src;

    if workspace_exists {
        println!("📦 Detected existing Rust project - will not overwrite project files");
    }

    // Initialize or open git repository
    let vcs = GitVcs::open_or_init(&root).context("Failed to initialize git repository")?;
    let repo_state = vcs.state().context("Failed to read repository state")?;
    let git_initialized = repo_state.head_commit.is_none();

    if git_initialized {
        println!("🔧 Initialized new git repository");
        vcs.ensure_initialized()
            .context("Failed to ensure git repository is initialized")?;
    } else {
        println!("✓ Using existing git repository");
    }

    // Create configuration file if it doesn't exist
    let config_created = if !config_file.exists() {
        fs::write(&config_file, DEFAULT_TDD_YAML)
            .with_context(|| format!("Failed to create config file: {}", config_file.display()))?;
        println!("✓ Created {}", config_path);
        true
    } else {
        // Validate existing config
        TddConfig::load_from_file(&config_file).with_context(|| {
            format!(
                "Failed to validate existing config: {}",
                config_file.display()
            )
        })?;
        println!("✓ Using existing {}", config_path);
        false
    };

    // Load config to get paths
    let config = TddConfig::load_from_file(&config_file)?;

    // Create kata description file if it doesn't exist
    let kata_path = root.join(&config.workspace.kata_file);
    let kata_created = if !kata_path.exists() {
        fs::write(&kata_path, DEFAULT_KATA_CONTENT)
            .with_context(|| format!("Failed to create kata file: {}", kata_path.display()))?;
        println!("✓ Created {}", config.workspace.kata_file.display());
        true
    } else {
        println!("✓ Using existing {}", config.workspace.kata_file.display());
        false
    };

    // Create .tdd directories
    let plan_dir = root.join(&config.workspace.plan_dir);
    let log_dir = root.join(&config.workspace.log_dir);
    let directories_created =
        create_directory_if_needed(&plan_dir)? | create_directory_if_needed(&log_dir)?;

    // Create initial commit if this is a new repository
    if git_initialized && (config_created || kata_created || directories_created) {
        vcs.stage_all().context("Failed to stage files")?;
        let author = CommitSignature::new(
            config.commit_author.name.clone(),
            config.commit_author.email.clone(),
        );
        vcs.commit("chore: initialize TDD workspace", &author)
            .context("Failed to create initial commit")?;
        println!("✓ Created initial commit");
    }

    Ok(InitResult {
        workspace_exists,
        config_created,
        kata_created,
        directories_created,
        git_initialized,
    })
}

fn create_directory_if_needed(path: &Path) -> Result<bool> {
    if !path.exists() {
        fs::create_dir_all(path)
            .with_context(|| format!("Failed to create directory: {}", path.display()))?;
        println!("✓ Created directory {}", path.display());
        Ok(true)
    } else {
        Ok(false)
    }
}
