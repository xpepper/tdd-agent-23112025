use std::{
    path::{Path, PathBuf},
    process::Command,
};

use thiserror::Error;

/// Result of executing a CI command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunOutcome {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Trait describing formatter/check/test runners used throughout the orchestrator.
pub trait Runner {
    fn fmt(&self) -> Result<RunOutcome, RunnerError>;
    fn check(&self) -> Result<RunOutcome, RunnerError>;
    fn test(&self) -> Result<RunOutcome, RunnerError>;
}

/// Concrete runner that shells out to configured commands.
pub struct CommandRunner {
    root: PathBuf,
    commands: RunnerCommands,
}

impl CommandRunner {
    pub fn new(root: impl AsRef<Path>, commands: RunnerCommands) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            commands,
        }
    }

    fn run_command(&self, spec: &CommandSpec) -> Result<RunOutcome, RunnerError> {
        let mut cmd = Command::new(&spec.program);
        cmd.args(&spec.args).current_dir(&self.root);
        let output = cmd.output().map_err(|source| RunnerError::SpawnFailed {
            program: spec.program.clone(),
            source,
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let outcome = RunOutcome {
            code: output.status.code().unwrap_or_default(),
            stdout,
            stderr,
        };

        if output.status.success() {
            Ok(outcome)
        } else {
            Err(RunnerError::CommandFailed {
                program: spec.program.clone(),
                code: outcome.code,
                stderr: outcome.stderr.clone(),
            })
        }
    }
}

impl Runner for CommandRunner {
    fn fmt(&self) -> Result<RunOutcome, RunnerError> {
        self.run_command(&self.commands.fmt)
    }

    fn check(&self) -> Result<RunOutcome, RunnerError> {
        self.run_command(&self.commands.check)
    }

    fn test(&self) -> Result<RunOutcome, RunnerError> {
        self.run_command(&self.commands.test)
    }
}

/// Per-command configuration used by the [`CommandRunner`].
#[derive(Debug, Clone)]
pub struct RunnerCommands {
    pub fmt: CommandSpec,
    pub check: CommandSpec,
    pub test: CommandSpec,
}

impl RunnerCommands {
    pub fn from_raw(
        fmt: Vec<String>,
        check: Vec<String>,
        test: Vec<String>,
    ) -> Result<Self, RunnerError> {
        Ok(Self {
            fmt: CommandSpec::from_vec(fmt)?,
            check: CommandSpec::from_vec(check)?,
            test: CommandSpec::from_vec(test)?,
        })
    }
}

/// Simple representation of a CLI command (`program` + `args`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
}

impl CommandSpec {
    pub fn new(
        program: impl Into<String>,
        args: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            program: program.into(),
            args: args.into_iter().map(Into::into).collect(),
        }
    }

    pub fn from_vec(parts: Vec<String>) -> Result<Self, RunnerError> {
        let mut iter = parts.into_iter();
        let program = iter
            .next()
            .ok_or_else(|| RunnerError::InvalidCommand("command cannot be empty".into()))?;
        let args = iter.collect();
        Ok(Self { program, args })
    }
}

#[derive(Debug, Error)]
pub enum RunnerError {
    #[error("{0}")]
    InvalidCommand(String),
    #[error("failed to spawn {program}: {source}")]
    SpawnFailed {
        program: String,
        #[source]
        source: std::io::Error,
    },
    #[error("command {program} failed with code {code}: {stderr}")]
    CommandFailed {
        program: String,
        code: i32,
        stderr: String,
    },
}
