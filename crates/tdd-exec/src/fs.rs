use ignore::WalkBuilder;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// List all non-hidden files inside the workspace honoring gitignore rules.
pub fn list_workspace_files(root: impl AsRef<Path>) -> Result<Vec<PathBuf>, FsError> {
    let root = root.as_ref();
    let mut builder = WalkBuilder::new(root);
    builder.hidden(false).git_ignore(true).git_exclude(true);

    let mut files = Vec::new();
    for entry in builder.build() {
        let entry = entry.map_err(FsError::Walk)?;
        let path = entry.path();
        if path.is_file() {
            files.push(path.to_path_buf());
        }
    }

    Ok(files)
}

#[derive(Debug, Error)]
pub enum FsError {
    #[error("failed to walk workspace: {0}")]
    Walk(ignore::Error),
}
