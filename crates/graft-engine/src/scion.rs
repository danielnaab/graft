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
use graft_common::{git_branch_delete, git_worktree_add, git_worktree_remove};
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
}
