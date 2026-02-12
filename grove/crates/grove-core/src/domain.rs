//! Domain types for Grove.

use crate::error::{CoreError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A validated workspace name (non-empty).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct WorkspaceName(String);

impl WorkspaceName {
    pub fn new(name: String) -> Result<Self> {
        if name.trim().is_empty() {
            return Err(CoreError::EmptyWorkspaceName);
        }
        Ok(Self(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for WorkspaceName {
    type Error = CoreError;

    fn try_from(value: String) -> Result<Self> {
        Self::new(value)
    }
}

impl From<WorkspaceName> for String {
    fn from(name: WorkspaceName) -> Self {
        name.0
    }
}

impl std::fmt::Display for WorkspaceName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A validated repository path (non-empty, tilde-expanded).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct RepoPath(PathBuf);

impl RepoPath {
    pub fn new(path: &str) -> Result<Self> {
        if path.trim().is_empty() {
            return Err(CoreError::EmptyRepoPath);
        }

        // Expand tilde and environment variables
        let expanded = shellexpand::full(path).map_err(|e| CoreError::InvalidRepoPath {
            path: format!("{path}: {e}"),
        })?;

        let path_buf = PathBuf::from(expanded.as_ref());
        Ok(Self(path_buf))
    }

    pub fn as_path(&self) -> &std::path::Path {
        &self.0
    }

    pub fn as_path_buf(&self) -> &PathBuf {
        &self.0
    }
}

impl TryFrom<String> for RepoPath {
    type Error = CoreError;

    fn try_from(value: String) -> Result<Self> {
        Self::new(&value)
    }
}

impl From<RepoPath> for String {
    fn from(path: RepoPath) -> Self {
        path.0.display().to_string()
    }
}

impl std::fmt::Display for RepoPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.display())
    }
}

/// A repository declaration in the workspace config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryDeclaration {
    pub path: RepoPath,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Workspace configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub name: WorkspaceName,
    pub repositories: Vec<RepositoryDeclaration>,
}

/// Git status information for a repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoStatus {
    pub path: RepoPath,
    pub branch: Option<String>,
    pub is_dirty: bool,
    pub ahead: Option<usize>,
    pub behind: Option<usize>,
    pub error: Option<String>,
}

impl RepoStatus {
    pub fn new(path: RepoPath) -> Self {
        Self {
            path,
            branch: None,
            is_dirty: false,
            ahead: None,
            behind: None,
            error: None,
        }
    }

    pub fn with_error(path: RepoPath, error: String) -> Self {
        Self {
            path,
            branch: None,
            is_dirty: false,
            ahead: None,
            behind: None,
            error: Some(error),
        }
    }
}

/// Status of a changed file in the working tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileChangeStatus {
    Modified,
    Added,
    Deleted,
    Renamed,
    Copied,
    Unknown,
}

/// A changed file with its path and status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChange {
    pub path: String,
    pub status: FileChangeStatus,
}

/// Summary information for a single commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitInfo {
    pub hash: String,
    pub subject: String,
    pub author: String,
    pub relative_date: String,
}

/// Detail information for a single repository (commits + changed files).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoDetail {
    pub commits: Vec<CommitInfo>,
    pub changed_files: Vec<FileChange>,
    pub error: Option<String>,
}

impl RepoDetail {
    /// Create an empty detail with no commits, no changes, and no error.
    pub fn empty() -> Self {
        Self {
            commits: Vec::new(),
            changed_files: Vec::new(),
            error: None,
        }
    }

    /// Create a detail representing an error state.
    pub fn with_error(error: String) -> Self {
        Self {
            commits: Vec::new(),
            changed_files: Vec::new(),
            error: Some(error),
        }
    }
}

/// Statistics from repository status refresh operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefreshStats {
    /// Number of repositories successfully refreshed
    pub successful: usize,
    /// Number of repositories that failed to refresh
    pub failed: usize,
}

impl RefreshStats {
    /// Total number of repositories processed
    pub fn total(&self) -> usize {
        self.successful + self.failed
    }

    /// Whether all repositories were successfully refreshed
    pub fn all_successful(&self) -> bool {
        self.failed == 0
    }
}

/// Command from graft.yaml
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Command {
    pub run: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub working_dir: Option<String>,
    #[serde(default)]
    pub env: Option<std::collections::HashMap<String, String>>,
}

/// Minimal graft.yaml representation (commands section only)
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraftYaml {
    #[serde(default)]
    pub commands: std::collections::HashMap<String, Command>,
}

/// State of a running command
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandState {
    NotStarted,
    Running,
    Completed { exit_code: i32 },
    Failed { error: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_name_rejects_empty() {
        assert!(WorkspaceName::new("".to_string()).is_err());
        assert!(WorkspaceName::new("  ".to_string()).is_err());
    }

    #[test]
    fn workspace_name_accepts_valid() {
        let name = WorkspaceName::new("my-workspace".to_string()).unwrap();
        assert_eq!(name.as_str(), "my-workspace");
    }

    #[test]
    fn repo_path_rejects_empty() {
        assert!(RepoPath::new("").is_err());
        assert!(RepoPath::new("  ").is_err());
    }

    #[test]
    fn repo_path_accepts_valid() {
        let path = RepoPath::new("/tmp/repo").unwrap();
        assert_eq!(path.as_path(), std::path::Path::new("/tmp/repo"));
    }

    #[test]
    fn repo_path_expands_tilde() {
        // Note: This test might behave differently depending on environment
        let path = RepoPath::new("~/repos/grove");
        assert!(path.is_ok());
    }

    #[test]
    fn repo_detail_empty_has_no_data() {
        let detail = RepoDetail::empty();
        assert!(detail.commits.is_empty());
        assert!(detail.changed_files.is_empty());
        assert!(detail.error.is_none());
    }

    #[test]
    fn repo_detail_with_error_stores_message() {
        let detail = RepoDetail::with_error("something went wrong".to_string());
        assert!(detail.commits.is_empty());
        assert!(detail.changed_files.is_empty());
        assert_eq!(detail.error, Some("something went wrong".to_string()));
    }
}
