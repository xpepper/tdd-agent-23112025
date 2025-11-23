//! Orchestrator contracts, helpers, and default implementation.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{anyhow, ensure, Context, Result};
use async_trait::async_trait;
use thiserror::Error;

use crate::commit_policy::{CommitMessageInputs, CommitPolicy, RunnerOutcomeSummary};
use crate::config::TddConfig;
use crate::step::{Role, StepContext, StepContextBuilder, StepResult};
use tdd_exec::runner::{Runner, RunnerError};
use tdd_exec::vcs::{CommitSignature, Vcs};

/// Common behavior required from every LLM-backed agent.
#[async_trait]
pub trait Agent: Send + Sync {
    fn role(&self) -> Role;
    async fn plan(&self, ctx: &StepContext) -> Result<String>;
    async fn edit(&self, ctx: &StepContext) -> Result<StepResult>;
}

/// High-level control loop that coordinates agents and git operations.
#[async_trait]
pub trait Orchestrator {
    fn current_role(&self) -> Role;
    async fn next(&mut self) -> Result<()>;
}

/// Persist agent plans to `.tdd/plan/` for traceability.
#[derive(Debug, Clone)]
struct PlanWriter {
    root: PathBuf,
    plan_dir: PathBuf,
}

impl PlanWriter {
    fn new(root: impl AsRef<Path>, plan_dir: impl Into<PathBuf>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            plan_dir: plan_dir.into(),
        }
    }

    fn write(
        &self,
        step_index: u32,
        role: Role,
        content: &str,
    ) -> Result<PathBuf, PlanWriterError> {
        let dir = self.root.join(&self.plan_dir);
        fs::create_dir_all(&dir).map_err(|source| PlanWriterError::CreateDir {
            path: dir.clone(),
            source,
        })?;

        let file_name = format!("step-{step_index:03}-{}.md", role.as_str());
        let path = dir.join(file_name);
        let body = format!(
            "# Plan for step {step_index} ({role})\n\n{plan}\n",
            role = role.as_str(),
            plan = content.trim()
        );
        fs::write(&path, body).map_err(|source| PlanWriterError::WriteFile {
            path: path.clone(),
            source,
        })?;
        Ok(path)
    }
}

#[derive(Debug, Error)]
enum PlanWriterError {
    #[error("failed to create plan directory {path}: {source}")]
    CreateDir {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to write plan file {path}: {source}")]
    WriteFile {
        path: PathBuf,
        source: std::io::Error,
    },
}

/// Concrete orchestrator used by the CLI and tests.
pub struct DefaultOrchestrator<V, R> {
    root: PathBuf,
    config: TddConfig,
    vcs: V,
    runner: R,
    agents: HashMap<Role, Arc<dyn Agent>>,
    plan_writer: PlanWriter,
    cycle: RoleCycle,
    step_index: u32,
    commit_author: CommitSignature,
    commit_policy: CommitPolicy,
}

impl<V, R> DefaultOrchestrator<V, R>
where
    V: Vcs,
    R: Runner,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        root: impl AsRef<Path>,
        config: TddConfig,
        vcs: V,
        runner: R,
        agents: Vec<Arc<dyn Agent>>,
        last_role: Option<Role>,
        starting_step: u32,
        commit_policy: CommitPolicy,
    ) -> Result<Self> {
        let root_buf = root.as_ref().to_path_buf();
        let repo_state = vcs.state()?;
        let plan_writer = PlanWriter::new(&root_buf, config.workspace.plan_dir.clone());
        let agents_map = Self::index_agents(agents);
        Self::ensure_all_roles_present(&agents_map)?;
        let cycle = RoleCycle::from_history(last_role, repo_state.head_commit.is_none());
        let commit_author = CommitSignature::new(
            config.commit_author.name.clone(),
            config.commit_author.email.clone(),
        );

        Ok(Self {
            root: root_buf,
            config,
            vcs,
            runner,
            agents: agents_map,
            plan_writer,
            cycle,
            step_index: starting_step,
            commit_author,
            commit_policy,
        })
    }

    fn index_agents(agents: Vec<Arc<dyn Agent>>) -> HashMap<Role, Arc<dyn Agent>> {
        let mut map = HashMap::new();
        for agent in agents {
            map.insert(agent.role(), agent);
        }
        map
    }

    fn ensure_all_roles_present(map: &HashMap<Role, Arc<dyn Agent>>) -> Result<()> {
        for role in [Role::Tester, Role::Implementor, Role::Refactorer] {
            ensure!(
                map.contains_key(&role),
                "missing agent for role {}",
                role.as_str()
            );
        }
        Ok(())
    }

    fn max_attempts_for(&self, role: Role) -> u32 {
        if matches!(role, Role::Tester) {
            1
        } else {
            self.config.workspace.max_attempts_per_agent.max(1)
        }
    }

    fn build_context(&self, role: Role) -> Result<StepContext> {
        let builder = StepContextBuilder::new(
            self.root.as_path(),
            self.config.workspace.kata_file.clone(),
            &self.vcs,
        );
        builder
            .build(role, self.step_index)
            .context("failed to build step context")
    }

    fn run_ci_checks(&self) -> Result<RunnerOutcomeSummary, RunnerError> {
        let fmt = self.runner.fmt()?;
        let check = self.runner.check()?;
        let test = self.runner.test()?;
        Ok(RunnerOutcomeSummary::new(fmt, check, test))
    }
}

#[async_trait]
impl<V, R> Orchestrator for DefaultOrchestrator<V, R>
where
    V: Vcs + Send + Sync,
    R: Runner + Send + Sync,
{
    fn current_role(&self) -> Role {
        self.cycle.current()
    }

    async fn next(&mut self) -> Result<()> {
        let role = self.cycle.current();
        let ctx = self.build_context(role)?;
        let agent = self
            .agents
            .get(&role)
            .cloned()
            .ok_or_else(|| anyhow!("no agent registered for role {}", role.as_str()))?;

        let plan = agent
            .plan(&ctx)
            .await
            .with_context(|| format!("plan failed for role {}", role.as_str()))?;
        let plan_path = self
            .plan_writer
            .write(self.step_index, role, &plan)
            .context("failed to persist plan")?;

        let max_attempts = self.max_attempts_for(role);
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            let step_result = agent
                .edit(&ctx)
                .await
                .with_context(|| format!("edit failed for role {}", role.as_str()))?;

            match self.run_ci_checks() {
                Ok(ci) => {
                    self.vcs.stage_all().context("failed to stage changes")?;
                    let commit_message = self.commit_policy.format(CommitMessageInputs {
                        role,
                        step_index: self.step_index,
                        kata_description: &ctx.kata_description,
                        agent_commit_message: &step_result.commit_message,
                        notes: &step_result.notes,
                        files_changed: &step_result.files_changed,
                        plan_path: &plan_path,
                        runner_outcomes: &ci,
                    });
                    self.vcs
                        .commit(&commit_message, &self.commit_author)
                        .context("failed to commit")?;
                    self.cycle.advance();
                    self.step_index += 1;
                    return Ok(());
                }
                Err(err) => {
                    if matches!(role, Role::Tester) || attempt >= max_attempts {
                        return Err(err.into());
                    }
                }
            }
        }
    }
}

/// Utility that encodes the Tester → Implementor → Refactorer rotation rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoleCycle {
    current: Role,
}

impl RoleCycle {
    /// Start from a specific role.
    pub const fn new(initial: Role) -> Self {
        Self { current: initial }
    }

    /// Determine the starting role based on repository history.
    ///
    /// When the repo is empty we always start with Tester. Otherwise we resume with the
    /// next role after the last successful one (defaults back to Tester when unknown).
    pub const fn from_history(last_role: Option<Role>, repo_is_empty: bool) -> Self {
        if repo_is_empty {
            return Self::new(Role::Tester);
        }

        let initial = match last_role {
            Some(role) => role.next(),
            None => Role::Tester,
        };
        Self::new(initial)
    }

    /// Return the current role in the cycle.
    pub const fn current(&self) -> Role {
        self.current
    }

    /// Advance to the next role and return it.
    pub fn advance(&mut self) -> Role {
        self.current = self.current.next();
        self.current
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashSet,
        sync::{Arc, Mutex},
    };

    use tempfile::tempdir;

    use super::*;
    use crate::step::StepResult;
    use tdd_exec::runner::{RunOutcome, RunnerError};
    use tdd_exec::vcs::{RepoState, Vcs, VcsError};

    #[test]
    fn role_cycle_advances_in_order() {
        let mut cycle = RoleCycle::new(Role::Tester);
        assert_eq!(cycle.current(), Role::Tester);
        assert_eq!(cycle.advance(), Role::Implementor);
        assert_eq!(cycle.advance(), Role::Refactorer);
        assert_eq!(cycle.advance(), Role::Tester);
    }

    #[test]
    fn role_cycle_respects_empty_repo_rule() {
        let cycle = RoleCycle::from_history(Some(Role::Refactorer), true);
        assert_eq!(cycle.current(), Role::Tester);
    }

    #[test]
    fn role_cycle_resumes_after_last_role() {
        let cycle = RoleCycle::from_history(Some(Role::Tester), false);
        assert_eq!(cycle.current(), Role::Implementor);

        let cycle = RoleCycle::from_history(None, false);
        assert_eq!(cycle.current(), Role::Tester);
    }

    #[test]
    fn plan_writer_persists_files() {
        let temp = tempdir().unwrap();
        let writer = PlanWriter::new(temp.path(), ".tdd/plan");
        let path = writer.write(3, Role::Tester, "add sample test").unwrap();
        let contents = fs::read_to_string(&path).unwrap();
        assert_eq!(
            path.file_name().unwrap().to_string_lossy(),
            "step-003-tester.md"
        );
        assert!(contents.contains("Plan for step 3"));
        assert!(contents.contains("add sample test"));
    }

    #[tokio::test]
    async fn orchestrator_writes_plan_and_commits() {
        let temp = tempdir().unwrap();
        fs::write(temp.path().join("kata.md"), "Solve kata").unwrap();

        let config = crate::config::TddConfig {
            workspace: crate::config::WorkspaceConfig {
                kata_file: "kata.md".into(),
                plan_dir: ".tdd/plan".into(),
                log_dir: ".tdd/logs".into(),
                max_steps: 10,
                max_attempts_per_agent: 2,
            },
            roles: crate::config::RoleConfigs {
                tester: crate::config::RoleConfig {
                    model: "gpt".into(),
                    temperature: 0.1,
                },
                implementor: crate::config::RoleConfig {
                    model: "gpt".into(),
                    temperature: 0.1,
                },
                refactorer: crate::config::RoleConfig {
                    model: "gpt".into(),
                    temperature: 0.1,
                },
            },
            llm: crate::config::LlmConfig {
                base_url: "http://localhost".into(),
                api_key_env: "KEY".into(),
            },
            ci: crate::config::CiConfig {
                fmt: vec!["cargo".into(), "fmt".into()],
                check: vec!["cargo".into(), "clippy".into()],
                test: vec!["cargo".into(), "test".into()],
            },
            commit_author: crate::config::CommitAuthor {
                name: "Bot".into(),
                email: "bot@example.com".into(),
            },
        };

        let vcs = FakeVcs::new();
        let runner = FakeRunner::default();
        let agents: Vec<Arc<dyn Agent>> = vec![
            Arc::new(FakeAgent::new(Role::Tester)),
            Arc::new(FakeAgent::new(Role::Implementor)),
            Arc::new(FakeAgent::new(Role::Refactorer)),
        ];
        let mut orchestrator = DefaultOrchestrator::new(
            temp.path(),
            config,
            vcs.clone(),
            runner,
            agents,
            None,
            1,
            CommitPolicy::default(),
        )
        .unwrap();

        orchestrator.next().await.unwrap();

        let plan_files: HashSet<_> = fs::read_dir(temp.path().join(".tdd/plan"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(plan_files.len(), 1);
        assert_eq!(orchestrator.current_role(), Role::Implementor);
        assert_eq!(orchestrator.step_index, 2);
        assert_eq!(vcs.commits.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn retries_for_implementor_on_runner_failure() {
        let temp = tempdir().unwrap();
        fs::write(temp.path().join("kata.md"), "Solve kata").unwrap();
        let config = minimal_config();
        let mut vcs = FakeVcs::new();
        vcs.state.head_commit = Some("abc123".into());
        let runner = FakeRunner {
            fail_stage: Some("test".into()),
            failures_before_success: Mutex::new(1),
        };
        let retry_agent = Arc::new(RetryAgent::new(Role::Implementor));
        let agents: Vec<Arc<dyn Agent>> = vec![
            Arc::new(FakeAgent::new(Role::Tester)),
            retry_agent.clone(),
            Arc::new(FakeAgent::new(Role::Refactorer)),
        ];
        let mut orchestrator = DefaultOrchestrator::new(
            temp.path(),
            config,
            vcs,
            runner,
            agents,
            Some(Role::Tester),
            5,
            CommitPolicy::default(),
        )
        .unwrap();

        assert_eq!(orchestrator.max_attempts_for(Role::Implementor), 2);
        orchestrator.next().await.unwrap();
        assert_eq!(*retry_agent.edit_calls.lock().unwrap(), 2);
    }

    fn minimal_config() -> TddConfig {
        TddConfig {
            workspace: crate::config::WorkspaceConfig {
                kata_file: "kata.md".into(),
                plan_dir: ".tdd/plan".into(),
                log_dir: ".tdd/logs".into(),
                max_steps: 10,
                max_attempts_per_agent: 2,
            },
            roles: crate::config::RoleConfigs {
                tester: crate::config::RoleConfig {
                    model: "gpt".into(),
                    temperature: 0.1,
                },
                implementor: crate::config::RoleConfig {
                    model: "gpt".into(),
                    temperature: 0.1,
                },
                refactorer: crate::config::RoleConfig {
                    model: "gpt".into(),
                    temperature: 0.1,
                },
            },
            llm: crate::config::LlmConfig {
                base_url: "http://localhost".into(),
                api_key_env: "KEY".into(),
            },
            ci: crate::config::CiConfig {
                fmt: vec!["cargo".into(), "fmt".into()],
                check: vec!["cargo".into(), "clippy".into()],
                test: vec!["cargo".into(), "test".into()],
            },
            commit_author: crate::config::CommitAuthor {
                name: "Bot".into(),
                email: "bot@example.com".into(),
            },
        }
    }

    #[derive(Clone)]
    struct FakeVcs {
        commits: Arc<Mutex<Vec<String>>>,
        state: RepoState,
    }

    impl FakeVcs {
        fn new() -> Self {
            Self {
                commits: Arc::new(Mutex::new(Vec::new())),
                state: RepoState {
                    head_commit: None,
                    last_commit_message: Some("initial".into()),
                    last_commit_diff: Some("diff".into()),
                    is_clean: true,
                },
            }
        }
    }

    impl Vcs for FakeVcs {
        fn ensure_initialized(&self) -> Result<(), VcsError> {
            Ok(())
        }

        fn state(&self) -> Result<RepoState, VcsError> {
            Ok(self.state.clone())
        }

        fn stage_all(&self) -> Result<(), VcsError> {
            Ok(())
        }

        fn commit(&self, message: &str, _author: &CommitSignature) -> Result<String, VcsError> {
            let mut guard = self.commits.lock().unwrap();
            guard.push(message.to_string());
            Ok(format!("commit-{}", guard.len()))
        }
    }

    #[derive(Default)]
    struct FakeRunner {
        fail_stage: Option<String>,
        failures_before_success: Mutex<u32>,
    }

    impl Runner for FakeRunner {
        fn fmt(&self) -> Result<RunOutcome, RunnerError> {
            self.maybe_fail("fmt")
        }

        fn check(&self) -> Result<RunOutcome, RunnerError> {
            self.maybe_fail("check")
        }

        fn test(&self) -> Result<RunOutcome, RunnerError> {
            self.maybe_fail("test")
        }
    }

    impl FakeRunner {
        fn maybe_fail(&self, stage: &str) -> Result<RunOutcome, RunnerError> {
            if self.fail_stage.as_deref() == Some(stage) {
                let mut remaining = self.failures_before_success.lock().unwrap();
                if *remaining > 0 {
                    *remaining -= 1;
                    return Err(RunnerError::CommandFailed {
                        program: stage.into(),
                        code: 1,
                        stderr: "boom".into(),
                    });
                }
            }
            Ok(RunOutcome {
                code: 0,
                stdout: format!("{stage} ok"),
                stderr: String::new(),
            })
        }
    }

    struct FakeAgent {
        role: Role,
    }

    impl FakeAgent {
        fn new(role: Role) -> Self {
            Self { role }
        }
    }

    #[async_trait]
    impl Agent for FakeAgent {
        fn role(&self) -> Role {
            self.role
        }

        async fn plan(&self, _ctx: &StepContext) -> Result<String> {
            Ok("plan".into())
        }

        async fn edit(&self, _ctx: &StepContext) -> Result<StepResult> {
            Ok(StepResult {
                files_changed: vec!["src/lib.rs".into()],
                commit_message: "test: add failing test".into(),
                notes: "notes".into(),
            })
        }
    }

    struct RetryAgent {
        role: Role,
        edit_calls: Arc<Mutex<u32>>,
    }

    impl RetryAgent {
        fn new(role: Role) -> Self {
            Self {
                role,
                edit_calls: Arc::new(Mutex::new(0)),
            }
        }
    }

    #[async_trait]
    impl Agent for RetryAgent {
        fn role(&self) -> Role {
            self.role
        }

        async fn plan(&self, _ctx: &StepContext) -> Result<String> {
            Ok("plan".into())
        }

        async fn edit(&self, _ctx: &StepContext) -> Result<StepResult> {
            let mut guard = self.edit_calls.lock().unwrap();
            *guard += 1;
            Ok(StepResult {
                files_changed: vec!["src/lib.rs".into()],
                commit_message: "feat: add behavior".into(),
                notes: "notes".into(),
            })
        }
    }
}
