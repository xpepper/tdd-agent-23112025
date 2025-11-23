use git2::{Commit, Diff, DiffFormat, IndexAddOption, Repository, Signature, StatusOptions};
use std::path::Path;
use thiserror::Error;

/// High-level summary of the repository state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoState {
    pub head_commit: Option<String>,
    pub last_commit_message: Option<String>,
    pub last_commit_diff: Option<String>,
    pub is_clean: bool,
}

/// Minimal author information required for commits.
#[derive(Debug, Clone)]
pub struct CommitSignature {
    pub name: String,
    pub email: String,
}

impl CommitSignature {
    pub fn new(name: impl Into<String>, email: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            email: email.into(),
        }
    }
}

/// Behavior expected from any version control wrapper.
pub trait Vcs {
    fn ensure_initialized(&self) -> Result<(), VcsError>;
    fn state(&self) -> Result<RepoState, VcsError>;
    fn stage_all(&self) -> Result<(), VcsError>;
    fn commit(&self, message: &str, author: &CommitSignature) -> Result<String, VcsError>;
}

/// `git2`-based implementation.
pub struct GitVcs {
    repo: Repository,
}

impl GitVcs {
    pub fn open_or_init(root: impl AsRef<Path>) -> Result<Self, VcsError> {
        let root = root.as_ref();
        let repo = Repository::open(root).or_else(|_| Repository::init(root))?;
        Ok(Self { repo })
    }

    fn signature(author: &CommitSignature) -> Result<Signature<'static>, VcsError> {
        Signature::now(&author.name, &author.email).map_err(VcsError::Git)
    }

    fn parent_commits(&self) -> Result<Vec<Commit<'_>>, VcsError> {
        match self.repo.head() {
            Ok(reference) => {
                let oid = reference
                    .target()
                    .ok_or_else(|| VcsError::Git(git2::Error::from_str("HEAD has no target")))?;
                let commit = self.repo.find_commit(oid)?;
                Ok(vec![commit])
            }
            Err(_) => Ok(vec![]),
        }
    }

    fn describe_head(&self) -> Result<Option<HeadDescription>, VcsError> {
        let reference = match self.repo.head() {
            Ok(reference) => reference,
            Err(_) => return Ok(None),
        };

        let oid = match reference.target() {
            Some(oid) => oid,
            None => return Ok(None),
        };

        let commit = self.repo.find_commit(oid)?;
        let message = commit
            .message()
            .map(|msg| msg.trim().to_string())
            .filter(|msg| !msg.is_empty());
        let diff = self.commit_diff(&commit)?;

        Ok(Some(HeadDescription {
            id: oid.to_string(),
            message,
            diff,
        }))
    }

    fn commit_diff(&self, commit: &Commit<'_>) -> Result<Option<String>, VcsError> {
        let tree = commit.tree()?;
        let diff = if commit.parent_count() == 0 {
            self.repo.diff_tree_to_tree(None, Some(&tree), None)?
        } else {
            let parent = commit.parent(0)?.tree()?;
            self.repo
                .diff_tree_to_tree(Some(&parent), Some(&tree), None)?
        };

        Ok(Some(Self::format_diff(&diff)?))
    }

    fn format_diff(diff: &Diff<'_>) -> Result<String, VcsError> {
        let mut output = String::new();
        diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
            let line_str = String::from_utf8_lossy(line.content());
            output.push_str(line_str.as_ref());
            true
        })?;
        Ok(output)
    }
}

struct HeadDescription {
    id: String,
    message: Option<String>,
    diff: Option<String>,
}

impl Vcs for GitVcs {
    fn ensure_initialized(&self) -> Result<(), VcsError> {
        if self.repo.is_empty()? {
            // Ensure HEAD exists by writing initial tree if necessary.
            let mut index = self.repo.index()?;
            index.write()?;
        }
        Ok(())
    }

    fn state(&self) -> Result<RepoState, VcsError> {
        let mut opts = StatusOptions::new();
        opts.include_untracked(true);
        let statuses = self.repo.statuses(Some(&mut opts))?;
        let head_description = self.describe_head()?;

        let (head_commit, last_commit_message, last_commit_diff) = match head_description {
            Some(desc) => (Some(desc.id), desc.message, desc.diff),
            None => (None, None, None),
        };

        Ok(RepoState {
            head_commit,
            last_commit_message,
            last_commit_diff,
            is_clean: statuses.is_empty(),
        })
    }

    fn stage_all(&self) -> Result<(), VcsError> {
        let mut index = self.repo.index()?;
        index.add_all(["*"], IndexAddOption::DEFAULT, None)?;
        index.write()?;
        Ok(())
    }

    fn commit(&self, message: &str, author: &CommitSignature) -> Result<String, VcsError> {
        let mut index = self.repo.index()?;
        index.write()?;
        let tree_id = index.write_tree()?;
        let tree = self.repo.find_tree(tree_id)?;
        let parents = self.parent_commits()?;
        let author_sig = Self::signature(author)?;
        let ref_sig = author_sig.clone();

        let parent_refs: Vec<&Commit<'_>> = parents.iter().collect();
        let oid = self
            .repo
            .commit(
                Some("HEAD"),
                &author_sig,
                &ref_sig,
                message,
                &tree,
                &parent_refs,
            )?
            .to_string();
        Ok(oid)
    }
}

#[derive(Debug, Error)]
pub enum VcsError {
    #[error("git error: {0}")]
    Git(#[from] git2::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
