//! Multi-repository registry with status caching.

use grove_core::{
    GitStatus, RefreshStats, RepoPath, RepoRegistry, RepoStatus, Result, WorkspaceConfig,
};
use std::collections::HashMap;

/// Repository registry managing multiple repositories.
#[derive(Debug)]
pub struct WorkspaceRegistry<G> {
    config: WorkspaceConfig,
    git_status: G,
    status_cache: HashMap<RepoPath, RepoStatus>,
}

impl<G: GitStatus> WorkspaceRegistry<G> {
    pub fn new(config: WorkspaceConfig, git_status: G) -> Self {
        Self {
            config,
            git_status,
            status_cache: HashMap::new(),
        }
    }

    pub fn config(&self) -> &WorkspaceConfig {
        &self.config
    }
}

impl<G: GitStatus> RepoRegistry for WorkspaceRegistry<G> {
    fn list_repos(&self) -> Vec<RepoPath> {
        self.config
            .repositories
            .iter()
            .map(|repo| repo.path.clone())
            .collect()
    }

    fn get_status(&self, repo_path: &RepoPath) -> Option<&RepoStatus> {
        self.status_cache.get(repo_path)
    }

    fn refresh_all(&mut self) -> Result<RefreshStats> {
        self.status_cache.clear();

        let mut successful = 0;
        let mut failed = 0;

        log::debug!(
            "Refreshing {} repositories...",
            self.config.repositories.len()
        );

        for repo_decl in &self.config.repositories {
            log::trace!("Querying status for: {}", repo_decl.path);

            let status = match self.git_status.get_status(&repo_decl.path) {
                Ok(status) => {
                    successful += 1;
                    log::trace!("  ✓ {}: {:?}", repo_decl.path, status.branch);
                    status
                }
                Err(e) => {
                    failed += 1;
                    // Graceful degradation: log error and store error status
                    log::warn!("Failed to get status for {}: {}", repo_decl.path, e);
                    RepoStatus::with_error(repo_decl.path.clone(), e.to_string())
                }
            };

            self.status_cache.insert(repo_decl.path.clone(), status);
        }

        log::debug!(
            "Refresh complete: {} successful, {} failed",
            successful,
            failed
        );

        Ok(RefreshStats { successful, failed })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use grove_core::{CoreError, RepositoryDeclaration, WorkspaceName};

    // Fake git status for testing
    struct FakeGitStatus;

    impl GitStatus for FakeGitStatus {
        fn get_status(&self, repo_path: &RepoPath) -> Result<RepoStatus> {
            // Return a fake status or error based on path
            if repo_path.as_path().to_str().unwrap().contains("error") {
                Err(CoreError::NotGitRepo {
                    path: repo_path.to_string(),
                })
            } else {
                Ok(RepoStatus {
                    path: repo_path.clone(),
                    branch: Some("main".to_string()),
                    is_dirty: false,
                    ahead: None,
                    behind: None,
                    error: None,
                })
            }
        }
    }

    #[test]
    fn lists_repos_from_config() {
        let config = WorkspaceConfig {
            name: WorkspaceName::new("test".to_string()).unwrap(),
            repositories: vec![
                RepositoryDeclaration {
                    path: RepoPath::new("/tmp/repo1").unwrap(),
                    tags: vec![],
                },
                RepositoryDeclaration {
                    path: RepoPath::new("/tmp/repo2").unwrap(),
                    tags: vec![],
                },
            ],
        };

        let registry = WorkspaceRegistry::new(config, FakeGitStatus);
        let repos = registry.list_repos();

        assert_eq!(repos.len(), 2);
    }

    #[test]
    fn refresh_all_updates_cache() {
        let config = WorkspaceConfig {
            name: WorkspaceName::new("test".to_string()).unwrap(),
            repositories: vec![RepositoryDeclaration {
                path: RepoPath::new("/tmp/repo1").unwrap(),
                tags: vec![],
            }],
        };

        let mut registry = WorkspaceRegistry::new(config, FakeGitStatus);
        registry.refresh_all().unwrap();

        let repos = registry.list_repos();
        let status = registry.get_status(&repos[0]);

        assert!(status.is_some());
        assert_eq!(status.unwrap().branch, Some("main".to_string()));
    }

    #[test]
    fn refresh_all_handles_errors_gracefully() {
        let config = WorkspaceConfig {
            name: WorkspaceName::new("test".to_string()).unwrap(),
            repositories: vec![RepositoryDeclaration {
                path: RepoPath::new("/tmp/error-repo").unwrap(),
                tags: vec![],
            }],
        };

        let mut registry = WorkspaceRegistry::new(config, FakeGitStatus);
        let result = registry.refresh_all();

        // Should succeed even with errors (graceful degradation)
        assert!(result.is_ok());

        let repos = registry.list_repos();
        let status = registry.get_status(&repos[0]);

        assert!(status.is_some());
        assert!(status.unwrap().error.is_some());
    }
}
