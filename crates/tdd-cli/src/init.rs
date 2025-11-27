use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::executor::{ensure_workspace_dirs, resolve_root};
use tdd_core::config::{BootstrapConfig, TddConfig};
use tdd_core::logging::{BootstrapLogEntry, BootstrapLogPaths, BootstrapLogger};
use tdd_exec::bootstrap::{BootstrapError, BootstrapResult, BootstrapRunner, BootstrapSpec};
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
# Optional provisioning hook (uncomment and configure to run bootstrap scripts)
#  bootstrap:
#    command: ["/bin/sh", "./scripts/bootstrap.sh"]
#    working_dir: "."
#    skip_files:
#      - ".tdd/state/bootstrap.skip"
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

const BOOTSTRAP_STATE_FILE: &str = ".tdd/state/bootstrap.json";

#[derive(Debug)]
pub struct InitResult {
    pub workspace_exists: bool,
    pub config_created: bool,
    pub kata_created: bool,
    pub directories_created: bool,
    pub git_initialized: bool,
    pub bootstrap: Option<BootstrapSummary>,
}

#[derive(Debug, Clone)]
pub struct BootstrapSummary {
    pub configured: bool,
    pub skipped: bool,
    pub log_file: Option<PathBuf>,
    pub state_file: Option<PathBuf>,
}

impl BootstrapSummary {
    fn not_configured() -> Self {
        Self {
            configured: false,
            skipped: true,
            log_file: None,
            state_file: None,
        }
    }

    fn from_paths(skipped: bool, paths: BootstrapLogPaths) -> Self {
        Self {
            configured: true,
            skipped,
            log_file: Some(paths.log_file),
            state_file: Some(paths.state_file),
        }
    }
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
    let state_dir = root.join(state_dir_path());
    let directories_created = create_directory_if_needed(&plan_dir)?
        | create_directory_if_needed(&log_dir)?
        | create_directory_if_needed(&state_dir)?;

    let bootstrap = maybe_run_bootstrap(&root, &config, false)?;

    // Create initial commit if this is a new repository
    if git_initialized
        && (config_created || kata_created || directories_created || bootstrap.is_some())
    {
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
        bootstrap,
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

fn state_dir_path() -> &'static str {
    ".tdd/state"
}

/// Execute the configured bootstrap command outside of `tdd-cli init`.
pub fn run_bootstrap(config_path: &str, force: bool) -> Result<BootstrapSummary> {
    let config_file = PathBuf::from(config_path);
    let config = TddConfig::load_from_file(&config_file)
        .with_context(|| format!("Failed to load config at {}", config_path))?;
    let root = resolve_root(&config_file)?;
    ensure_workspace_dirs(&root, &config)?;
    create_directory_if_needed(&root.join(state_dir_path()))?;

    if config.workspace.bootstrap.is_none() {
        println!("No bootstrap configuration found in {}", config_path);
        return Ok(BootstrapSummary::not_configured());
    }

    execute_bootstrap(&root, &config, force).with_context(|| "failed to run bootstrap command")
}

fn maybe_run_bootstrap(
    root: &Path,
    config: &TddConfig,
    force: bool,
) -> Result<Option<BootstrapSummary>> {
    if config.workspace.bootstrap.is_none() {
        return Ok(None);
    }

    ensure_workspace_dirs(root, config)?;
    create_directory_if_needed(&root.join(state_dir_path()))?;

    let summary = execute_bootstrap(root, config, force)?;
    Ok(Some(summary))
}

fn execute_bootstrap(root: &Path, config: &TddConfig, force: bool) -> Result<BootstrapSummary> {
    let bootstrap_cfg = match &config.workspace.bootstrap {
        Some(cfg) => cfg,
        None => return Ok(BootstrapSummary::not_configured()),
    };

    let spec = build_bootstrap_spec(root, bootstrap_cfg);
    let runner = BootstrapRunner::new(root, spec);
    let logger = BootstrapLogger::new(root, &config.workspace.log_dir, BOOTSTRAP_STATE_FILE);

    match runner.run(force) {
        Ok(result) => persist_bootstrap(&logger, result),
        Err(BootstrapError::CommandFailed { result, code }) => {
            let summary = persist_bootstrap(&logger, result)?;
            let log_path = summary
                .log_file
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "<unknown>".into());
            Err(anyhow!(
                "Bootstrap command failed with exit code {:?}. See {log_path}",
                code
            ))
        }
        Err(err) => Err(err.into()),
    }
}

fn persist_bootstrap(
    logger: &BootstrapLogger,
    result: BootstrapResult,
) -> Result<BootstrapSummary> {
    let entry = BootstrapLogEntry::from_result(&result);
    let paths = logger
        .persist(&entry)
        .context("failed to persist bootstrap telemetry")?;
    if result.skipped {
        println!(
            "⏭️  Skipped bootstrap: {}",
            entry.skip_reason.unwrap_or_default()
        );
    } else {
        println!("⚙️  Ran bootstrap command: {:?}", entry.command);
    }
    Ok(BootstrapSummary::from_paths(result.skipped, paths))
}

fn build_bootstrap_spec(root: &Path, config: &BootstrapConfig) -> BootstrapSpec {
    let working_dir = config
        .working_dir
        .as_ref()
        .map(|dir| absolutize(root, dir))
        .unwrap_or_else(|| root.to_path_buf());

    let skip_files = config
        .skip_files
        .iter()
        .map(|path| absolutize(root, path))
        .collect();

    BootstrapSpec::new(config.command.clone(), working_dir, skip_files)
}

fn absolutize(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}
