//! JSON edit plan parsing and filesystem application helpers.

use std::{
    collections::HashSet,
    fs,
    path::{Component, Path, PathBuf},
};

use serde::Deserialize;
use thiserror::Error;

/// Structured representation of a plan returned by the LLM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditPlan {
    commit_message: String,
    notes: String,
    files: Vec<FileEdit>,
}

/// Single file edit operation in a plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEdit {
    path: String,
    contents: String,
}

impl EditPlan {
    pub fn commit_message(&self) -> &str {
        &self.commit_message
    }

    pub fn notes(&self) -> &str {
        &self.notes
    }

    pub fn files(&self) -> &[FileEdit] {
        &self.files
    }

    pub fn parse(raw: &str) -> Result<Self, EditPlanError> {
        let sanitized = sanitize_raw_plan(raw);
        let raw_plan: RawEditPlan = serde_json::from_str(&sanitized)?;
        let commit_message = raw_plan
            .commit_message
            .and_then(|value| {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            })
            .ok_or(EditPlanError::MissingCommitMessage)?;
        if raw_plan.files.is_empty() {
            return Err(EditPlanError::EmptyFiles);
        }
        let notes = raw_plan.notes.unwrap_or_default().trim().to_string();
        let mut seen = HashSet::new();
        let mut files = Vec::with_capacity(raw_plan.files.len());
        for file in raw_plan.files {
            let normalized_path = normalize_path(&file.path)?;
            if !seen.insert(normalized_path.clone()) {
                return Err(EditPlanError::DuplicatePath {
                    path: normalized_path,
                });
            }
            files.push(FileEdit {
                path: normalized_path,
                contents: file.contents,
            });
        }
        Ok(Self {
            commit_message,
            notes,
            files,
        })
    }

    pub fn apply(&self, root: &Path) -> Result<Vec<String>, EditPlanError> {
        let mut changed = Vec::with_capacity(self.files.len());
        for file in &self.files {
            let rel = Path::new(&file.path);
            let absolute = root.join(rel);
            if let Some(parent) = absolute.parent() {
                fs::create_dir_all(parent).map_err(|source| EditPlanError::Io {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
            fs::write(&absolute, file.contents.as_bytes()).map_err(|source| EditPlanError::Io {
                path: absolute.clone(),
                source,
            })?;
            changed.push(file.path.clone());
        }
        Ok(changed)
    }
}

impl FileEdit {
    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn contents(&self) -> &str {
        &self.contents
    }
}

#[derive(Debug, Error)]
pub enum EditPlanError {
    #[error("invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error("plan must include at least one file")]
    EmptyFiles,
    #[error("commit_message field is required")]
    MissingCommitMessage,
    #[error("file path {path} is invalid")]
    InvalidPath { path: String },
    #[error("duplicate entry for path {path}")]
    DuplicatePath { path: String },
    #[error("failed to write {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, Deserialize)]
struct RawEditPlan {
    commit_message: Option<String>,
    notes: Option<String>,
    files: Vec<RawFileEdit>,
}

#[derive(Debug, Deserialize)]
struct RawFileEdit {
    path: String,
    contents: String,
}

fn sanitize_raw_plan(raw: &str) -> String {
    let trimmed = raw.trim();
    if !trimmed.starts_with("```") {
        return trimmed.to_string();
    }
    let mut lines: Vec<&str> = trimmed.lines().collect();
    if !lines.is_empty() {
        lines.remove(0);
    }
    if let Some(last) = lines.last() {
        if last.trim_start().starts_with("```") {
            lines.pop();
        }
    }
    lines.join("\n").trim().to_string()
}

fn normalize_path(input: &str) -> Result<String, EditPlanError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(EditPlanError::InvalidPath {
            path: input.to_string(),
        });
    }
    let normalized = trimmed.replace('\\', "/");
    let path = Path::new(&normalized);
    let has_escape = path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir));
    if has_escape {
        return Err(EditPlanError::InvalidPath { path: normalized });
    }
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn parses_basic_plan() {
        let raw = r#"{
			"commit_message": "test: add addition test",
			"notes": "Covers + operator",
			"files": [
				{"path": "tests/math.rs", "contents": "fn main() {}"}
			]
		}"#;
        let plan = EditPlan::parse(raw).expect("expected parse success");
        assert_eq!(plan.commit_message(), "test: add addition test");
        assert_eq!(plan.notes(), "Covers + operator");
        assert_eq!(plan.files().len(), 1);
        assert_eq!(plan.files()[0].path(), "tests/math.rs");
    }

    #[test]
    fn parses_plan_wrapped_in_code_fence() {
        let raw = "```json\n{\n  \"commit_message\": \"refactor: tidy\",\n  \"notes\": \"wrap\",\n  \"files\": [{ \"path\": \"src/lib.rs\", \"contents\": \"pub fn hi() {}\" }]\n}\n```";
        let plan = EditPlan::parse(raw).expect("expected parse success");
        assert_eq!(plan.files()[0].path(), "src/lib.rs");
    }

    #[test]
    fn apply_writes_files_and_returns_paths() {
        let raw = r#"{
			"commit_message": "feat: add module",
			"notes": "",
			"files": [
				{"path": "src/lib.rs", "contents": "pub fn hi() {}"},
				{"path": "tests/lib.rs", "contents": "fn test_hi() {}"}
			]
		}"#;
        let plan = EditPlan::parse(raw).unwrap();
        let dir = tempdir().unwrap();
        let changed = plan.apply(dir.path()).expect("apply succeeds");
        assert_eq!(changed, vec!["src/lib.rs", "tests/lib.rs"]);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("src/lib.rs")).unwrap(),
            "pub fn hi() {}"
        );
    }

    #[test]
    fn parse_rejects_paths_that_escape_root() {
        let raw = r#"{
			"commit_message": "chore: unsafe",
			"notes": "",
			"files": [
				{"path": "../secret.txt", "contents": "bad"}
			]
		}"#;
        let err = EditPlan::parse(raw).unwrap_err();
        matches!(err, EditPlanError::InvalidPath { .. });
    }
}
