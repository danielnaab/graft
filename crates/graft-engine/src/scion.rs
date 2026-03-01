//! Scion lifecycle operations.
//!
//! A scion is a named parallel workstream implemented as a git worktree +
//! branch pair. This module applies the graft naming convention:
//! - Worktree path: `.worktrees/<name>` (relative to repo root)
//! - Branch name: `feature/<name>`
//!
//! Git primitives from `graft-common` take explicit paths and branch names.
//! The naming convention lives here, not in the primitives.

use crate::domain::{GraftConfig, ScionHooks};
use crate::error::{GraftError, Result};
use graft_common::{
    git_ahead_behind, git_branch_delete, git_is_dirty, git_last_commit_time, git_worktree_add,
    git_worktree_list, git_worktree_remove,
};
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Compute the worktree path for a scion: `<repo_root>/.worktrees/<name>`.
fn worktree_path(repo: &Path, name: &str) -> PathBuf {
    repo.join(".worktrees").join(name)
}

/// Compute the branch name for a scion: `feature/<name>`.
fn branch_name(name: &str) -> String {
    format!("feature/{name}")
}

/// Create a new scion (worktree + branch) for the given name.
///
/// Applies the naming convention:
/// - Worktree: `<repo_root>/.worktrees/<name>`
/// - Branch: `feature/<name>`
///
/// # Arguments
/// * `repo_path` - Absolute path to the main git repository
/// * `name`      - Scion name (e.g. `my-feature`)
///
/// # Returns
/// Absolute path to the newly created worktree.
///
/// # Errors
/// Returns `GraftError` if the worktree or branch already exists, or the
/// underlying git operation fails.
pub fn scion_create(repo_path: impl AsRef<Path>, name: &str) -> Result<PathBuf> {
    let repo = repo_path.as_ref();
    let path = worktree_path(repo, name);
    let branch = branch_name(name);
    git_worktree_add(repo, &path, &branch).map_err(GraftError::from)?;
    Ok(path)
}

/// Remove a scion (worktree + branch) by name.
///
/// Applies the naming convention:
/// - Worktree: `<repo_root>/.worktrees/<name>`
/// - Branch: `feature/<name>`
///
/// The worktree is removed first, then the branch.
///
/// # Arguments
/// * `repo_path` - Absolute path to the main git repository
/// * `name`      - Scion name (e.g. `my-feature`)
///
/// # Errors
/// Returns `GraftError` if the worktree or branch does not exist, or the
/// underlying git operation fails.
pub fn scion_prune(repo_path: impl AsRef<Path>, name: &str) -> Result<()> {
    let repo = repo_path.as_ref();
    let path = worktree_path(repo, name);
    let branch = branch_name(name);
    git_worktree_remove(repo, &path).map_err(GraftError::from)?;
    git_branch_delete(repo, &branch).map_err(GraftError::from)?;
    Ok(())
}

/// A scion lifecycle event that triggers hooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookEvent {
    /// After worktree + branch creation.
    OnCreate,
    /// Before merging the feature branch to main.
    PreFuse,
    /// After the merge commit is applied to main.
    PostFuse,
    /// Before worktree + branch removal.
    OnPrune,
}

/// A resolved hook ready for execution.
///
/// Produced by `resolve_hook_chain`. Each entry carries enough context to
/// locate the command definition and set up the execution environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedHook {
    /// Command name as defined in a graft.yaml `commands:` section.
    /// For dependency hooks this is qualified: `dep_name:command_name`.
    /// For project hooks this is unqualified: `command_name`.
    pub command_name: String,
    /// Namespace: dependency name, or `None` for project-level hooks.
    pub namespace: Option<String>,
    /// Working directory for hook execution.
    pub working_dir: PathBuf,
}

/// Extract the hook command list for the given event from `ScionHooks`.
fn hooks_for_event(hooks: &ScionHooks, event: HookEvent) -> Option<&Vec<String>> {
    match event {
        HookEvent::OnCreate => hooks.on_create.as_ref(),
        HookEvent::PreFuse => hooks.pre_fuse.as_ref(),
        HookEvent::PostFuse => hooks.post_fuse.as_ref(),
        HookEvent::OnPrune => hooks.on_prune.as_ref(),
    }
}

/// Determine the working directory for a hook based on the lifecycle event.
///
/// - `on_create`, `pre_fuse`, `on_prune` → scion worktree (the work happens there)
/// - `post_fuse` → project root (the worktree may be about to be removed)
fn working_dir_for_event(event: HookEvent, scion_worktree: &Path, project_root: &Path) -> PathBuf {
    match event {
        HookEvent::OnCreate | HookEvent::PreFuse | HookEvent::OnPrune => {
            scion_worktree.to_path_buf()
        }
        HookEvent::PostFuse => project_root.to_path_buf(),
    }
}

/// Resolve the ordered chain of hooks for a scion lifecycle event.
///
/// Dependencies' hooks run first (in the order provided), then the project's
/// own hooks. Within each scope, hooks run in list order.
///
/// # Arguments
/// * `event`           - The lifecycle event to resolve hooks for
/// * `config`          - The project's `GraftConfig` (for project-level hooks)
/// * `dep_configs`     - Dependency configs in declaration order: `(name, config)`
/// * `scion_worktree`  - Absolute path to the scion's worktree
/// * `project_root`    - Absolute path to the project root
///
/// # Returns
/// Ordered list of `ResolvedHook`s. Empty if no hooks are defined.
pub fn resolve_hook_chain(
    event: HookEvent,
    config: &GraftConfig,
    dep_configs: &[(String, GraftConfig)],
    scion_worktree: &Path,
    project_root: &Path,
) -> Vec<ResolvedHook> {
    let working_dir = working_dir_for_event(event, scion_worktree, project_root);
    let mut chain = Vec::new();

    // Dependencies first, in declaration order
    for (dep_name, dep_config) in dep_configs {
        if let Some(hooks) = &dep_config.scion_hooks {
            if let Some(cmds) = hooks_for_event(hooks, event) {
                for cmd in cmds {
                    chain.push(ResolvedHook {
                        command_name: format!("{dep_name}:{cmd}"),
                        namespace: Some(dep_name.clone()),
                        working_dir: working_dir.clone(),
                    });
                }
            }
        }
    }

    // Project hooks last
    if let Some(hooks) = &config.scion_hooks {
        if let Some(cmds) = hooks_for_event(hooks, event) {
            for cmd in cmds {
                chain.push(ResolvedHook {
                    command_name: cmd.clone(),
                    namespace: None,
                    working_dir: working_dir.clone(),
                });
            }
        }
    }

    chain
}

/// Structured information about a scion workstream.
///
/// Returned by `scion_list`. All fields are derived from git artifacts —
/// no worker registry or heartbeat is needed.
#[derive(Debug, Clone, Serialize)]
pub struct ScionInfo {
    /// Scion name (e.g. `my-feature`).
    pub name: String,
    /// Full branch name (e.g. `feature/my-feature`).
    pub branch: String,
    /// Absolute path to the worktree directory.
    pub worktree_path: PathBuf,
    /// Commits in the scion branch not yet in main.
    pub ahead: usize,
    /// Commits in main not yet in the scion branch.
    pub behind: usize,
    /// Unix timestamp of the most recent commit on the scion branch.
    /// `None` if the scion has no commits (freshly created).
    pub last_commit_time: Option<i64>,
    /// Whether the worktree has uncommitted changes.
    pub dirty: bool,
}

/// List all scions for the repository.
///
/// Enumerates worktrees whose paths fall under `.worktrees/`, extracts the
/// scion name from the path component, and gathers per-scion metrics.
///
/// # Arguments
/// * `repo_path` - Absolute path to the main git repository
///
/// # Returns
/// A list of `ScionInfo` structs, one per scion (in the order returned by
/// `git worktree list`). The main worktree is excluded.
///
/// # Errors
/// Returns `GraftError` if the worktree enumeration fails.
pub fn scion_list(repo_path: impl AsRef<Path>) -> Result<Vec<ScionInfo>> {
    let repo = repo_path.as_ref();
    let worktrees = git_worktree_list(repo).map_err(GraftError::from)?;

    // The first entry is always the main worktree; its branch is our base.
    let base_branch = worktrees
        .first()
        .and_then(|w| w.branch.clone())
        .unwrap_or_else(|| "main".to_string());

    let mut scions = Vec::new();
    for wt in &worktrees {
        // Filter to .worktrees/ entries only
        let components: Vec<_> = wt.path.components().collect();
        let len = components.len();
        if len < 2 {
            continue;
        }
        // The parent directory must be named ".worktrees"
        let parent_name = components[len - 2].as_os_str().to_str().unwrap_or("");
        if parent_name != ".worktrees" {
            continue;
        }

        let scion_name = components[len - 1]
            .as_os_str()
            .to_str()
            .unwrap_or("")
            .to_string();
        if scion_name.is_empty() {
            continue;
        }

        let branch = branch_name(&scion_name);

        let (ahead, behind) = git_ahead_behind(repo, &branch, &base_branch).unwrap_or((0, 0));

        let last_commit_time = git_last_commit_time(repo, &branch).ok();

        let dirty = git_is_dirty(&wt.path).unwrap_or(false);

        scions.push(ScionInfo {
            name: scion_name,
            branch,
            worktree_path: wt.path.clone(),
            ahead,
            behind,
            last_commit_time,
            dirty,
        });
    }

    Ok(scions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    fn init_test_repo(path: &Path) {
        Command::new("git")
            .args(["init"])
            .current_dir(path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(path)
            .output()
            .unwrap();
        fs::write(path.join("README.md"), "test").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(path)
            .output()
            .unwrap();
    }

    #[test]
    fn scion_create_creates_worktree_and_branch() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());

        let wt_path = scion_create(temp.path(), "my-feature").unwrap();

        // Worktree directory exists at expected path
        assert_eq!(wt_path, temp.path().join(".worktrees").join("my-feature"));
        assert!(wt_path.exists());

        // Confirm the branch name via worktree list
        let worktrees = graft_common::git_worktree_list(temp.path()).unwrap();
        let scion_wt = worktrees
            .iter()
            .find(|w| w.branch.as_deref() == Some("feature/my-feature"))
            .expect("scion worktree not found");
        assert!(scion_wt.path.ends_with(".worktrees/my-feature"));
    }

    #[test]
    fn scion_create_fails_if_already_exists() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());

        scion_create(temp.path(), "dup").unwrap();
        let result = scion_create(temp.path(), "dup");
        assert!(result.is_err());
    }

    #[test]
    fn scion_prune_removes_worktree_and_branch() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());

        scion_create(temp.path(), "to-prune").unwrap();
        scion_prune(temp.path(), "to-prune").unwrap();

        // Worktree gone from list
        let worktrees = graft_common::git_worktree_list(temp.path()).unwrap();
        assert!(!worktrees
            .iter()
            .any(|w| w.branch.as_deref() == Some("feature/to-prune")));

        // Branch gone (attempting to delete again should fail)
        let del = graft_common::git_branch_delete(temp.path(), "feature/to-prune");
        assert!(del.is_err());
    }

    #[test]
    fn scion_prune_fails_for_nonexistent() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());

        let result = scion_prune(temp.path(), "does-not-exist");
        assert!(result.is_err());
    }

    #[test]
    fn scion_create_then_prune_round_trip() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());

        let wt = scion_create(temp.path(), "round-trip").unwrap();
        assert!(wt.exists());

        scion_prune(temp.path(), "round-trip").unwrap();
        assert!(!wt.exists());
    }

    fn make_commit(path: &Path, filename: &str, message: &str) {
        fs::write(path.join(filename), "content").unwrap();
        Command::new("git")
            .args(["add", filename])
            .current_dir(path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(path)
            .output()
            .unwrap();
    }

    #[test]
    fn scion_list_empty_returns_empty_vec() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());

        let scions = scion_list(temp.path()).unwrap();
        assert!(scions.is_empty());
    }

    #[test]
    fn scion_list_returns_created_scion() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());

        scion_create(temp.path(), "alpha").unwrap();
        let scions = scion_list(temp.path()).unwrap();

        assert_eq!(scions.len(), 1);
        let s = &scions[0];
        assert_eq!(s.name, "alpha");
        assert_eq!(s.branch, "feature/alpha");
        assert!(s.worktree_path.ends_with(".worktrees/alpha"));
        assert_eq!(s.ahead, 0);
        assert_eq!(s.behind, 0);
        assert!(!s.dirty);
    }

    #[test]
    fn scion_list_shows_commits_ahead() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());

        scion_create(temp.path(), "beta").unwrap();
        let wt_path = temp.path().join(".worktrees").join("beta");

        // Make 2 commits in the scion worktree
        make_commit(&wt_path, "a.txt", "feat: a");
        make_commit(&wt_path, "b.txt", "feat: b");

        let scions = scion_list(temp.path()).unwrap();
        let s = scions.iter().find(|s| s.name == "beta").unwrap();
        assert_eq!(s.ahead, 2);
        assert_eq!(s.behind, 0);
        // last_commit_time should be set now that we have commits
        assert!(s.last_commit_time.is_some());
    }

    #[test]
    fn scion_list_shows_dirty_worktree() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path());

        scion_create(temp.path(), "gamma").unwrap();
        let wt_path = temp.path().join(".worktrees").join("gamma");

        // Write a file without committing
        fs::write(wt_path.join("dirty.txt"), "uncommitted").unwrap();

        let scions = scion_list(temp.path()).unwrap();
        let s = scions.iter().find(|s| s.name == "gamma").unwrap();
        assert!(s.dirty);
    }

    // --- resolve_hook_chain tests ---

    fn make_config_with_hooks(hooks: ScionHooks) -> GraftConfig {
        let mut config = GraftConfig::new("graft/v1").unwrap();
        config.scion_hooks = Some(hooks);
        config
    }

    #[test]
    fn resolve_hook_chain_no_hooks() {
        let config = GraftConfig::new("graft/v1").unwrap();
        let chain = resolve_hook_chain(
            HookEvent::OnCreate,
            &config,
            &[],
            Path::new("/wt"),
            Path::new("/root"),
        );
        assert!(chain.is_empty());
    }

    #[test]
    fn resolve_hook_chain_project_only() {
        let config = make_config_with_hooks(ScionHooks {
            on_create: Some(vec!["setup".to_string(), "init".to_string()]),
            pre_fuse: None,
            post_fuse: None,
            on_prune: None,
        });
        let chain = resolve_hook_chain(
            HookEvent::OnCreate,
            &config,
            &[],
            Path::new("/wt"),
            Path::new("/root"),
        );
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].command_name, "setup");
        assert!(chain[0].namespace.is_none());
        assert_eq!(chain[1].command_name, "init");
        assert!(chain[1].namespace.is_none());
    }

    #[test]
    fn resolve_hook_chain_dep_only() {
        let config = GraftConfig::new("graft/v1").unwrap();
        let dep = make_config_with_hooks(ScionHooks {
            on_create: Some(vec!["dep-setup".to_string()]),
            pre_fuse: None,
            post_fuse: None,
            on_prune: None,
        });
        let chain = resolve_hook_chain(
            HookEvent::OnCreate,
            &config,
            &[("my-dep".to_string(), dep)],
            Path::new("/wt"),
            Path::new("/root"),
        );
        assert_eq!(chain.len(), 1);
        assert_eq!(chain[0].command_name, "my-dep:dep-setup");
        assert_eq!(chain[0].namespace.as_deref(), Some("my-dep"));
    }

    #[test]
    fn resolve_hook_chain_mixed_deps_then_project() {
        let config = make_config_with_hooks(ScionHooks {
            on_create: Some(vec!["project-setup".to_string()]),
            pre_fuse: None,
            post_fuse: None,
            on_prune: None,
        });
        let dep_a = make_config_with_hooks(ScionHooks {
            on_create: Some(vec!["a-init".to_string()]),
            pre_fuse: None,
            post_fuse: None,
            on_prune: None,
        });
        let dep_b = make_config_with_hooks(ScionHooks {
            on_create: Some(vec!["b-init".to_string(), "b-check".to_string()]),
            pre_fuse: None,
            post_fuse: None,
            on_prune: None,
        });
        let chain = resolve_hook_chain(
            HookEvent::OnCreate,
            &config,
            &[("dep-a".to_string(), dep_a), ("dep-b".to_string(), dep_b)],
            Path::new("/wt"),
            Path::new("/root"),
        );
        assert_eq!(chain.len(), 4);
        // Deps first in order, then project
        assert_eq!(chain[0].command_name, "dep-a:a-init");
        assert_eq!(chain[1].command_name, "dep-b:b-init");
        assert_eq!(chain[2].command_name, "dep-b:b-check");
        assert_eq!(chain[3].command_name, "project-setup");
    }

    #[test]
    fn resolve_hook_chain_working_dir_worktree_events() {
        let config = make_config_with_hooks(ScionHooks {
            on_create: Some(vec!["a".to_string()]),
            pre_fuse: Some(vec!["b".to_string()]),
            post_fuse: None,
            on_prune: Some(vec!["c".to_string()]),
        });
        let wt = Path::new("/worktree");
        let root = Path::new("/root");

        // on_create → worktree
        let chain = resolve_hook_chain(HookEvent::OnCreate, &config, &[], wt, root);
        assert_eq!(chain[0].working_dir, wt);

        // pre_fuse → worktree
        let chain = resolve_hook_chain(HookEvent::PreFuse, &config, &[], wt, root);
        assert_eq!(chain[0].working_dir, wt);

        // on_prune → worktree
        let chain = resolve_hook_chain(HookEvent::OnPrune, &config, &[], wt, root);
        assert_eq!(chain[0].working_dir, wt);
    }

    #[test]
    fn resolve_hook_chain_working_dir_post_fuse_uses_project_root() {
        let config = make_config_with_hooks(ScionHooks {
            on_create: None,
            pre_fuse: None,
            post_fuse: Some(vec!["notify".to_string()]),
            on_prune: None,
        });
        let chain = resolve_hook_chain(
            HookEvent::PostFuse,
            &config,
            &[],
            Path::new("/worktree"),
            Path::new("/root"),
        );
        assert_eq!(chain[0].working_dir, Path::new("/root"));
    }
}
