use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use tdd_cli::{executor, init};
use tdd_llm::mock::MockLlmClient;
use tempfile::tempdir;

#[test]
fn init_detects_existing_rust_project() {
    let temp = tempdir().unwrap();
    let root = temp.path();

    // Create an existing Rust project structure
    fs::create_dir(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"existing\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(
        root.join("src/lib.rs"),
        "pub fn existing_fn() -> i32 { 42 }\n",
    )
    .unwrap();

    // Change to the temp directory
    let _workdir = WorkdirGuard::enter(root);

    // Initialize TDD workspace in existing project
    let result = init::initialize_workspace("tdd.yaml").unwrap();

    // Should detect existing workspace
    assert!(result.workspace_exists);

    // Should create TDD-specific files
    assert!(result.config_created);
    assert!(result.kata_created);
    assert!(root.join("tdd.yaml").exists());
    assert!(root.join("kata.md").exists());
    assert!(root.join(".tdd/plan").exists());
    assert!(root.join(".tdd/logs").exists());

    // Should not have overwritten existing files
    assert!(root.join("Cargo.toml").exists());
    assert!(root.join("src/lib.rs").exists());
    let lib_content = fs::read_to_string(root.join("src/lib.rs")).unwrap();
    assert!(lib_content.contains("existing_fn"));
}

#[test]
fn baseline_check_passes_with_passing_tests() {
    let temp = tempdir().unwrap();
    let root = temp.path();

    // Create a Rust project with passing tests
    fs::create_dir(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();
    fs::write(
        root.join("src/lib.rs"),
        r#"pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 2), 4);
    }
}
"#,
    )
    .unwrap();

    // Change to the temp directory
    let _workdir = WorkdirGuard::enter(root);

    // Initialize TDD workspace
    init::initialize_workspace("tdd.yaml").unwrap();
    overwrite_config_for_fast_ci(root);

    // Create mock LLM client and enqueue plan/edit responses for Tester step
    let llm = Arc::new(MockLlmClient::default());
    enqueue_single_step_tester_responses(&llm);

    // Run steps - should succeed with baseline check
    let result = executor::run_steps_with_client("tdd.yaml", 1, llm);

    // Baseline check should pass and allow execution
    if let Err(err) = &result {
        eprintln!("baseline passing test error: {err:?}");
    }
    assert!(
        result.is_ok(),
        "Expected success with passing baseline tests"
    );
}

#[test]
fn baseline_check_fails_with_failing_tests() {
    let temp = tempdir().unwrap();
    let root = temp.path();

    // Create a Rust project with failing tests
    fs::create_dir(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();
    fs::write(
        root.join("src/lib.rs"),
        r#"pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_fails() {
        assert_eq!(add(2, 2), 5); // Intentionally wrong
    }
}
"#,
    )
    .unwrap();

    // Change to the temp directory
    let _workdir = WorkdirGuard::enter(root);

    // Initialize TDD workspace
    init::initialize_workspace("tdd.yaml").unwrap();

    // Create mock LLM client (won't be used because baseline check will fail)
    let llm = Arc::new(MockLlmClient::default());

    // Run steps - should fail at baseline check
    let result = executor::run_steps_with_client("tdd.yaml", 1, llm);

    // Baseline check should fail
    assert!(
        result.is_err(),
        "Expected failure with failing baseline tests"
    );
    let err = result.unwrap_err();
    eprintln!("baseline failure error: {err:?}");
    let error_msg = err.to_string();
    assert!(
        error_msg.contains("Baseline test check failed"),
        "Error should mention baseline test failure"
    );
}

#[test]
fn existing_repo_preserves_git_history() {
    let temp = tempdir().unwrap();
    let root = temp.path();

    // Create an existing Rust project with git history
    fs::create_dir(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"existing\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(root.join("src/lib.rs"), "pub fn v1() -> i32 { 1 }\n").unwrap();

    // Initialize git and create initial commit
    let repo = git2::Repository::init(root).unwrap();
    let sig = git2::Signature::now("Test", "test@example.com").unwrap();
    let mut index = repo.index().unwrap();
    index
        .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
        .unwrap();

    // Change to the temp directory
    let _workdir = WorkdirGuard::enter(root);

    // Get commit count before init
    let commits_before = count_commits(&repo);

    // Initialize TDD workspace
    init::initialize_workspace("tdd.yaml").unwrap();

    // Get commit count after init
    let commits_after = count_commits(&repo);

    // Should preserve existing history (no automatic commits)
    assert_eq!(
        commits_after, commits_before,
        "Existing repo history should remain unchanged during init"
    );

    // Verify original files still exist
    assert!(root.join("src/lib.rs").exists());
    let lib_content = fs::read_to_string(root.join("src/lib.rs")).unwrap();
    assert!(lib_content.contains("v1"));
}

fn count_commits(repo: &git2::Repository) -> usize {
    let mut revwalk = repo.revwalk().unwrap();
    revwalk.push_head().unwrap();
    revwalk.count()
}

fn enqueue_single_step_tester_responses(llm: &Arc<MockLlmClient>) {
    llm.push_response("Plan: add reassurance test");
    llm.push_response(
                r##"{
    "commit_message": "test: ensure add works",
    "notes": "Confirm baseline behavior",
    "files": [
        {"path": "tests/test_add.rs", "contents": "use test_project::add;\n\n#[test]\nfn verifies_add_function() {\n    assert_eq!(add(2, 2), 4);\n}\n"}
    ]
}"##,
        );
}

fn overwrite_config_for_fast_ci(root: &Path) {
    let config_path = root.join("tdd.yaml");
    let config = r#"workspace:
    kata_file: kata.md
    plan_dir: .tdd/plan
    log_dir: .tdd/logs
    max_steps: 10
    max_attempts_per_agent: 2
roles:
    tester:
        model: mock
        temperature: 0.1
    implementor:
        model: mock
        temperature: 0.2
    refactorer:
        model: mock
        temperature: 0.15
llm:
    base_url: http://localhost
    api_key_env: MOCK_TOKEN
ci:
    fmt: ["true"]
    check: ["true"]
    test: ["cargo", "test"]
commit_author:
    name: Fast Bot
    email: fast.bot@example.com
"#;
    fs::write(config_path, config).unwrap();
}

static WORKDIR_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

struct WorkdirGuard {
    original_dir: PathBuf,
    _lock: MutexGuard<'static, ()>,
}

impl WorkdirGuard {
    fn enter(target: &Path) -> Self {
        let lock = WORKDIR_MUTEX
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(target).unwrap();
        Self {
            original_dir,
            _lock: lock,
        }
    }
}

impl Drop for WorkdirGuard {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.original_dir).unwrap();
    }
}
