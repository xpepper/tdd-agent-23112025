//! Per-step execution logging helpers.

use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::commit_policy::RunnerOutcomeSummary;
use crate::step::Role;
use tdd_exec::bootstrap::BootstrapResult;

/// Snapshot of a completed step persisted to `.tdd/logs`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StepLogEntry {
    pub step_index: u32,
    pub role: Role,
    pub plan_path: String,
    pub files_changed: Vec<String>,
    pub commit_id: String,
    pub commit_message: String,
    pub notes: String,
    pub provider: String,
    pub runner: RunnerLog,
}

impl StepLogEntry {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        step_index: u32,
        role: Role,
        plan_path: impl Into<String>,
        files_changed: Vec<String>,
        commit_id: impl Into<String>,
        commit_message: impl Into<String>,
        notes: impl Into<String>,
        provider: impl Into<String>,
        runner: RunnerLog,
    ) -> Self {
        Self {
            step_index,
            role,
            plan_path: plan_path.into(),
            files_changed,
            commit_id: commit_id.into(),
            commit_message: commit_message.into(),
            notes: notes.into(),
            provider: provider.into(),
            runner,
        }
    }
}

/// CI command outcome summary stored inside [`StepLogEntry`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunnerLog {
    pub fmt: CommandLog,
    pub check: CommandLog,
    pub test: CommandLog,
}

impl RunnerLog {
    pub fn from_summary(summary: &RunnerOutcomeSummary) -> Self {
        Self {
            fmt: CommandLog::from(&summary.fmt),
            check: CommandLog::from(&summary.check),
            test: CommandLog::from(&summary.test),
        }
    }
}

/// Single command execution result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandLog {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl From<&tdd_exec::runner::RunOutcome> for CommandLog {
    fn from(outcome: &tdd_exec::runner::RunOutcome) -> Self {
        Self {
            code: outcome.code,
            stdout: outcome.stdout.clone(),
            stderr: outcome.stderr.clone(),
        }
    }
}

/// Telemetry emitted after running a bootstrap/provisioning command.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BootstrapLogEntry {
    pub command: Vec<String>,
    pub working_dir: String,
    pub skipped: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub started_at_ms: u128,
    pub duration_ms: u128,
}

impl BootstrapLogEntry {
    /// Convert an execution result into a persistable log entry.
    pub fn from_result(result: &BootstrapResult) -> Self {
        Self {
            command: result.command.clone(),
            working_dir: result.working_dir.to_string_lossy().to_string(),
            skipped: result.skipped,
            skip_reason: result.skip_reason.clone(),
            exit_code: result.exit_code,
            stdout: result.stdout.clone(),
            stderr: result.stderr.clone(),
            started_at_ms: system_time_to_millis(result.started_at),
            duration_ms: result.duration.as_millis(),
        }
    }
}

/// Persists bootstrap telemetry under `.tdd/logs` and `.tdd/state`.
#[derive(Debug, Clone)]
pub struct BootstrapLogger {
    root: PathBuf,
    log_dir: PathBuf,
    state_file: PathBuf,
}

impl BootstrapLogger {
    pub fn new(
        root: impl AsRef<Path>,
        log_dir: impl Into<PathBuf>,
        state_file: impl Into<PathBuf>,
    ) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            log_dir: log_dir.into(),
            state_file: state_file.into(),
        }
    }

    pub fn persist(&self, entry: &BootstrapLogEntry) -> Result<BootstrapLogPaths, LogError> {
        let log_file = self.write_log(entry)?;
        let state_file = self.write_state(entry)?;
        Ok(BootstrapLogPaths {
            log_file,
            state_file,
        })
    }

    fn write_log(&self, entry: &BootstrapLogEntry) -> Result<PathBuf, LogError> {
        let dir = self.root.join(&self.log_dir);
        fs::create_dir_all(&dir).map_err(|source| LogError::CreateDir {
            path: dir.clone(),
            source,
        })?;
        let file_name = format!("bootstrap-{}.json", entry.started_at_ms);
        let path = dir.join(file_name);
        let json = serde_json::to_string_pretty(entry).map_err(LogError::Serialize)?;
        fs::write(&path, json).map_err(|source| LogError::WriteFile {
            path: path.clone(),
            source,
        })?;
        Ok(path)
    }

    fn write_state(&self, entry: &BootstrapLogEntry) -> Result<PathBuf, LogError> {
        let path = self.root.join(&self.state_file);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| LogError::CreateDir {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let json = serde_json::to_string_pretty(entry).map_err(LogError::Serialize)?;
        fs::write(&path, json).map_err(|source| LogError::WriteFile {
            path: path.clone(),
            source,
        })?;
        Ok(path)
    }
}

/// Paths returned after persisting bootstrap logs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapLogPaths {
    pub log_file: PathBuf,
    pub state_file: PathBuf,
}

/// Utility that persists [`StepLogEntry`] documents.
#[derive(Debug, Clone)]
pub struct StepLogger {
    root: PathBuf,
    log_dir: PathBuf,
}

impl StepLogger {
    pub fn new(root: impl AsRef<Path>, log_dir: impl Into<PathBuf>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            log_dir: log_dir.into(),
        }
    }

    pub fn write(&self, entry: &StepLogEntry) -> Result<PathBuf, LogError> {
        let dir = self.root.join(&self.log_dir);
        fs::create_dir_all(&dir).map_err(|source| LogError::CreateDir {
            path: dir.clone(),
            source,
        })?;
        let file_name = format!(
            "step-{step:03}-{}.json",
            entry.role.as_str(),
            step = entry.step_index
        );
        let path = dir.join(file_name);
        let json = serde_json::to_string_pretty(entry).map_err(LogError::Serialize)?;
        fs::write(&path, json).map_err(|source| LogError::WriteFile {
            path: path.clone(),
            source,
        })?;
        Ok(path)
    }
}

/// Read the most recent log entry from the workspace, if any.
pub fn latest_log_entry(
    root: impl AsRef<Path>,
    log_dir: impl AsRef<Path>,
) -> Result<Option<StepLogEntry>, LogError> {
    let dir = root.as_ref().join(log_dir);
    let reader = match fs::read_dir(&dir) {
        Ok(reader) => reader,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(LogError::ReadDir { path: dir, source }),
    };

    let mut latest: Option<(u32, Role, PathBuf)> = None;
    for entry in reader {
        let entry = entry.map_err(|source| LogError::ReadDirEntry { source })?;
        if let Some((step, role)) = parse_log_filename(&entry.file_name()) {
            let should_replace = latest
                .as_ref()
                .map(|(max_step, _, _)| step > *max_step)
                .unwrap_or(true);
            if should_replace {
                latest = Some((step, role, entry.path()));
            }
        }
    }

    let (_, _, path) = match latest {
        Some(value) => value,
        None => return Ok(None),
    };

    let contents = fs::read_to_string(&path).map_err(|source| LogError::ReadFile {
        path: path.clone(),
        source,
    })?;
    let entry =
        serde_json::from_str(&contents).map_err(|source| LogError::Parse { path, source })?;
    Ok(Some(entry))
}

fn parse_log_filename(name: &OsStr) -> Option<(u32, Role)> {
    let name = name.to_str()?;
    if !name.starts_with("step-") || !name.ends_with(".json") {
        return None;
    }
    let inner = &name[5..name.len() - 5];
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

#[derive(Debug, Error)]
pub enum LogError {
    #[error("failed to create log directory {path:?}: {source}")]
    CreateDir {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to write log file {path:?}: {source}")]
    WriteFile {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to serialize log entry: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("failed to read log directory {path:?}: {source}")]
    ReadDir {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to list log directory entry: {source}")]
    ReadDirEntry {
        #[source]
        source: io::Error,
    },
    #[error("failed to read log file {path:?}: {source}")]
    ReadFile {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to parse log file {path:?}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

fn system_time_to_millis(time: SystemTime) -> u128 {
    time.duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample_runner_log() -> RunnerLog {
        RunnerLog {
            fmt: CommandLog {
                code: 0,
                stdout: "fmt ok".into(),
                stderr: String::new(),
            },
            check: CommandLog {
                code: 0,
                stdout: "check ok".into(),
                stderr: String::new(),
            },
            test: CommandLog {
                code: 0,
                stdout: "tests ok".into(),
                stderr: String::new(),
            },
        }
    }

    #[test]
    fn write_persists_json_file() {
        let temp = tempdir().unwrap();
        let logger = StepLogger::new(temp.path(), ".tdd/logs");
        let entry = StepLogEntry::new(
            3,
            Role::Tester,
            ".tdd/plan/step-003-tester.md",
            vec!["tests/math.rs".into()],
            "abc123",
            "test: add failing case",
            "notes",
            "openai",
            sample_runner_log(),
        );
        let path = logger.write(&entry).unwrap();
        let contents = fs::read_to_string(&path).unwrap();
        assert!(contents.contains("\"step_index\": 3"));
        assert!(contents.contains("tests/math.rs"));
    }

    #[test]
    fn latest_entry_finds_highest_step() {
        let temp = tempdir().unwrap();
        let logger = StepLogger::new(temp.path(), ".tdd/logs");
        let base_entry = StepLogEntry::new(
            1,
            Role::Tester,
            "plan",
            vec![],
            "id",
            "msg",
            "notes",
            "openai",
            sample_runner_log(),
        );
        logger.write(&base_entry).unwrap();
        logger
            .write(&StepLogEntry {
                step_index: 2,
                role: Role::Implementor,
                ..base_entry.clone()
            })
            .unwrap();
        logger
            .write(&StepLogEntry {
                step_index: 3,
                role: Role::Refactorer,
                ..base_entry.clone()
            })
            .unwrap();

        let latest = latest_log_entry(temp.path(), ".tdd/logs").unwrap().unwrap();
        assert_eq!(latest.step_index, 3);
        assert_eq!(latest.role, Role::Refactorer);
    }

    #[test]
    fn bootstrap_logger_writes_log_and_state() {
        let temp = tempdir().unwrap();
        let logger = BootstrapLogger::new(temp.path(), ".tdd/logs", ".tdd/state/bootstrap.json");
        let entry = BootstrapLogEntry {
            command: vec!["sh".into(), "script.sh".into()],
            working_dir: ".".into(),
            skipped: false,
            skip_reason: None,
            exit_code: Some(0),
            stdout: "ok".into(),
            stderr: String::new(),
            started_at_ms: 123,
            duration_ms: 5,
        };

        let paths = logger.persist(&entry).unwrap();
        assert!(paths.log_file.exists());
        assert!(paths.state_file.exists());

        let log_contents = fs::read_to_string(paths.log_file).unwrap();
        assert!(log_contents.contains("script.sh"));
        let state_contents = fs::read_to_string(paths.state_file).unwrap();
        assert!(state_contents.contains("duration_ms"));
    }
}
