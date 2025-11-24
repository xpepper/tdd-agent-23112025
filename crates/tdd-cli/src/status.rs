use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use tdd_core::{
    config::TddConfig,
    logging::{latest_log_entry, StepLogEntry},
    step::Role,
};
use tdd_exec::vcs::{GitVcs, Vcs};

use crate::executor::{absolutize_path, resolve_root};

#[derive(Debug, Clone)]
pub struct StatusReport {
    pub next_role: Role,
    pub next_step: u32,
    pub max_steps: u32,
    pub repo_clean: bool,
    pub last_commit_id: Option<String>,
    pub last_commit_message: Option<String>,
    pub last_log: Option<StepLogEntry>,
}

impl StatusReport {
    pub fn format_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(format!(
            "Next role: {} (step {} of {})",
            self.next_role.as_str(),
            self.next_step,
            self.max_steps
        ));
        lines.push(format!(
            "Workspace clean: {}",
            if self.repo_clean { "yes" } else { "no" }
        ));
        match (&self.last_commit_message, &self.last_commit_id) {
            (Some(message), Some(id)) => {
                lines.push(format!("Last commit: {message} ({id})"));
            }
            (Some(message), None) => lines.push(format!("Last commit: {message}")),
            (None, Some(id)) => lines.push(format!("Last commit id: {id}")),
            _ => lines.push("Last commit: none".into()),
        }
        if let Some(log) = &self.last_log {
            lines.push(format!(
                "Last step: {} #{}, plan {}",
                log.role.as_str(),
                log.step_index,
                log.plan_path
            ));
            lines.push(format!(
                "CI exit codes: fmt={}, check={}, test={}",
                log.runner.fmt.code, log.runner.check.code, log.runner.test.code
            ));
        } else {
            lines.push("No step logs found.".into());
        }
        lines
    }
}

pub fn gather_status(config_path: impl AsRef<Path>) -> Result<StatusReport> {
    let config_path = absolutize_path(config_path.as_ref())?;
    let config = TddConfig::load_from_file(&config_path)?;
    let root = resolve_root(&config_path)?;
    build_report(config, root)
}

fn build_report(config: TddConfig, root: PathBuf) -> Result<StatusReport> {
    let vcs = GitVcs::open_or_init(&root).context("failed to open git repository")?;
    let repo_state = vcs.state().context("failed to read git state")?;
    let last_log = latest_log_entry(&root, &config.workspace.log_dir)
        .context("failed to inspect log directory")?;
    let (next_role, next_step) = match &last_log {
        Some(entry) => (entry.role.next(), entry.step_index + 1),
        None => (Role::Tester, 1),
    };

    let (last_commit_id, last_commit_message) = if let Some(log) = &last_log {
        (
            Some(log.commit_id.clone()),
            Some(log.commit_message.clone()),
        )
    } else {
        (
            repo_state.head_commit.clone(),
            repo_state.last_commit_message.clone(),
        )
    };

    Ok(StatusReport {
        next_role,
        next_step,
        max_steps: config.workspace.max_steps,
        repo_clean: repo_state.is_clean,
        last_commit_id,
        last_commit_message,
        last_log,
    })
}
