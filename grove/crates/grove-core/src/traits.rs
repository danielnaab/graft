//! Trait definitions (ports) for Grove.

use crate::domain::{GraftYaml, RefreshStats, RepoDetail, RepoPath, RepoStatus, WorkspaceConfig};
use crate::error::Result;

/// Capability to load workspace configuration.
pub trait ConfigLoader {
    /// Load workspace configuration from the specified path.
    fn load_workspace(&self, config_path: &str) -> Result<WorkspaceConfig>;
}

/// Capability to query git repository status.
pub trait GitStatus {
    /// Get the git status for a repository at the specified path.
    fn get_status(&self, repo_path: &RepoPath) -> Result<RepoStatus>;
}

/// Capability to query detailed information for a single repository.
pub trait RepoDetailProvider {
    /// Get detail (recent commits + changed files) for a repository.
    fn get_detail(&self, path: &RepoPath, max_commits: usize) -> Result<RepoDetail>;
}

/// Repository registry managing multiple repositories.
pub trait RepoRegistry {
    /// List all configured repositories.
    fn list_repos(&self) -> Vec<RepoPath>;

    /// Get the cached status for a repository.
    fn get_status(&self, repo_path: &RepoPath) -> Option<&RepoStatus>;

    /// Refresh status for all repositories.
    ///
    /// Returns statistics about the refresh operation (successful/failed counts).
    fn refresh_all(&mut self) -> Result<RefreshStats>;
}

/// Capability to load graft.yaml files.
pub trait GraftYamlLoader {
    /// Load and parse graft.yaml from path.
    fn load_graft(&self, graft_path: &str) -> Result<GraftYaml>;
}
