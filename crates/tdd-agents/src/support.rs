//! Shared helpers for agent prompts and path constraints.

use std::fmt::Write;

use tdd_core::step::StepContext;
use tdd_llm::client::{Message, MessageRole};

/// JSON schema hint shared by all agents when requesting edit plans.
pub const EDIT_PLAN_INSTRUCTIONS: &str = r#"
Return **only** JSON matching this schema:
{
  "commit_message": "conventional commit summary",
  "notes": "bullet list or paragraph summarizing edits",
  "files": [
    { "path": "relative/path.rs", "contents": "entire file contents" }
  ]
}
Do not include prose outside of the JSON object.
"#;

/// Build chat messages for the planning phase.
pub fn plan_messages(system_prompt: &str, ctx: &StepContext) -> Vec<Message> {
    vec![
        Message {
            role: MessageRole::System,
            content: system_prompt.to_string(),
        },
        Message {
            role: MessageRole::User,
            content: format_context_payload("Outline your next test strategy.", ctx),
        },
    ]
}

/// Build chat messages for the edit phase, reusing the cached plan if present.
pub fn edit_messages(
    system_prompt: &str,
    ctx: &StepContext,
    cached_plan: Option<&str>,
) -> Vec<Message> {
    let mut user_instructions = String::new();
    if let Some(plan) = cached_plan {
        let _ = writeln!(user_instructions, "Previously proposed plan:\n{plan}\n");
    }
    user_instructions.push_str(EDIT_PLAN_INSTRUCTIONS);
    user_instructions.push_str("\n\nApply edits now using the repository context below.");

    vec![
        Message {
            role: MessageRole::System,
            content: system_prompt.to_string(),
        },
        Message {
            role: MessageRole::User,
            content: format_context_payload(&user_instructions, ctx),
        },
    ]
}

/// Determine whether the provided path targets a Rust test file.
pub fn is_test_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    normalized.starts_with("tests/")
        || normalized.contains("/tests/")
        || normalized.ends_with("_test.rs")
        || normalized.ends_with("_tests.rs")
        || normalized.ends_with("test.rs")
        || normalized.contains("/test/")
}

/// Determine whether a path targets Rust source files.
pub fn is_source_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    normalized.starts_with("src/")
        || normalized.starts_with("examples/")
        || normalized == "Cargo.toml"
        || normalized.ends_with(".rs") && !is_test_path(&normalized)
}

fn format_context_payload(instruction: &str, ctx: &StepContext) -> String {
    let mut buffer = String::new();
    let _ = writeln!(buffer, "Instruction:\n{instruction}\n");
    let _ = writeln!(buffer, "Role: {}", ctx.role.as_str());
    let _ = writeln!(buffer, "Step: {}", ctx.step_index);
    let _ = writeln!(
        buffer,
        "Kata description:\n{}\n",
        truncate(&ctx.kata_description, 1200)
    );
    if !ctx.git_last_commit_msg.trim().is_empty() {
        let _ = writeln!(
            buffer,
            "Last commit message:\n{}\n",
            truncate(&ctx.git_last_commit_msg, 600)
        );
    }
    if !ctx.git_last_diff.trim().is_empty() {
        let _ = writeln!(
            buffer,
            "Last diff snippet:\n{}\n",
            truncate(&ctx.git_last_diff, 1200)
        );
    }
    if !ctx.repo_snapshot_paths.is_empty() {
        let file_list = ctx
            .repo_snapshot_paths
            .iter()
            .take(30)
            .map(|path| format!("- {path}"))
            .collect::<Vec<_>>()
            .join("\n");
        let _ = writeln!(buffer, "Tracked files (first 30):\n{file_list}\n");
    }
    buffer
}

fn truncate(input: &str, limit: usize) -> String {
    if input.len() <= limit {
        return input.to_string();
    }
    let mut truncated = input[..limit].to_string();
    truncated.push('…');
    truncated
}
