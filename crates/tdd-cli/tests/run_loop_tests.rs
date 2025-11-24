use std::{fs, path::Path, sync::Arc};

use git2::Repository;
use tdd_cli::executor;
use tdd_llm::mock::MockLlmClient;
use tempfile::tempdir;

#[test]
fn run_command_executes_three_steps_with_mock_llm() {
    let temp = tempdir().expect("temp dir");
    write_kata_files(temp.path());

    let llm = Arc::new(MockLlmClient::default());
    enqueue_mock_responses(&llm);

    let config_path = temp.path().join("tdd.yaml");
    let summary = executor::run_steps_with_client(config_path, 3, llm).expect("run succeeds");
    assert_eq!(summary.executed, 3);

    let repo = Repository::open(temp.path()).expect("git repo opened");
    let mut revwalk = repo.revwalk().expect("revwalk");
    revwalk.push_head().expect("push head");
    assert_eq!(revwalk.count(), 3, "expected three commits");

    let plan_dir = temp.path().join(".tdd/plan");
    assert!(plan_dir.join("step-001-tester.md").exists());
    assert!(plan_dir.join("step-002-implementor.md").exists());
    assert!(plan_dir.join("step-003-refactorer.md").exists());
}

fn write_kata_files(root: &Path) {
    fs::write(root.join("kata.md"), "Practice string calculator").expect("kata file");
    fs::write(root.join("tdd.yaml"), config_template()).expect("config file");
}

fn config_template() -> &'static str {
    r#"workspace:
  kata_file: kata.md
  plan_dir: .tdd/plan
  log_dir: .tdd/logs
  max_steps: 10
roles:
  tester:
    model: mock
  implementor:
    model: mock
  refactorer:
    model: mock
llm:
  base_url: http://localhost
  api_key_env: MOCK_KEY
ci:
  fmt: ["true"]
  check: ["true"]
  test: ["true"]
commit_author:
  name: Tester Bot
  email: tester@example.com
"#
}

fn enqueue_mock_responses(llm: &Arc<MockLlmClient>) {
    llm.push_response("Plan: add failing test");
    llm.push_response(
        r##"{
  "commit_message": "test: cover empty input",
  "notes": "Add failing test for empty string",
  "files": [
    {"path": "tests/string_calculator.rs", "contents": "#[test]\nfn fails() { assert_eq!(1, 2); }"}
  ]
}"##,
    );

    llm.push_response("Plan: make implementation pass");
    llm.push_response(
        r##"{
  "commit_message": "feat: handle empty input",
  "notes": "Return zero for empty input",
  "files": [
    {"path": "src/lib.rs", "contents": "pub fn add(input: &str) -> u32 { if input.trim().is_empty() { 0 } else { input.len() as u32 } }"}
  ]
}"##,
    );

    llm.push_response("Plan: tidy implementation");
    llm.push_response(
        r##"{
  "commit_message": "refactor: tighten add",
  "notes": "Remove duplication",
  "files": [
    {"path": "src/lib.rs", "contents": "pub fn add(input: &str) -> u32 { (input.trim().is_empty()) as u32 * 0 }"}
  ]
}"##,
    );
}
