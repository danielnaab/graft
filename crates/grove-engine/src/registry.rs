//! Multi-repository registry with status caching.

use crate::git::count_commits_ahead_of_lock;
use grove_core::{
    EntryDisplayMeta, GitStatus, RefreshStats, RepoPath, RepoRegistry, RepoStatus, Result,
    WorkspaceConfig,
};
use std::collections::HashMap;
use std::path::Path;

/// Minimal lock file representation for dep lock-staleness checks.
#[derive(serde::Deserialize)]
struct SimpleLockFile {
    #[serde(default)]
    dependencies: HashMap<String, SimpleLockEntry>,
}

#[derive(serde::Deserialize)]
struct SimpleLockEntry {
    commit: String,
}

/// Parse locked commits from a `graft.lock` file at `lock_path`.
///
/// Returns an empty map if the file is missing or unparseable (silent degradation).
fn parse_lock_commits(lock_path: &Path) -> HashMap<String, String> {
    let Ok(contents) = std::fs::read_to_string(lock_path) else {
        return HashMap::new();
    };
    serde_yaml::from_str::<SimpleLockFile>(&contents)
        .map(|lf| {
            lf.dependencies
                .into_iter()
                .map(|(k, v)| (k, v.commit))
                .collect()
        })
        .unwrap_or_default()
}

/// Repository registry managing multiple repositories.
#[derive(Debug)]
pub struct WorkspaceRegistry<G> {
    config: WorkspaceConfig,
    git_status: G,
    status_cache: HashMap<RepoPath, RepoStatus>,
    /// Declared repos interleaved with discovered graft dep repos in display order.
    display_order: Vec<RepoPath>,
    /// Display metadata (depth, lock staleness) keyed by repo path.
    entry_meta: HashMap<RepoPath, EntryDisplayMeta>,
}

impl<G: GitStatus> WorkspaceRegistry<G> {
    pub fn new(config: WorkspaceConfig, git_status: G) -> Self {
        // Pre-populate display_order from config so rendering works before first refresh.
        let display_order = config.repositories.iter().map(|r| r.path.clone()).collect();
        Self {
            config,
            git_status,
            status_cache: HashMap::new(),
            display_order,
            entry_meta: HashMap::new(),
        }
    }

    pub fn config(&self) -> &WorkspaceConfig {
        &self.config
    }
}

impl<G: GitStatus> RepoRegistry for WorkspaceRegistry<G> {
    fn list_repos(&self) -> Vec<RepoPath> {
        self.display_order.clone()
    }

    fn get_status(&self, repo_path: &RepoPath) -> Option<&RepoStatus> {
        self.status_cache.get(repo_path)
    }

    fn get_display_meta(&self, repo_path: &RepoPath) -> EntryDisplayMeta {
        self.entry_meta.get(repo_path).cloned().unwrap_or_default()
    }

    fn refresh_all(&mut self) -> Result<RefreshStats> {
        self.status_cache.clear();
        self.display_order.clear();
        self.entry_meta.clear();

        let mut successful = 0;
        let mut failed = 0;

        log::debug!(
            "Refreshing {} repositories...",
            self.config.repositories.len()
        );

        // Collect only paths to free the borrow on self.config during the loop.
        let repo_paths: Vec<RepoPath> = self
            .config
            .repositories
            .iter()
            .map(|r| r.path.clone())
            .collect();

        for repo_path in &repo_paths {
            log::trace!("Querying status for: {repo_path}");

            let status = match self.git_status.get_status(repo_path) {
                Ok(status) => {
                    successful += 1;
                    log::trace!("  ✓ {repo_path}: {:?}", status.branch);
                    status
                }
                Err(e) => {
                    failed += 1;
                    log::warn!("Failed to get status for {repo_path}: {e}");
                    RepoStatus::with_error(repo_path.clone(), e.to_string())
                }
            };

            self.status_cache.insert(repo_path.clone(), status);
            self.display_order.push(repo_path.clone());
            // Declared repos get default metadata (depth=0, no lock staleness).

            // --- Graft dep discovery ---
            let graft_yaml_path = repo_path.as_path().join("graft.yaml");
            let dep_names = std::fs::read_to_string(&graft_yaml_path)
                .ok()
                .and_then(|content| graft_common::parse_dependency_names_from_str(&content).ok())
                .unwrap_or_default();

            if dep_names.is_empty() {
                continue;
            }

            // Parse lock file for this repo (best-effort; empty map if absent).
            let lock_path = repo_path.as_path().join("graft.lock");
            let locked_commits = parse_lock_commits(&lock_path);

            for dep_name in &dep_names {
                let dep_path = repo_path.as_path().join(".graft").join(dep_name);
                if !dep_path.is_dir() {
                    log::trace!("Skipping missing dep directory: {}", dep_path.display());
                    continue;
                }

                let dep_path_str = dep_path.display().to_string();
                let Ok(dep_repo_path) = RepoPath::new(&dep_path_str) else {
                    continue;
                };

                // Dep git-status failures are informational; don't count in declared stats.
                let dep_status = match self.git_status.get_status(&dep_repo_path) {
                    Ok(s) => {
                        log::trace!("  ✓ dep {dep_name}: {:?}", s.branch);
                        s
                    }
                    Err(e) => {
                        log::warn!("Failed to get status for dep {dep_name}: {e}");
                        RepoStatus::with_error(dep_repo_path.clone(), e.to_string())
                    }
                };

                // Compute lock staleness using dep_repo_path (consistently tilde-expanded).
                let ahead_of_lock = locked_commits.get(dep_name).and_then(|commit| {
                    count_commits_ahead_of_lock(dep_repo_path.as_path(), commit)
                });

                self.status_cache.insert(dep_repo_path.clone(), dep_status);
                self.entry_meta.insert(
                    dep_repo_path.clone(),
                    EntryDisplayMeta {
                        depth: 1,
                        ahead_of_lock,
                    },
                );
                self.display_order.push(dep_repo_path);
            }
        }

        log::debug!("Refresh complete: {successful} successful, {failed} failed");

        Ok(RefreshStats { successful, failed })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use grove_core::{CoreError, RepositoryDeclaration, WorkspaceName};

    // Fake git status for testing — returns a clean main-branch status for any non-error path.
    struct FakeGitStatus;

    impl GitStatus for FakeGitStatus {
        fn get_status(&self, repo_path: &RepoPath) -> Result<RepoStatus> {
            if repo_path.as_path().to_str().unwrap().contains("error") {
                Err(CoreError::NotGitRepo {
                    path: repo_path.to_string(),
                })
            } else {
                let mut status = RepoStatus::new(repo_path.clone());
                status.branch = Some("main".to_string());
                Ok(status)
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

        assert!(result.is_ok());

        let repos = registry.list_repos();
        let status = registry.get_status(&repos[0]);

        assert!(status.is_some());
        assert!(status.unwrap().error.is_some());
    }

    #[test]
    fn refresh_stats_count_only_declared_repos() {
        let temp = tempfile::TempDir::new().unwrap();
        let repo_dir = temp.path();

        // A graft.yaml declaring one dep
        std::fs::write(
            repo_dir.join("graft.yaml"),
            "dependencies:\n  my-dep: https://example.com/dep\n",
        )
        .unwrap();
        // The dep directory exists
        std::fs::create_dir_all(repo_dir.join(".graft").join("my-dep")).unwrap();

        let config = WorkspaceConfig {
            name: WorkspaceName::new("test".to_string()).unwrap(),
            repositories: vec![RepositoryDeclaration {
                path: RepoPath::new(repo_dir.to_str().unwrap()).unwrap(),
                tags: vec![],
            }],
        };

        let mut registry = WorkspaceRegistry::new(config, FakeGitStatus);
        let stats = registry.refresh_all().unwrap();

        // Only the one declared repo is counted, not the dep.
        assert_eq!(stats.successful, 1, "Only declared repos counted");
        assert_eq!(stats.failed, 0);
    }

    #[test]
    fn discovers_dep_repos_after_refresh() {
        let temp = tempfile::TempDir::new().unwrap();
        let repo_dir = temp.path();

        std::fs::write(
            repo_dir.join("graft.yaml"),
            "dependencies:\n  my-dep: https://example.com/dep\n",
        )
        .unwrap();
        std::fs::create_dir_all(repo_dir.join(".graft").join("my-dep")).unwrap();

        let config = WorkspaceConfig {
            name: WorkspaceName::new("test".to_string()).unwrap(),
            repositories: vec![RepositoryDeclaration {
                path: RepoPath::new(repo_dir.to_str().unwrap()).unwrap(),
                tags: vec![],
            }],
        };

        let mut registry = WorkspaceRegistry::new(config, FakeGitStatus);
        registry.refresh_all().unwrap();

        let repos = registry.list_repos();
        assert_eq!(repos.len(), 2, "Should include declared repo + 1 dep");

        // Dep appears after its parent.
        let dep_path = repo_dir.join(".graft").join("my-dep");
        let dep_path_str = dep_path.display().to_string();
        assert!(
            repos[1]
                .as_path()
                .display()
                .to_string()
                .contains(&dep_path_str)
                || repos[1].as_path() == dep_path.as_path(),
            "Second entry should be the dep path"
        );
    }

    #[test]
    fn dep_entry_has_depth_one_metadata() {
        let temp = tempfile::TempDir::new().unwrap();
        let repo_dir = temp.path();

        std::fs::write(
            repo_dir.join("graft.yaml"),
            "dependencies:\n  my-dep: https://example.com/dep\n",
        )
        .unwrap();
        std::fs::create_dir_all(repo_dir.join(".graft").join("my-dep")).unwrap();

        let config = WorkspaceConfig {
            name: WorkspaceName::new("test".to_string()).unwrap(),
            repositories: vec![RepositoryDeclaration {
                path: RepoPath::new(repo_dir.to_str().unwrap()).unwrap(),
                tags: vec![],
            }],
        };

        let mut registry = WorkspaceRegistry::new(config, FakeGitStatus);
        registry.refresh_all().unwrap();

        let repos = registry.list_repos();

        // Declared repo: depth 0 (default).
        let meta0 = registry.get_display_meta(&repos[0]);
        assert_eq!(meta0.depth, 0, "Declared repo has depth 0");
        assert!(meta0.ahead_of_lock.is_none());

        // Dep repo: depth 1.
        let meta1 = registry.get_display_meta(&repos[1]);
        assert_eq!(meta1.depth, 1, "Dep repo has depth 1");
    }

    #[test]
    fn skips_missing_dep_directory() {
        let temp = tempfile::TempDir::new().unwrap();
        let repo_dir = temp.path();

        // graft.yaml lists a dep but .graft/missing-dep/ does not exist
        std::fs::write(
            repo_dir.join("graft.yaml"),
            "dependencies:\n  missing-dep: https://example.com/dep\n",
        )
        .unwrap();

        let config = WorkspaceConfig {
            name: WorkspaceName::new("test".to_string()).unwrap(),
            repositories: vec![RepositoryDeclaration {
                path: RepoPath::new(repo_dir.to_str().unwrap()).unwrap(),
                tags: vec![],
            }],
        };

        let mut registry = WorkspaceRegistry::new(config, FakeGitStatus);
        registry.refresh_all().unwrap();

        let repos = registry.list_repos();
        assert_eq!(repos.len(), 1, "Missing dep dir should be skipped");
    }

    #[test]
    fn multiple_deps_all_appended_after_parent() {
        let temp = tempfile::TempDir::new().unwrap();
        let repo_dir = temp.path();

        std::fs::write(
            repo_dir.join("graft.yaml"),
            "dependencies:\n  dep-a: https://example.com/a\n  dep-b: https://example.com/b\n",
        )
        .unwrap();
        std::fs::create_dir_all(repo_dir.join(".graft").join("dep-a")).unwrap();
        std::fs::create_dir_all(repo_dir.join(".graft").join("dep-b")).unwrap();

        let config = WorkspaceConfig {
            name: WorkspaceName::new("test".to_string()).unwrap(),
            repositories: vec![RepositoryDeclaration {
                path: RepoPath::new(repo_dir.to_str().unwrap()).unwrap(),
                tags: vec![],
            }],
        };

        let mut registry = WorkspaceRegistry::new(config, FakeGitStatus);
        registry.refresh_all().unwrap();

        let repos = registry.list_repos();
        assert_eq!(repos.len(), 3, "1 declared + 2 deps");

        let meta1 = registry.get_display_meta(&repos[1]);
        let meta2 = registry.get_display_meta(&repos[2]);
        assert_eq!(meta1.depth, 1);
        assert_eq!(meta2.depth, 1);
    }

    #[test]
    fn parse_lock_commits_returns_empty_on_missing_file() {
        let commits = parse_lock_commits(Path::new("/nonexistent/graft.lock"));
        assert!(commits.is_empty());
    }

    #[test]
    fn parse_lock_commits_parses_valid_lock() {
        use std::io::Write;
        let mut file = tempfile::NamedTempFile::new().unwrap();
        writeln!(
            file,
            r"
dependencies:
  software-factory:
    commit: abc1234
  tools:
    commit: def5678
"
        )
        .unwrap();

        let commits = parse_lock_commits(file.path());
        assert_eq!(
            commits.get("software-factory").map(String::as_str),
            Some("abc1234")
        );
        assert_eq!(commits.get("tools").map(String::as_str), Some("def5678"));
    }
}
