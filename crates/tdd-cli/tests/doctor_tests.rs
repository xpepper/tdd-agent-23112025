use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};

use tempfile::tempdir;
use tdd_cli::{doctor, init};
use tdd_core::logging::BootstrapLogEntry;

#[test]
fn doctor_reports_clean_environment() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    let _guard = WorkdirGuard::enter(root);

    init::initialize_workspace("tdd.yaml").expect("init succeeds");
    let server = LocalHttpServer::start();
    write_test_config(root, server.url(), "TEST_TOKEN", &["true"], &["true"], &["true"], true);
    let _env = EnvGuard::set("TEST_TOKEN", "token");
    write_bootstrap_state(root, BootstrapLogEntry {
        command: vec!["sh".into(), "bootstrap.sh".into()],
        working_dir: ".".into(),
        skipped: false,
        skip_reason: None,
        exit_code: Some(0),
        stdout: String::new(),
        stderr: String::new(),
        started_at_ms: 1,
        duration_ms: 1,
    });

    let report = doctor::run_doctor("tdd.yaml").expect("doctor succeeds");
    assert!(report.git_clean, "git repo should be clean");
    assert!(report.ci.fmt_ready && report.ci.check_ready && report.ci.test_ready);
    assert!(report.llm.token_loaded);
    assert!(report.llm.base_url_reachable);
    let bootstrap = report.bootstrap.expect("bootstrap report");
    assert!(bootstrap.healthy);
    assert!(report.issues.is_empty(), "no issues expected");
}

#[test]
fn doctor_flags_missing_requirements() {
    let temp = tempdir().expect("temp dir");
    let root = temp.path();
    let _guard = WorkdirGuard::enter(root);

    init::initialize_workspace("tdd.yaml").expect("init succeeds");
    write_test_config(
        root,
        "http://127.0.0.1:9",
        "MISSING_TOKEN",
        &["definitely-missing-binary"],
        &["true"],
        &["true"],
        true,
    );
    fs::write(root.join("kata.md"), "dirty").expect("dirty workspace");
    remove_env_var("MISSING_TOKEN");

    let report = doctor::run_doctor("tdd.yaml").expect("doctor succeeds");
    assert!(!report.git_clean, "should detect dirty repo");
    assert!(!report.ci.fmt_ready, "missing fmt binary");
    assert!(!report.llm.token_loaded, "token should be missing");
    assert!(!report.llm.base_url_reachable, "url should be unreachable");
    let bootstrap = report.bootstrap.expect("bootstrap report");
    assert!(!bootstrap.healthy, "bootstrap should be unhealthy without state");
    assert!(report
        .issues
        .iter()
        .any(|issue| issue.contains("LLM token")));
    assert!(report
        .issues
        .iter()
        .any(|issue| issue.contains("Missing CI")));
    assert!(report
        .issues
        .iter()
        .any(|issue| issue.contains("bootstrap")));
}

struct LocalHttpServer {
    #[allow(dead_code)]
    listener: TcpListener,
    url: String,
}

impl LocalHttpServer {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind http");
        listener
            .set_nonblocking(true)
            .expect("make listener nonblocking");
        let url = format!("http://{}", listener.local_addr().unwrap());
        Self { listener, url }
    }

    fn url(&self) -> &str {
        &self.url
    }
}

struct EnvGuard {
    key: String,
    original: Option<String>,
}

impl EnvGuard {
    fn set(key: &str, value: &str) -> Self {
        let original = std::env::var(key).ok();
        std::env::set_var(key, value);
        Self {
            key: key.into(),
            original,
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        if let Some(value) = &self.original {
            std::env::set_var(&self.key, value);
        } else {
            std::env::remove_var(&self.key);
        }
    }
}

fn remove_env_var(key: &str) {
    std::env::remove_var(key);
}

fn write_test_config(
    root: &Path,
    base_url: &str,
    token_env: &str,
    fmt: &[&str],
    check: &[&str],
    test: &[&str],
    include_bootstrap: bool,
) {
    let bootstrap_block = if include_bootstrap {
        r#"  bootstrap:
    command: ["/bin/sh", "./scripts/bootstrap.sh"]
    working_dir: "."
    skip_files:
      - ".tdd/state/bootstrap.skip"
"#
    } else {
        ""
    };

    let config = format!(
        "workspace:\n  kata_file: kata.md\n  plan_dir: .tdd/plan\n  log_dir: .tdd/logs\n  max_steps: 10\n  max_attempts_per_agent: 2\n{bootstrap}roles:\n  tester:\n    model: mock\n  implementor:\n    model: mock\n  refactorer:\n    model: mock\nllm:\n  provider: openai\n  base_url: {base_url}\n  api_key_env: {token_env}\nci:\n  fmt: {fmt:?}\n  check: {check:?}\n  test: {test:?}\ncommit_author:\n  name: Test\n  email: test@example.com\n",
        bootstrap = bootstrap_block,
        base_url = base_url,
        token_env = token_env,
        fmt = fmt,
        check = check,
        test = test
    );
    fs::write(root.join("tdd.yaml"), config).expect("write config");
}

fn write_bootstrap_state(root: &Path, entry: BootstrapLogEntry) {
    let state_path = root.join(".tdd/state/bootstrap.json");
    if let Some(parent) = state_path.parent() {
        fs::create_dir_all(parent).expect("create state dir");
    }
    let json = serde_json::to_string(&entry).expect("serialize bootstrap state");
    fs::write(&state_path, json).expect("write state file");
}

use std::sync::{Mutex, MutexGuard, OnceLock};

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
        let original = std::env::current_dir().expect("current dir");
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
