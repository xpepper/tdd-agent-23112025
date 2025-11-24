use std::{fs, path::Path, sync::Arc};

use tempfile::tempdir;
use tdd_cli::{executor, status};
use tdd_core::step::Role;
use tdd_llm::mock::MockLlmClient;

#[test]
fn status_reports_next_role_and_ci_codes() {
    let temp = tempdir().expect("temp dir");
    write_kata_files(temp.path());

    let llm = Arc::new(MockLlmClient::default());
    enqueue_mock_responses(&llm);

    let config_path = temp.path().join("tdd.yaml");
    executor::run_steps_with_client(&config_path, 2, llm).expect("run succeeds");

    let report = status::gather_status(&config_path).expect("status gathers");
    assert_eq!(report.next_role, Role::Refactorer);
    assert_eq!(report.next_step, 3);
    assert!(report.repo_clean);

    let log = report.last_log.as_ref().expect("log exists");
    assert_eq!(log.role, Role::Implementor);
    assert_eq!(log.step_index, 2);
    assert_eq!(log.runner.test.code, 0);

    let summary = report.format_lines().join("\n");
    assert!(summary.contains("Next role: refactorer"));
    assert!(summary.contains("CI exit codes"));
}

fn write_kata_files(root: &Path) {
    fs::write(root.join("kata.md"), "Practice string calculator").expect("kata file");
    fs::write(root.join("tdd.yaml"), config_template()).expect("config file");
  fs::write(root.join(".gitignore"), ".tdd/logs/\n").expect("gitignore");
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
}
