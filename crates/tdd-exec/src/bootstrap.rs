use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant, SystemTime};

use thiserror::Error;

/// Specification for running a bootstrap/provisioning script.
#[derive(Debug, Clone)]
pub struct BootstrapSpec {
    pub command: Vec<String>,
    pub working_dir: PathBuf,
    pub skip_files: Vec<PathBuf>,
}

impl BootstrapSpec {
    pub fn new(command: Vec<String>, working_dir: PathBuf, skip_files: Vec<PathBuf>) -> Self {
        Self {
            command,
            working_dir,
            skip_files,
        }
    }
}

/// Executes a configured bootstrap command, respecting skip markers.
pub struct BootstrapRunner {
    root: PathBuf,
    spec: BootstrapSpec,
}

impl BootstrapRunner {
    pub fn new(root: impl AsRef<Path>, spec: BootstrapSpec) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            spec,
        }
    }

    pub fn run(&self, force: bool) -> Result<BootstrapResult, BootstrapError> {
        if !force {
            if let Some(reason) = self.next_skip_reason()? {
                return Ok(BootstrapResult::skipped(
                    self.spec.command.clone(),
                    self.spec.working_dir.clone(),
                    reason,
                ));
            }
        }

        let started_at = SystemTime::now();
        let timer = Instant::now();
        let (program, args) = split_command(&self.spec.command)?;
        let mut command = Command::new(program);
        command.args(args);
        command.current_dir(&self.spec.working_dir);

        let output = command
            .output()
            .map_err(|source| BootstrapError::SpawnFailed {
                program: program.to_string(),
                source,
            })?;
        let duration = timer.elapsed();

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        let result = BootstrapResult {
            command: self.spec.command.clone(),
            working_dir: self.spec.working_dir.clone(),
            skipped: false,
            skip_reason: None,
            exit_code: output.status.code(),
            stdout,
            stderr,
            started_at,
            duration,
        };

        if output.status.success() {
            Ok(result)
        } else {
            let code = result.exit_code;
            Err(BootstrapError::CommandFailed { result, code })
        }
    }

    fn next_skip_reason(&self) -> Result<Option<String>, BootstrapError> {
        for marker in &self.spec.skip_files {
            let path = if marker.is_absolute() {
                marker.clone()
            } else {
                self.root.join(marker)
            };
            if path.exists() {
                return Ok(Some(format!("skip marker present at {}", path.display())));
            }
        }
        Ok(None)
    }
}

fn split_command(command: &[String]) -> Result<(&str, &[String]), BootstrapError> {
    if command.is_empty() {
        return Err(BootstrapError::InvalidCommand(
            "bootstrap command cannot be empty".into(),
        ));
    }
    let program = command[0].as_str();
    Ok((program, &command[1..]))
}

/// Completion details from running a bootstrap command.
#[derive(Debug, Clone)]
pub struct BootstrapResult {
    pub command: Vec<String>,
    pub working_dir: PathBuf,
    pub skipped: bool,
    pub skip_reason: Option<String>,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub started_at: SystemTime,
    pub duration: Duration,
}

impl BootstrapResult {
    fn skipped(command: Vec<String>, working_dir: PathBuf, reason: String) -> Self {
        Self {
            command,
            working_dir,
            skipped: true,
            skip_reason: Some(reason),
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            started_at: SystemTime::now(),
            duration: Duration::from_millis(0),
        }
    }
}

#[derive(Debug, Error)]
pub enum BootstrapError {
    #[error("{0}")]
    InvalidCommand(String),
    #[error("failed to spawn bootstrap command {program}: {source}")]
    SpawnFailed {
        program: String,
        #[source]
        source: std::io::Error,
    },
    #[error("bootstrap command failed with exit code {code:?}")]
    CommandFailed {
        result: BootstrapResult,
        code: Option<i32>,
    },
}
