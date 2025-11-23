use git2::{Commit, IndexAddOption, Repository, Signature, StatusOptions};
use std::path::Path;
use thiserror::Error;

/// High-level summary of the repository state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoState {
    pub head_commit: Option<String>,
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
        let head_commit = self
            .repo
            .head()
            .ok()
            .and_then(|head| head.target())
            .map(|oid| oid.to_string());

        Ok(RepoState {
            head_commit,
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
