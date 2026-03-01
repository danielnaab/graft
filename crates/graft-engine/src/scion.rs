//! Scion lifecycle operations.
//!
//! A scion is a named parallel workstream implemented as a git worktree +
//! branch pair. This module applies the graft naming convention:
//! - Worktree path: `.worktrees/<name>` (relative to repo root)
//! - Branch name: `feature/<name>`
//!
//! Git primitives from `graft-common` take explicit paths and branch names.
//! The naming convention lives here, not in the primitives.

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
        let parent_name = components[len - 2]
            .as_os_str()
            .to_str()
            .unwrap_or("");
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

        let (ahead, behind) =
            git_ahead_behind(repo, &branch, &base_branch).unwrap_or((0, 0));

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
}
