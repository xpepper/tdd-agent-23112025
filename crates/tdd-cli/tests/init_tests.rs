use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard, OnceLock},
};

use serde_yaml::{Mapping, Value};
use tdd_cli::init;
use tempfile::tempdir;

#[test]
fn init_creates_workspace_directories_and_state() {
    let temp = tempdir().expect("temp dir");
    let _guard = WorkdirGuard::enter(temp.path());

    let result = init::initialize_workspace("tdd.yaml").expect("init succeeds");

    assert!(result.config_created, "config file should be newly created");
    assert!(temp.path().join("kata.md").exists());
    assert!(temp.path().join("tdd.yaml").exists());
    assert!(temp.path().join(".tdd/plan").is_dir());
    assert!(temp.path().join(".tdd/logs").is_dir());
    assert!(
        temp.path().join(".tdd/state").is_dir(),
        "state directory should be created"
    );
}

#[test]
fn init_is_idempotent_on_subsequent_runs() {
    let temp = tempdir().expect("temp dir");
    let _guard = WorkdirGuard::enter(temp.path());

    let first = init::initialize_workspace("tdd.yaml").expect("first init succeeds");
    assert!(first.config_created);

    let second = init::initialize_workspace("tdd.yaml").expect("second init succeeds");
    assert!(
        !second.config_created,
        "second init should reuse existing config"
    );
    assert!(!second.kata_created, "existing kata.md should be preserved");
}

#[test]
fn bootstrap_command_runs_successfully() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    let _guard = WorkdirGuard::enter(root);

    init::initialize_workspace("tdd.yaml").expect("init succeeds");

    let script = write_bootstrap_script(root, "echo bootstrapped > bootstrap-output.txt");
    enable_bootstrap(root, script);

    let summary = init::run_bootstrap("tdd.yaml", false).expect("bootstrap succeeds");
    assert!(
        root.join("bootstrap-output.txt").exists(),
        "bootstrap script should create output file"
    );
    assert!(
        !summary.skipped,
        "bootstrap must run when skip markers are absent"
    );
}

#[test]
fn bootstrap_command_failures_surface_errors() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    let _guard = WorkdirGuard::enter(root);

    init::initialize_workspace("tdd.yaml").expect("init succeeds");

    let script = write_bootstrap_script(root, "echo fail >&2; exit 9");
    enable_bootstrap(root, script);

    let err =
        init::run_bootstrap("tdd.yaml", false).expect_err("bootstrap should bubble up failures");
    let message = format!("{err:?}");
    assert!(message.contains("exit code"));
    assert!(message.contains("9"));
}

fn write_bootstrap_script(root: &Path, body: &str) -> PathBuf {
    let script = root.join("bootstrap.sh");
    let script_body = format!("#!/bin/sh\nset -euo pipefail\n{body}\n");
    fs::write(&script, script_body).expect("write script");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&script).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script, perms).expect("chmod script");
    }
    script
}

fn enable_bootstrap(root: &Path, script: PathBuf) {
    let config_path = root.join("tdd.yaml");
    let raw = fs::read_to_string(&config_path).expect("read config");
    let mut doc: Value = serde_yaml::from_str(&raw).expect("parse config");

    let workspace = doc
        .get_mut("workspace")
        .and_then(Value::as_mapping_mut)
        .expect("workspace map");

    let mut bootstrap = Mapping::new();
    bootstrap.insert(
        Value::from("command"),
        Value::Sequence(vec![
            Value::String("/bin/sh".into()),
            Value::String(script.to_string_lossy().into()),
        ]),
    );
    bootstrap.insert(Value::from("working_dir"), Value::String(".".into()));

    workspace.insert(Value::from("bootstrap"), Value::Mapping(bootstrap));

    let updated = serde_yaml::to_string(&doc).expect("serialize config");
    fs::write(&config_path, updated).expect("write updated config");
}

static WORKDIR_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

struct WorkdirGuard {
    original: PathBuf,
    _lock: MutexGuard<'static, ()>,
}

impl WorkdirGuard {
    fn enter(target: &Path) -> Self {
        let lock = WORKDIR_MUTEX
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let original = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(target).expect("chdir");
        Self {
            original,
            _lock: lock,
        }
    }
}

impl Drop for WorkdirGuard {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.original).expect("restore cwd");
    }
}
