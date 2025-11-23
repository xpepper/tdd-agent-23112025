use std::fs;

use tdd_exec::runner::{CommandRunner, Runner, RunnerCommands};
use tdd_exec::vcs::{CommitSignature, GitVcs, Vcs};
use tempfile::tempdir;

fn success_command() -> Vec<String> {
    if cfg!(windows) {
        vec!["cmd".into(), "/C".into(), "exit 0".into()]
    } else {
        vec!["sh".into(), "-c".into(), "true".into()]
    }
}

fn failure_command() -> Vec<String> {
    if cfg!(windows) {
        vec!["cmd".into(), "/C".into(), "exit 1".into()]
    } else {
        vec!["sh".into(), "-c".into(), "exit 1".into()]
    }
}

#[test]
fn command_runner_executes_commands() {
    let dir = tempdir().unwrap();
    let commands =
        RunnerCommands::from_raw(success_command(), success_command(), success_command()).unwrap();
    let runner = CommandRunner::new(dir.path(), commands);

    runner.fmt().expect("fmt command should succeed");
    runner.check().expect("check command should succeed");
    runner.test().expect("test command should succeed");
}

#[test]
fn command_runner_surfaces_failures() {
    let dir = tempdir().unwrap();
    let commands =
        RunnerCommands::from_raw(failure_command(), success_command(), success_command()).unwrap();
    let runner = CommandRunner::new(dir.path(), commands);

    assert!(runner.fmt().is_err());
}

#[test]
fn git_vcs_initializes_and_commits() {
    let dir = tempdir().unwrap();
    let repo_path = dir.path();
    let vcs = GitVcs::open_or_init(repo_path).expect("git repo");
    vcs.ensure_initialized().unwrap();

    let file_path = repo_path.join("hello.txt");
    fs::write(&file_path, "hello").unwrap();
    vcs.stage_all().unwrap();
    let oid = vcs
        .commit(
            "feat: add hello",
            &CommitSignature::new("Test", "test@example.com"),
        )
        .expect("commit");

    let state = vcs.state().unwrap();
    assert_eq!(state.head_commit, Some(oid));
    assert!(state.is_clean);
}
