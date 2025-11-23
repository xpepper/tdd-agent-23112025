//! Core types shared across the orchestrator and agents.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use tdd_exec::{
    fs::{list_workspace_files, FsError},
    vcs::{Vcs, VcsError},
};
use thiserror::Error;

/// Roles participating in the red–green–refactor loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    Tester,
    Implementor,
    Refactorer,
}

impl Role {
    /// Human-friendly identifier used in logs, plan files, and LLM prompts.
    pub const fn as_str(self) -> &'static str {
        match self {
            Role::Tester => "tester",
            Role::Implementor => "implementor",
            Role::Refactorer => "refactorer",
        }
    }

    /// Determine the next role in the fixed Tester → Implementor → Refactorer cycle.
    pub const fn next(self) -> Role {
        match self {
            Role::Tester => Role::Implementor,
            Role::Implementor => Role::Refactorer,
            Role::Refactorer => Role::Tester,
        }
    }
}

/// Snapshot of repository context shared with LLM agents before each step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepContext {
    pub role: Role,
    pub step_index: u32,
    pub kata_description: String,
    pub git_last_commit_msg: String,
    pub git_last_diff: String,
    pub repo_snapshot_paths: Vec<String>,
}

impl StepContext {
    /// Convenience constructor primarily used by tests.
    pub fn new(
        role: Role,
        step_index: u32,
        kata_description: impl Into<String>,
        git_last_commit_msg: impl Into<String>,
        git_last_diff: impl Into<String>,
        repo_snapshot_paths: Vec<String>,
    ) -> Self {
        Self {
            role,
            step_index,
            kata_description: kata_description.into(),
            git_last_commit_msg: git_last_commit_msg.into(),
            git_last_diff: git_last_diff.into(),
            repo_snapshot_paths,
        }
    }
}

/// Result of applying an agent plan to the filesystem.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepResult {
    pub files_changed: Vec<String>,
    pub commit_message: String,
    pub notes: String,
}

impl StepResult {
    /// Helper for situations where an agent produced no edits but the orchestrator must advance.
    pub fn empty(commit_message: impl Into<String>) -> Self {
        Self {
            files_changed: Vec::new(),
            commit_message: commit_message.into(),
            notes: String::new(),
        }
    }
}

/// Helper that assembles the [`StepContext`] using workspace metadata.
pub struct StepContextBuilder<'a, V: Vcs> {
    root: &'a Path,
    kata_file: PathBuf,
    vcs: &'a V,
}

impl<'a, V: Vcs> StepContextBuilder<'a, V> {
    /// Construct a new builder rooted at `root`, reading the kata description from `kata_file`.
    pub fn new(root: &'a Path, kata_file: impl Into<PathBuf>, vcs: &'a V) -> Self {
        Self {
            root,
            kata_file: kata_file.into(),
            vcs,
        }
    }

    /// Build the context for `role` and `step_index`.
    pub fn build(&self, role: Role, step_index: u32) -> Result<StepContext, StepContextError> {
        let repo_state = self.vcs.state().map_err(StepContextError::Vcs)?;
        let kata_path = self.root.join(&self.kata_file);
        let kata_description =
            fs::read_to_string(&kata_path).map_err(|source| StepContextError::ReadKata {
                path: kata_path.clone(),
                source,
            })?;

        let mut files = list_workspace_files(self.root).map_err(StepContextError::Fs)?;
        files.sort();
        let repo_snapshot_paths = files
            .into_iter()
            .map(|path| self.to_repo_path(&path))
            .collect();

        Ok(StepContext {
            role,
            step_index,
            kata_description,
            git_last_commit_msg: repo_state.last_commit_message.unwrap_or_default(),
            git_last_diff: repo_state.last_commit_diff.unwrap_or_default(),
            repo_snapshot_paths,
        })
    }

    fn to_repo_path(&self, path: &Path) -> String {
        path.strip_prefix(self.root)
            .ok()
            .map(|rel| rel.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|| path.to_string_lossy().into_owned())
    }
}

/// Errors emitted while gathering context for an agent.
#[derive(Debug, Error)]
pub enum StepContextError {
    #[error("failed to read kata description at {path}: {source}")]
    ReadKata {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(transparent)]
    Fs(#[from] FsError),
    #[error(transparent)]
    Vcs(#[from] VcsError),
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use tdd_exec::vcs::{CommitSignature, GitVcs};
    use tempfile::tempdir;

    #[test]
    fn role_cycle_is_fixed() {
        assert_eq!(Role::Tester.next(), Role::Implementor);
        assert_eq!(Role::Implementor.next(), Role::Refactorer);
        assert_eq!(Role::Refactorer.next(), Role::Tester);
    }

    #[test]
    fn builds_context_for_empty_repo() {
        let dir = tempdir().unwrap();
        let kata = dir.path().join("kata.md");
        fs::write(&kata, "Practice strings").unwrap();

        let vcs = GitVcs::open_or_init(dir.path()).unwrap();
        let builder = StepContextBuilder::new(dir.path(), "kata.md", &vcs);
        let ctx = builder.build(Role::Tester, 1).unwrap();

        assert_eq!(ctx.role, Role::Tester);
        assert_eq!(ctx.step_index, 1);
        assert!(ctx.kata_description.contains("Practice"));
        assert!(ctx.git_last_commit_msg.is_empty());
        assert!(ctx.git_last_diff.is_empty());
        assert!(ctx
            .repo_snapshot_paths
            .iter()
            .any(|p| p.contains("kata.md")));
    }

    #[test]
    fn builds_context_with_last_commit_details() {
        let dir = tempdir().unwrap();
        let kata = dir.path().join("kata.md");
        fs::write(&kata, "Steps").unwrap();
        let code = dir.path().join("src/lib.rs");
        fs::create_dir_all(code.parent().unwrap()).unwrap();
        fs::write(&code, "pub fn meaning_of_life() -> u32 { 42 }").unwrap();

        let vcs = GitVcs::open_or_init(dir.path()).unwrap();
        vcs.ensure_initialized().unwrap();
        vcs.stage_all().unwrap();
        vcs.commit(
            "feat: add meaning_of_life",
            &CommitSignature::new("Bot", "bot@example.com"),
        )
        .unwrap();

        let builder = StepContextBuilder::new(dir.path(), "kata.md", &vcs);
        let ctx = builder.build(Role::Implementor, 2).unwrap();

        assert_eq!(ctx.role, Role::Implementor);
        assert_eq!(ctx.step_index, 2);
        assert!(ctx.git_last_commit_msg.contains("meaning_of_life"));
        assert!(ctx.git_last_diff.contains("meaning_of_life"));
    }
}
