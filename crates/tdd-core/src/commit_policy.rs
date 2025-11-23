//! Conventional commit helpers shared by the orchestrator.

use std::path::Path;

use crate::step::Role;
use tdd_exec::runner::RunOutcome;

/// Formats commit messages that include context, rationale, diff summary, and verification.
#[derive(Debug, Default, Clone)]
pub struct CommitPolicy;

impl CommitPolicy {
    pub fn format(&self, input: CommitMessageInputs<'_>) -> String {
        let (summary, body) = split_message(input.agent_commit_message);
        let kata_goal = extract_kata_goal(input.kata_description);
        let rationale = if input.notes.trim().is_empty() {
            "- Agent did not provide additional notes.".to_string()
        } else {
            input
                .notes
                .lines()
                .map(|line| format!("- {}", line.trim()))
                .collect::<Vec<_>>()
                .join("\n")
        };
        let diff_summary = if input.files_changed.is_empty() {
            "- No files reported".to_string()
        } else {
            input
                .files_changed
                .iter()
                .map(|file| format!("- {file}"))
                .collect::<Vec<_>>()
                .join("\n")
        };
        let verification = format_verification(input.runner_outcomes);
        let plan_ref = input.plan_path.display().to_string();

        let mut message = summary;
        if let Some(body) = body {
            if !body.trim().is_empty() {
                message.push('\n');
                message.push_str(body.trim());
            }
        }

        format!(
			"{message}\n\nContext:\n- Role: {role}\n- Step: {step}\n- Kata goal: {kata_goal}\n- Plan: {plan}\n\nRationale:\n{rationale}\n\nDiff summary:\n{diff}\n\nVerification:\n{verification}\n",
			role = input.role.as_str(),
			step = input.step_index,
			kata_goal = kata_goal,
			plan = plan_ref,
			rationale = rationale,
			diff = diff_summary,
			verification = verification,
		)
    }
}

/// Inputs required to generate a commit message.
pub struct CommitMessageInputs<'a> {
    pub role: Role,
    pub step_index: u32,
    pub kata_description: &'a str,
    pub agent_commit_message: &'a str,
    pub notes: &'a str,
    pub files_changed: &'a [String],
    pub plan_path: &'a Path,
    pub runner_outcomes: &'a RunnerOutcomeSummary,
}

/// Captures the CI command outputs for the verification section.
#[derive(Debug, Clone)]
pub struct RunnerOutcomeSummary {
    pub fmt: RunOutcome,
    pub check: RunOutcome,
    pub test: RunOutcome,
}

impl RunnerOutcomeSummary {
    pub fn new(fmt: RunOutcome, check: RunOutcome, test: RunOutcome) -> Self {
        Self { fmt, check, test }
    }
}

fn split_message(message: &str) -> (String, Option<String>) {
    let mut lines = message.lines();
    let summary = lines.next().unwrap_or("chore: update").trim().to_string();
    let rest = lines.collect::<Vec<_>>();
    let body = if rest.is_empty() {
        None
    } else {
        let joined = rest.join("\n");
        if joined.trim().is_empty() {
            None
        } else {
            Some(joined)
        }
    };
    (summary, body)
}

fn extract_kata_goal(description: &str) -> &str {
    description
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("See kata.md for details")
}

fn format_verification(summary: &RunnerOutcomeSummary) -> String {
    [
        ("fmt", &summary.fmt),
        ("check", &summary.check),
        ("test", &summary.test),
    ]
    .iter()
    .map(|(label, outcome)| {
        let log = if outcome.stdout.trim().is_empty() {
            String::new()
        } else {
            format!(" ({})", outcome.stdout.trim())
        };
        format!("- {label}: exit {}{log}", outcome.code)
    })
    .collect::<Vec<_>>()
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_commit_with_sections() {
        let policy = CommitPolicy::default();
        let runner_summary = RunnerOutcomeSummary::new(
            RunOutcome {
                code: 0,
                stdout: "fmt ok".into(),
                stderr: String::new(),
            },
            RunOutcome {
                code: 0,
                stdout: "check ok".into(),
                stderr: String::new(),
            },
            RunOutcome {
                code: 0,
                stdout: "tests ok".into(),
                stderr: String::new(),
            },
        );

        let files = vec!["src/lib.rs".into(), "tests/lib.rs".into()];
        let inputs = CommitMessageInputs {
            role: Role::Tester,
            step_index: 1,
            kata_description: "Implement calculator\nMore details",
            agent_commit_message: "test: add failing test\n\nbody",
            notes: "Ensure parser supports plus",
            files_changed: &files,
            plan_path: Path::new(".tdd/plan/step-001-tester.md"),
            runner_outcomes: &runner_summary,
        };

        let message = policy.format(inputs);
        assert!(message.contains("Context:"));
        assert!(message.contains("Diff summary:"));
        assert!(message.contains("Verification:"));
        assert!(message.contains("test: add failing test"));
        assert!(message.contains("src/lib.rs"));
    }
}
