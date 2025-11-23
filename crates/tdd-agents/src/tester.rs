//! Tester agent implementation.

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::{
    edit_plan::EditPlan,
    support::{edit_messages, is_test_path, plan_messages},
};
use anyhow::{bail, Result};
use async_trait::async_trait;
use tdd_core::{
    orchestrator::Agent,
    step::{Role, StepContext, StepResult},
};
use tdd_llm::client::LlmClient;

pub struct TesterAgent {
    llm: Arc<dyn LlmClient>,
    root: PathBuf,
    last_plan: Mutex<Option<String>>,
}

impl TesterAgent {
    pub fn new(llm: Arc<dyn LlmClient>, root: impl AsRef<std::path::Path>) -> Self {
        Self {
            llm,
            root: root.as_ref().to_path_buf(),
            last_plan: Mutex::new(None),
        }
    }
}

#[async_trait]
impl Agent for TesterAgent {
    fn role(&self) -> Role {
        Role::Tester
    }

    async fn plan(&self, ctx: &StepContext) -> Result<String> {
        let messages = plan_messages(TESTER_PLAN_PROMPT, ctx);
        let response = self.llm.chat(Role::Tester.as_str(), &messages).await?;
        let trimmed = response.trim().to_string();
        *self.last_plan.lock().unwrap() = Some(trimmed.clone());
        Ok(trimmed)
    }

    async fn edit(&self, ctx: &StepContext) -> Result<StepResult> {
        let cached = self.last_plan.lock().unwrap().clone();
        let messages = edit_messages(TESTER_EDIT_PROMPT, ctx, cached.as_deref());
        let response = self.llm.chat(Role::Tester.as_str(), &messages).await?;
        let plan = EditPlan::parse(&response)?;
        enforce_test_file_scope(&plan)?;
        let files_changed = plan.apply(&self.root)?;
        Ok(StepResult {
            files_changed,
            commit_message: plan.commit_message().to_string(),
            notes: plan.notes().to_string(),
        })
    }
}

const TESTER_PLAN_PROMPT: &str = r#"
You are the Tester agent in a strict red-green-refactor loop.
Focus on identifying the next failing test that reveals missing behavior.
Respond with a concise plan (bullets encouraged) describing what test you will add and why.
Do not describe implementation changes.
"#;

const TESTER_EDIT_PROMPT: &str = r#"
You are the Tester agent applying changes.
Only modify test files. Do not change production code.
Return a JSON edit plan that adds or updates tests according to the schema.
Ensure the resulting test fails for the current implementation.
"#;

fn enforce_test_file_scope(plan: &EditPlan) -> Result<()> {
    if plan.files().is_empty() {
        bail!("Tester plans must include at least one test file edit");
    }
    for file in plan.files() {
        if !is_test_path(file.path()) {
            bail!("Tester edits must touch tests only (got {})", file.path());
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
            Role::Tester,
            1,
            "Tackle string calculator",
            "",
            "",
            vec!["kata.md".into()],
        )
    }

    #[tokio::test]
    async fn plan_returns_llm_response() {
        let ctx = sample_context();
        let llm = Arc::new(MockLlmClient::default());
        llm.push_response("Write failing test");
        let dir = tempdir().unwrap();
        let agent = TesterAgent::new(llm, dir.path());
        let plan = agent.plan(&ctx).await.unwrap();
        assert!(plan.contains("failing test"));
    }

    #[tokio::test]
    async fn edit_applies_test_only_changes() {
        let dir = tempdir().unwrap();
        let ctx = sample_context();
        let llm = Arc::new(MockLlmClient::default());
        llm.push_response("Plan");
        llm.push_response(
            r#"{
				"commit_message": "test: add failing case",
				"notes": "Add coverage",
				"files": [{"path": "tests/math.rs", "contents": "fn thing() {}"}]
			}"#,
        );
        let agent = TesterAgent::new(llm, dir.path());
        let _ = agent.plan(&ctx).await.unwrap();
        let result = agent.edit(&ctx).await.unwrap();
        assert_eq!(result.files_changed, vec!["tests/math.rs"]);
        assert!(dir.path().join("tests/math.rs").exists());
    }

    #[tokio::test]
    async fn edit_rejects_non_test_paths() {
        let dir = tempdir().unwrap();
        let ctx = sample_context();
        let llm = Arc::new(MockLlmClient::default());
        llm.push_response("Plan");
        llm.push_response(
            r#"{
				"commit_message": "test: bad",
				"notes": "",
				"files": [{"path": "src/lib.rs", "contents": "pub fn hi() {}"}]
			}"#,
        );
        let agent = TesterAgent::new(llm, dir.path());
        let _ = agent.plan(&ctx).await.unwrap();
        let err = agent.edit(&ctx).await.unwrap_err();
        assert!(err.to_string().contains("tests only"));
    }
}
