//! Implementor agent implementation.

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::{
    edit_plan::EditPlan,
    support::{edit_messages, is_source_path, plan_messages},
};
use anyhow::{bail, Result};
use async_trait::async_trait;
use tdd_core::{
    orchestrator::Agent,
    step::{Role, StepContext, StepResult},
};
use tdd_llm::client::LlmClient;

pub struct ImplementorAgent {
    llm: Arc<dyn LlmClient>,
    root: PathBuf,
    last_plan: Mutex<Option<String>>,
}

impl ImplementorAgent {
    pub fn new(llm: Arc<dyn LlmClient>, root: impl AsRef<std::path::Path>) -> Self {
        Self {
            llm,
            root: root.as_ref().to_path_buf(),
            last_plan: Mutex::new(None),
        }
    }
}

#[async_trait]
impl Agent for ImplementorAgent {
    fn role(&self) -> Role {
        Role::Implementor
    }

    async fn plan(&self, ctx: &StepContext) -> Result<String> {
        let messages = plan_messages(IMPLEMENTOR_PLAN_PROMPT, ctx);
        let response = self.llm.chat(Role::Implementor.as_str(), &messages).await?;
        let trimmed = response.trim().to_string();
        *self.last_plan.lock().unwrap() = Some(trimmed.clone());
        Ok(trimmed)
    }

    async fn edit(&self, ctx: &StepContext) -> Result<StepResult> {
        let cached = self.last_plan.lock().unwrap().clone();
        let messages = edit_messages(IMPLEMENTOR_EDIT_PROMPT, ctx, cached.as_deref());
        let response = self.llm.chat(Role::Implementor.as_str(), &messages).await?;
        let plan = EditPlan::parse(&response)?;
        enforce_implementor_scope(&plan)?;
        let files_changed = plan.apply(&self.root)?;
        Ok(StepResult {
            files_changed,
            commit_message: plan.commit_message().to_string(),
            notes: plan.notes().to_string(),
        })
    }
}

const IMPLEMENTOR_PLAN_PROMPT: &str = r#"
You are the Implementor agent. Study the failing tests and outline the minimal code change needed to pass them.
Keep the plan focused on production code adjustments.
"#;

const IMPLEMENTOR_EDIT_PROMPT: &str = r#"
You are applying the minimal code change required to make tests pass.
Only touch the files that are absolutely necessary, preferring small, surgical diffs.
Return a JSON edit plan according to the schema.
"#;

fn enforce_implementor_scope(plan: &EditPlan) -> Result<()> {
    let mut source_files = 0;
    for file in plan.files() {
        if is_source_path(file.path()) {
            source_files += 1;
        }
    }
    if source_files == 0 {
        bail!("Implementor must modify at least one source file");
    }
    if plan.files().len() > 5 {
        bail!("Implementor plan is too large; limit edits to at most 5 files");
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
            Role::Implementor,
            2,
            "Continue kata",
            "test: add failing test",
            "diff",
            vec!["src/lib.rs".into(), "tests/lib.rs".into()],
        )
    }

    #[tokio::test]
    async fn plan_returns_llm_response() {
        let ctx = sample_context();
        let llm = Arc::new(MockLlmClient::default());
        llm.push_response("Implement addition");
        let dir = tempdir().unwrap();
        let agent = ImplementorAgent::new(llm, dir.path());
        let plan = agent.plan(&ctx).await.unwrap();
        assert!(plan.contains("Implement"));
    }

    #[tokio::test]
    async fn edit_applies_changes_with_source_file() {
        let ctx = sample_context();
        let llm = Arc::new(MockLlmClient::default());
        llm.push_response("Plan");
        llm.push_response(
            r#"{
				"commit_message": "feat: make tests pass",
				"notes": "",
				"files": [
					{"path": "src/lib.rs", "contents": "pub fn hi() -> u32 { 42 }"}
				]
			}"#,
        );
        let dir = tempdir().unwrap();
        let agent = ImplementorAgent::new(llm, dir.path());
        let _ = agent.plan(&ctx).await.unwrap();
        let result = agent.edit(&ctx).await.unwrap();
        assert_eq!(result.files_changed, vec!["src/lib.rs"]);
    }

    #[tokio::test]
    async fn edit_requires_source_file() {
        let ctx = sample_context();
        let llm = Arc::new(MockLlmClient::default());
        llm.push_response("Plan");
        llm.push_response(
            r#"{
				"commit_message": "feat: bad",
				"notes": "",
				"files": [
					{"path": "tests/lib.rs", "contents": "fn test_case() {}"}
				]
			}"#,
        );
        let dir = tempdir().unwrap();
        let agent = ImplementorAgent::new(llm, dir.path());
        let _ = agent.plan(&ctx).await.unwrap();
        let err = agent.edit(&ctx).await.unwrap_err();
        assert!(err.to_string().contains("source file"));
    }

    #[tokio::test]
    async fn edit_rejects_when_too_many_files() {
        let ctx = sample_context();
        let llm = Arc::new(MockLlmClient::default());
        llm.push_response("Plan");
        llm.push_response(
            r#"{
				"commit_message": "feat: huge",
				"notes": "",
				"files": [
					{"path": "src/a.rs", "contents": ""},
					{"path": "src/b.rs", "contents": ""},
					{"path": "src/c.rs", "contents": ""},
					{"path": "src/d.rs", "contents": ""},
					{"path": "src/e.rs", "contents": ""},
					{"path": "src/f.rs", "contents": ""}
				]
			}"#,
        );
        let dir = tempdir().unwrap();
        let agent = ImplementorAgent::new(llm, dir.path());
        let _ = agent.plan(&ctx).await.unwrap();
        let err = agent.edit(&ctx).await.unwrap_err();
        assert!(err.to_string().contains("too large"));
    }
}
