use std::{
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
};

use anyhow::{bail, Context, Result};
use tokio::runtime::Runtime;

use tdd_agents::{implementor::ImplementorAgent, refactorer::RefactorerAgent, tester::TesterAgent};
use tdd_core::{
    commit_policy::CommitPolicy,
    config::TddConfig,
    orchestrator::{Agent, DefaultOrchestrator, Orchestrator},
    step::Role,
};
use tdd_exec::{
    runner::{CommandRunner, Runner, RunnerCommands, RunnerError},
    vcs::{CommitSignature, GitVcs, RepoState, Vcs, VcsError},
};
use tdd_llm::{
    client::{create_client, LlmClient},
    config::LlmClientSettings,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionSummary {
    pub requested: u32,
    pub executed: u32,
}

pub fn run_steps(config_path: impl AsRef<Path>, requested_steps: u32) -> Result<ExecutionSummary> {
    let config_path = absolutize_path(config_path.as_ref())?;
    let config = TddConfig::load_from_file(&config_path)?;
    let settings = LlmClientSettings::from_core_config(&config);
    let llm = create_client(
        config.llm.provider,
        settings,
        config.llm.effective_api_version(),
    )
    .context("failed to initialize LLM client")?;
    execute_steps(&config_path, config, requested_steps, llm)
}

pub fn run_steps_with_client(
    config_path: impl AsRef<Path>,
    requested_steps: u32,
    llm: Arc<dyn LlmClient>,
) -> Result<ExecutionSummary> {
    let config_path = absolutize_path(config_path.as_ref())?;
    let config = TddConfig::load_from_file(&config_path)?;
    execute_steps(&config_path, config, requested_steps, llm)
}

fn execute_steps(
    config_path: &Path,
    config: TddConfig,
    requested_steps: u32,
    llm: Arc<dyn LlmClient>,
) -> Result<ExecutionSummary> {
    if requested_steps == 0 {
        bail!("requested steps must be at least 1");
    }

    let root = resolve_root(config_path)?;
    ensure_workspace_dirs(&root, &config)?;

    let git_vcs = GitVcs::open_or_init(&root).context("failed to open git repository")?;
    git_vcs
        .ensure_initialized()
        .context("failed to initialize git repository")?;
    let vcs = SharedGitVcs::new(git_vcs);

    let runner_commands = RunnerCommands::from_raw(
        config.ci.fmt.clone(),
        config.ci.check.clone(),
        config.ci.test.clone(),
    )
    .context("invalid CI command configuration")?;
    let runner = CommandRunner::new(&root, runner_commands);

    let (last_role, starting_step) = detect_plan_progress(&root, &config.workspace.plan_dir)
        .context("failed to inspect plan history")?;
    let completed_steps = starting_step.saturating_sub(1);
    let remaining = config.workspace.max_steps.saturating_sub(completed_steps);
    if remaining == 0 {
        bail!(
            "workspace already reached configured max_steps ({}).",
            config.workspace.max_steps
        );
    }
    let steps_to_run = requested_steps.min(remaining);
    if steps_to_run == 0 {
        bail!("no steps available within configured limits");
    }

    // Baseline test check: if this is an existing project with tests, ensure they pass
    if starting_step == 1 && has_existing_tests(&root)? {
        println!("🔍 Detected existing tests - running baseline check...");
        match runner.test() {
            Ok(_) => println!("✓ Baseline tests pass"),
            Err(RunnerError::CommandFailed {
                code,
                stdout,
                stderr,
                ..
            }) => {
                bail!(
                    "Baseline test check failed. Existing tests must pass before autonomous TDD steps can run.\n\
                     Fix the failing tests manually, then try again.\n\n\
                     Exit code: {code}\n\
                     Stdout:\n{stdout}\n\
                     Stderr:\n{stderr}",
                );
            }
            Err(err) => {
                return Err(err).context("failed to run baseline cargo test");
            }
        }
    }

    let agents = build_agents(&root, llm);
    let mut orchestrator = DefaultOrchestrator::new(
        &root,
        config,
        vcs,
        runner,
        agents,
        last_role,
        starting_step,
        CommitPolicy,
    )?;

    let runtime = Runtime::new().context("failed to initialize tokio runtime")?;
    for _ in 0..steps_to_run {
        let role = orchestrator.current_role();
        runtime
            .block_on(orchestrator.next())
            .with_context(|| format!("failed to execute {} step", role.as_str()))?;
    }

    Ok(ExecutionSummary {
        requested: requested_steps,
        executed: steps_to_run,
    })
}

pub(crate) fn absolutize_path(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    let cwd = std::env::current_dir().context("failed to determine current directory")?;
    Ok(cwd.join(path))
}

pub(crate) fn resolve_root(config_path: &Path) -> Result<PathBuf> {
    let root = config_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    if root.is_absolute() {
        Ok(root)
    } else {
        let cwd = std::env::current_dir().context("failed to determine current directory")?;
        Ok(cwd.join(root))
    }
}

pub(crate) fn ensure_workspace_dirs(root: &Path, config: &TddConfig) -> Result<()> {
    for dir in [&config.workspace.plan_dir, &config.workspace.log_dir] {
        let full_path = root.join(dir);
        fs::create_dir_all(&full_path)
            .with_context(|| format!("failed to create workspace directory {full_path:?}"))?;
    }
    Ok(())
}

fn detect_plan_progress(root: &Path, plan_dir: &Path) -> Result<(Option<Role>, u32)> {
    let dir = root.join(plan_dir);
    let reader = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok((None, 1)),
        Err(err) => {
            return Err(err).with_context(|| format!("failed to read plan directory {dir:?}"))
        }
    };

    let mut max_step = 0;
    let mut last_role = None;

    for entry in reader {
        let entry = entry?;
        if let Some((step, role)) = parse_plan_filename(&entry.file_name()) {
            if step > max_step {
                max_step = step;
                last_role = Some(role);
            }
        }
    }

    if max_step == 0 {
        Ok((last_role, 1))
    } else {
        Ok((last_role, max_step + 1))
    }
}

fn parse_plan_filename(name: &OsStr) -> Option<(u32, Role)> {
    let name = name.to_str()?;
    if !name.starts_with("step-") || !name.ends_with(".md") {
        return None;
    }
    let inner = &name[5..name.len() - 3];
    let mut parts = inner.splitn(2, '-');
    let step = parts.next()?.parse().ok()?;
    let role_str = parts.next()?;
    let role = match role_str {
        "tester" => Role::Tester,
        "implementor" => Role::Implementor,
        "refactorer" => Role::Refactorer,
        _ => return None,
    };
    Some((step, role))
}

fn has_existing_tests(root: &Path) -> Result<bool> {
    // Check for common test file patterns in Rust projects
    let tests_dir = root.join("tests");
    let src_dir = root.join("src");

    // Check if tests/ directory exists and has .rs files
    if tests_dir.exists() && tests_dir.is_dir() {
        let has_test_files = fs::read_dir(&tests_dir)?
            .filter_map(|entry| entry.ok())
            .any(|entry| {
                entry
                    .path()
                    .extension()
                    .map(|ext| ext == "rs")
                    .unwrap_or(false)
            });
        if has_test_files {
            return Ok(true);
        }
    }

    // Check if src/ directory has files with #[cfg(test)] or #[test] attributes
    if src_dir.exists() && src_dir.is_dir() {
        for entry in fs::read_dir(&src_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|ext| ext == "rs").unwrap_or(false) {
                if let Ok(content) = fs::read_to_string(&path) {
                    if content.contains("#[test]") || content.contains("#[cfg(test)]") {
                        return Ok(true);
                    }
                }
            }
        }
    }

    Ok(false)
}

fn build_agents(root: &Path, llm: Arc<dyn LlmClient>) -> Vec<Arc<dyn Agent>> {
    vec![
        Arc::new(TesterAgent::new(llm.clone(), root)) as Arc<dyn Agent>,
        Arc::new(ImplementorAgent::new(llm.clone(), root)) as Arc<dyn Agent>,
        Arc::new(RefactorerAgent::new(llm, root)) as Arc<dyn Agent>,
    ]
}

#[derive(Clone)]
struct SharedGitVcs {
    inner: Arc<Mutex<GitVcs>>,
}

impl SharedGitVcs {
    fn new(inner: GitVcs) -> Self {
        Self {
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    fn guard(&self) -> MutexGuard<'_, GitVcs> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl Vcs for SharedGitVcs {
    fn ensure_initialized(&self) -> Result<(), VcsError> {
        self.guard().ensure_initialized()
    }

    fn state(&self) -> Result<RepoState, VcsError> {
        self.guard().state()
    }

    fn stage_all(&self) -> Result<(), VcsError> {
        self.guard().stage_all()
    }

    fn commit(&self, message: &str, author: &CommitSignature) -> Result<String, VcsError> {
        self.guard().commit(message, author)
    }
}
