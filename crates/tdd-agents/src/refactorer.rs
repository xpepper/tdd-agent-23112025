//! Refactorer agent implementation.

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::{
    edit_plan::EditPlan,
    support::{edit_messages, is_source_path, is_test_path, plan_messages},
};
use anyhow::{bail, Result};
use async_trait::async_trait;
use tdd_core::{
    orchestrator::Agent,
    step::{Role, StepContext, StepResult},
};
use tdd_llm::client::LlmClient;

pub struct RefactorerAgent {
    llm: Arc<dyn LlmClient>,
    root: PathBuf,
    last_plan: Mutex<Option<String>>,
}

impl RefactorerAgent {
    pub fn new(llm: Arc<dyn LlmClient>, root: impl AsRef<std::path::Path>) -> Self {
        Self {
            llm,
            root: root.as_ref().to_path_buf(),
            last_plan: Mutex::new(None),
        }
    }
}

#[async_trait]
impl Agent for RefactorerAgent {
    fn role(&self) -> Role {
        Role::Refactorer
    }

    async fn plan(&self, ctx: &StepContext) -> Result<String> {
        let messages = plan_messages(REFACTORER_PLAN_PROMPT, ctx);
        let response = self.llm.chat(Role::Refactorer.as_str(), &messages).await?;
        let trimmed = response.trim().to_string();
        *self.last_plan.lock().unwrap() = Some(trimmed.clone());
        Ok(trimmed)
    }

    async fn edit(&self, ctx: &StepContext) -> Result<StepResult> {
        let cached = self.last_plan.lock().unwrap().clone();
        let messages = edit_messages(REFACTORER_EDIT_PROMPT, ctx, cached.as_deref());
        let response = self.llm.chat(Role::Refactorer.as_str(), &messages).await?;
        let plan = EditPlan::parse(&response)?;
        enforce_refactorer_scope(&plan)?;
        let files_changed = plan.apply(&self.root)?;
        Ok(StepResult {
            files_changed,
            commit_message: plan.commit_message().to_string(),
            notes: plan.notes().to_string(),
        })
    }
}

const REFACTORER_PLAN_PROMPT: &str = r#"
You are the Refactorer agent. Identify safe improvements that keep behavior and tests unchanged.
Focus on cleanup, deduplication, and clarity improvements after the green step.
"#;

const REFACTORER_EDIT_PROMPT: &str = r#"
Apply refactorings that do not modify any tests or change observable behavior.
Only touch production code files and keep the edit set small.
Return the JSON edit plan per schema.
"#;

fn enforce_refactorer_scope(plan: &EditPlan) -> Result<()> {
    if plan.files().is_empty() {
        bail!("Refactorer plans must modify at least one source file");
    }
    if plan.files().len() > 5 {
        bail!("Refactorer plan touches too many files; limit to 5");
    }
    for file in plan.files() {
        if is_test_path(file.path()) {
            bail!("Refactorer cannot modify test files ({}).", file.path());
        }
        if !is_source_path(file.path()) {
            bail!(
                "Refactorer may only modify Rust source files ({}).",
                file.path()
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tdd_llm::mock::MockLlmClient;
    use tempfile::tempdir;

    fn sample_context() -> StepContext {
        StepContext::new(
            Role::Refactorer,
            3,
            "Continue kata",
            "feat: make tests pass",
            "diff",
            vec!["src/lib.rs".into(), "src/utils.rs".into()],
        )
    }

    #[tokio::test]
    async fn plan_returns_llm_response() {
        let ctx = sample_context();
        let llm = Arc::new(MockLlmClient::default());
        llm.push_response("Refactor duplication");
        let dir = tempdir().unwrap();
        let agent = RefactorerAgent::new(llm, dir.path());
        let plan = agent.plan(&ctx).await.unwrap();
        assert!(plan.contains("Refactor"));
    }

    #[tokio::test]
    async fn edit_applies_refactor_changes() {
        let ctx = sample_context();
        let llm = Arc::new(MockLlmClient::default());
        llm.push_response("Plan");
        llm.push_response(
            r#"{
				"commit_message": "refactor: tidy utils",
				"notes": "",
				"files": [
					{"path": "src/utils.rs", "contents": "pub fn clean() {}"}
				]
			}"#,
        );
        let dir = tempdir().unwrap();
        let agent = RefactorerAgent::new(llm, dir.path());
        let _ = agent.plan(&ctx).await.unwrap();
        let result = agent.edit(&ctx).await.unwrap();
        assert_eq!(result.files_changed, vec!["src/utils.rs"]);
    }

    #[tokio::test]
    async fn edit_rejects_test_changes() {
        let ctx = sample_context();
        let llm = Arc::new(MockLlmClient::default());
        llm.push_response("Plan");
        llm.push_response(
            r#"{
				"commit_message": "refactor: bad",
				"notes": "",
				"files": [
					{"path": "tests/lib.rs", "contents": "fn test_case() {}"}
				]
			}"#,
        );
        let dir = tempdir().unwrap();
        let agent = RefactorerAgent::new(llm, dir.path());
        let _ = agent.plan(&ctx).await.unwrap();
        let err = agent.edit(&ctx).await.unwrap_err();
        assert!(err.to_string().contains("test files"));
    }
}
