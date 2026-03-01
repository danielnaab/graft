//! Git operations with timeout protection.
//!
//! This module provides shared git primitives used by both graft and grove.
//! All operations apply a 30-second default timeout to prevent hangs on network
//! or I/O issues. The `GRAFT_PROCESS_TIMEOUT_MS` environment variable overrides
//! this default when set.

use crate::process::{run_to_completion_with_timeout, ProcessConfig, ProcessError};
use std::path::Path;
use std::time::Duration;

const GIT_DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Error type for git operations.
#[derive(thiserror::Error, Debug)]
pub enum GitError {
    #[error("Git command failed: {0}")]
    CommandFailed(String),

    #[error("Process execution error: {0}")]
    Process(#[from] ProcessError),
}

/// Check if a path is a git repository.
///
/// Returns `true` if the path has a `.git` directory or file (for submodules).
pub fn is_git_repo(path: impl AsRef<Path>) -> bool {
    path.as_ref().join(".git").exists()
}

/// Get the current commit hash from a git repository.
///
/// Runs `git rev-parse HEAD` in the repository directory.
///
/// # Arguments
/// * `path` - Path to the git repository
///
/// # Errors
/// Returns an error if the git command fails or the repository is in an invalid state.
pub fn get_current_commit(path: impl AsRef<Path>) -> Result<String, GitError> {
    let path = path.as_ref();
    let config = ProcessConfig {
        command: "git rev-parse HEAD".to_string(),
        working_dir: path.to_path_buf(),
        env: None,
        env_remove: vec![],
        log_path: None,
        timeout: Some(Duration::from_secs(GIT_DEFAULT_TIMEOUT_SECS)),
        stdin: None,
    };
    let output = run_to_completion_with_timeout(&config)?;
    if !output.success {
        return Err(GitError::CommandFailed(format!(
            "git rev-parse HEAD failed: {}",
            output.stderr
        )));
    }
    Ok(output.stdout.trim().to_string())
}

/// Resolve a git ref to a commit hash.
///
/// Tries to resolve the ref in the following order:
/// 1. `origin/<ref>` (for remote branches)
/// 2. `<ref>` (for local branches, tags, or commit hashes)
///
/// # Arguments
/// * `path` - Path to the git repository
/// * `git_ref` - The git reference to resolve (branch, tag, or commit hash)
///
/// # Errors
/// Returns an error if the ref cannot be resolved.
pub fn git_rev_parse(path: impl AsRef<Path>, git_ref: &str) -> Result<String, GitError> {
    let path = path.as_ref();

    // Try origin/<ref> first for branches
    let refs_to_try = vec![format!("origin/{git_ref}"), git_ref.to_string()];

    for ref_name in refs_to_try {
        let config = ProcessConfig {
            command: format!("git rev-parse {ref_name}"),
            working_dir: path.to_path_buf(),
            env: None,
            env_remove: vec![],
            log_path: None,
            timeout: Some(Duration::from_secs(GIT_DEFAULT_TIMEOUT_SECS)),
            stdin: None,
        };
        let output = run_to_completion_with_timeout(&config)?;
        if output.success {
            return Ok(output.stdout.trim().to_string());
        }
    }

    Err(GitError::CommandFailed(format!(
        "Could not resolve ref: {git_ref}"
    )))
}

/// Fetch all refs from remote.
///
/// Runs `git fetch --all` to update remote refs.
///
/// # Arguments
/// * `path` - Path to the git repository
///
/// # Errors
/// Returns an error if the git command fails.
pub fn git_fetch(path: impl AsRef<Path>) -> Result<(), GitError> {
    let path = path.as_ref();
    let config = ProcessConfig {
        command: "git fetch --all".to_string(),
        working_dir: path.to_path_buf(),
        env: None,
        env_remove: vec![],
        log_path: None,
        timeout: Some(Duration::from_secs(GIT_DEFAULT_TIMEOUT_SECS)),
        stdin: None,
    };
    let output = run_to_completion_with_timeout(&config)?;
    if !output.success {
        return Err(GitError::CommandFailed(format!(
            "git fetch failed: {}",
            output.stderr
        )));
    }
    Ok(())
}

/// Information about a git worktree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeInfo {
    /// Absolute path to the worktree directory.
    pub path: std::path::PathBuf,
    /// Branch checked out in this worktree, or `None` for a detached HEAD.
    pub branch: Option<String>,
    /// The HEAD commit hash.
    pub head: String,
}

/// Parse the output of `git worktree list --porcelain` into a list of `WorktreeInfo`.
fn parse_worktree_list(output: &str) -> Result<Vec<WorktreeInfo>, GitError> {
    let mut result = Vec::new();
    // Stanzas are separated by blank lines
    for stanza in output.split("\n\n") {
        let stanza = stanza.trim();
        if stanza.is_empty() {
            continue;
        }
        let mut path: Option<std::path::PathBuf> = None;
        let mut head: Option<String> = None;
        let mut branch: Option<String> = None;

        for line in stanza.lines() {
            if let Some(p) = line.strip_prefix("worktree ") {
                path = Some(std::path::PathBuf::from(p.trim()));
            } else if let Some(h) = line.strip_prefix("HEAD ") {
                head = Some(h.trim().to_string());
            } else if let Some(b) = line.strip_prefix("branch ") {
                let b = b.trim();
                // "refs/heads/feature/foo" -> "feature/foo"
                let name = b.strip_prefix("refs/heads/").unwrap_or(b).to_string();
                branch = Some(name);
            }
            // "detached", "locked", "prunable" lines are intentionally skipped
        }

        match (path, head) {
            (Some(p), Some(h)) => result.push(WorktreeInfo {
                path: p,
                branch,
                head: h,
            }),
            _ => {
                return Err(GitError::CommandFailed(format!(
                    "Failed to parse worktree stanza: {stanza}"
                )));
            }
        }
    }
    Ok(result)
}

/// List all git worktrees for the repository.
///
/// Runs `git worktree list --porcelain` and parses the output into a structured
/// list. The first entry is always the main worktree.
///
/// # Arguments
/// * `repo` - Path to the git repository
///
/// # Errors
/// Returns an error if the git command fails or the output cannot be parsed.
pub fn git_worktree_list(repo: impl AsRef<Path>) -> Result<Vec<WorktreeInfo>, GitError> {
    let repo = repo.as_ref();
    let config = ProcessConfig {
        command: "git worktree list --porcelain".to_string(),
        working_dir: repo.to_path_buf(),
        env: None,
        env_remove: vec![],
        log_path: None,
        timeout: Some(Duration::from_secs(GIT_DEFAULT_TIMEOUT_SECS)),
        stdin: None,
    };
    let output = run_to_completion_with_timeout(&config)?;
    if !output.success {
        return Err(GitError::CommandFailed(format!(
            "git worktree list failed: {}",
            output.stderr
        )));
    }
    parse_worktree_list(&output.stdout)
}

/// Create a new git worktree at the given path on a new branch.
///
/// Runs `git worktree add <path> -b <branch>`. The branch must not already exist,
/// and the path must not already be registered as a worktree.
///
/// # Arguments
/// * `repo`   - Path to the main git repository
/// * `path`   - Where to create the worktree (relative or absolute)
/// * `branch` - Name of the new branch to create in the worktree
///
/// # Returns
/// The canonicalized absolute path to the new worktree.
///
/// # Errors
/// Returns `GitError` if the worktree path or branch already exists, or the git
/// command fails for any other reason.
pub fn git_worktree_add(
    repo: impl AsRef<Path>,
    path: impl AsRef<Path>,
    branch: &str,
) -> Result<std::path::PathBuf, GitError> {
    let repo = repo.as_ref();
    let path = path.as_ref();
    let path_str = path
        .to_str()
        .ok_or_else(|| GitError::CommandFailed("worktree path is not valid UTF-8".to_string()))?;
    let config = ProcessConfig {
        command: format!("git worktree add {path_str} -b {branch}"),
        working_dir: repo.to_path_buf(),
        env: None,
        env_remove: vec![],
        log_path: None,
        timeout: Some(Duration::from_secs(GIT_DEFAULT_TIMEOUT_SECS)),
        stdin: None,
    };
    let output = run_to_completion_with_timeout(&config)?;
    if !output.success {
        return Err(GitError::CommandFailed(format!(
            "git worktree add failed: {}",
            output.stderr
        )));
    }
    // Resolve the absolute path (the caller may have passed a relative path)
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo.join(path)
    };
    Ok(abs)
}

/// Remove a git worktree.
///
/// Runs `git worktree remove <path> --force`. The `--force` flag removes the
/// worktree even if it has uncommitted changes.
///
/// # Arguments
/// * `repo` - Path to the main git repository
/// * `path` - Path to the worktree to remove
///
/// # Errors
/// Returns `GitError` if the worktree does not exist or the git command fails.
pub fn git_worktree_remove(
    repo: impl AsRef<Path>,
    path: impl AsRef<Path>,
) -> Result<(), GitError> {
    let repo = repo.as_ref();
    let path = path.as_ref();
    let path_str = path
        .to_str()
        .ok_or_else(|| GitError::CommandFailed("worktree path is not valid UTF-8".to_string()))?;
    let config = ProcessConfig {
        command: format!("git worktree remove {path_str} --force"),
        working_dir: repo.to_path_buf(),
        env: None,
        env_remove: vec![],
        log_path: None,
        timeout: Some(Duration::from_secs(GIT_DEFAULT_TIMEOUT_SECS)),
        stdin: None,
    };
    let output = run_to_completion_with_timeout(&config)?;
    if !output.success {
        return Err(GitError::CommandFailed(format!(
            "git worktree remove failed: {}",
            output.stderr
        )));
    }
    Ok(())
}

/// Delete a git branch (force delete).
///
/// Runs `git branch -D <branch>`. The force flag allows deleting unmerged branches.
///
/// # Arguments
/// * `repo`   - Path to the git repository
/// * `branch` - Name of the branch to delete
///
/// # Errors
/// Returns `GitError` if the branch does not exist or the git command fails.
pub fn git_branch_delete(repo: impl AsRef<Path>, branch: &str) -> Result<(), GitError> {
    let repo = repo.as_ref();
    let config = ProcessConfig {
        command: format!("git branch -D {branch}"),
        working_dir: repo.to_path_buf(),
        env: None,
        env_remove: vec![],
        log_path: None,
        timeout: Some(Duration::from_secs(GIT_DEFAULT_TIMEOUT_SECS)),
        stdin: None,
    };
    let output = run_to_completion_with_timeout(&config)?;
    if !output.success {
        return Err(GitError::CommandFailed(format!(
            "git branch -D failed: {}",
            output.stderr
        )));
    }
    Ok(())
}

/// Checkout a specific commit.
///
/// Runs `git checkout <commit>` to move HEAD to the specified commit.
///
/// # Arguments
/// * `path` - Path to the git repository
/// * `commit` - The commit hash to checkout
///
/// # Errors
/// Returns an error if the git command fails.
pub fn git_checkout(path: impl AsRef<Path>, commit: &str) -> Result<(), GitError> {
    let path = path.as_ref();
    let config = ProcessConfig {
        command: format!("git checkout {commit}"),
        working_dir: path.to_path_buf(),
        env: None,
        env_remove: vec![],
        log_path: None,
        timeout: Some(Duration::from_secs(GIT_DEFAULT_TIMEOUT_SECS)),
        stdin: None,
    };
    let output = run_to_completion_with_timeout(&config)?;
    if !output.success {
        return Err(GitError::CommandFailed(format!(
            "git checkout failed: {}",
            output.stderr
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    /// Initialize a git repo with user config and an initial commit.
    fn init_test_repo(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        Command::new("git")
            .args(["init"])
            .current_dir(path)
            .output()?;
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(path)
            .output()?;
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(path)
            .output()?;
        fs::write(path.join("README.md"), "test")?;
        Command::new("git")
            .args(["add", "."])
            .current_dir(path)
            .output()?;
        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(path)
            .output()?;
        Ok(())
    }

    #[test]
    fn is_git_repo_returns_true_for_repo() {
        let temp_dir = TempDir::new().unwrap();
        init_test_repo(temp_dir.path()).unwrap();
        assert!(is_git_repo(temp_dir.path()));
    }

    #[test]
    fn is_git_repo_returns_false_for_non_repo() {
        let temp_dir = TempDir::new().unwrap();
        assert!(!is_git_repo(temp_dir.path()));
    }

    #[test]
    fn get_current_commit_returns_valid_hash() {
        let temp_dir = TempDir::new().unwrap();
        init_test_repo(temp_dir.path()).unwrap();

        let commit = get_current_commit(temp_dir.path()).unwrap();
        // SHA-1 hash should be 40 hex characters
        assert_eq!(commit.len(), 40);
        assert!(commit.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn get_current_commit_fails_for_non_repo() {
        let temp_dir = TempDir::new().unwrap();
        let result = get_current_commit(temp_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn git_rev_parse_resolves_head() {
        let temp_dir = TempDir::new().unwrap();
        init_test_repo(temp_dir.path()).unwrap();

        let commit = git_rev_parse(temp_dir.path(), "HEAD").unwrap();
        assert_eq!(commit.len(), 40);
        assert!(commit.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn git_rev_parse_fails_for_invalid_ref() {
        let temp_dir = TempDir::new().unwrap();
        init_test_repo(temp_dir.path()).unwrap();

        let result = git_rev_parse(temp_dir.path(), "nonexistent-branch");
        assert!(result.is_err());
    }

    #[test]
    fn git_fetch_succeeds_without_remote() {
        let temp_dir = TempDir::new().unwrap();
        init_test_repo(temp_dir.path()).unwrap();

        // git fetch --all succeeds even without remotes (it just does nothing)
        let result = git_fetch(temp_dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn git_checkout_changes_commit() {
        let temp_dir = TempDir::new().unwrap();
        init_test_repo(temp_dir.path()).unwrap();

        // Create a second commit
        fs::write(temp_dir.path().join("file2.txt"), "content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Second commit"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        let second_commit = get_current_commit(temp_dir.path()).unwrap();

        // Checkout HEAD~1 (first commit)
        git_checkout(temp_dir.path(), "HEAD~1").unwrap();

        let first_commit = get_current_commit(temp_dir.path()).unwrap();
        assert_ne!(first_commit, second_commit);
    }

    #[test]
    fn git_checkout_fails_for_invalid_commit() {
        let temp_dir = TempDir::new().unwrap();
        init_test_repo(temp_dir.path()).unwrap();

        let result = git_checkout(temp_dir.path(), "0000000000000000000000000000000000000000");
        assert!(result.is_err());
    }

    #[test]
    fn git_worktree_list_main_worktree() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path()).unwrap();

        let worktrees = git_worktree_list(temp.path()).unwrap();
        // Always at least the main worktree
        assert!(!worktrees.is_empty());
        // First entry is main worktree with a branch
        let main = &worktrees[0];
        assert!(main.branch.is_some());
        assert_eq!(main.head.len(), 40);
    }

    #[test]
    fn git_worktree_list_includes_added_worktree() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path()).unwrap();

        let wt_path = temp.path().join("extra");
        Command::new("git")
            .args([
                "worktree",
                "add",
                wt_path.to_str().unwrap(),
                "-b",
                "feature/test-wt",
            ])
            .current_dir(temp.path())
            .output()
            .unwrap();

        let worktrees = git_worktree_list(temp.path()).unwrap();
        assert_eq!(worktrees.len(), 2);

        let wt = worktrees
            .iter()
            .find(|w| w.branch.as_deref() == Some("feature/test-wt"))
            .expect("added worktree not found");
        // path in output is absolute; wt_path may not be canonicalized the same way
        assert!(wt.path.ends_with("extra"));
        assert_eq!(wt.head.len(), 40);
    }

    #[test]
    fn git_worktree_add_creates_worktree() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path()).unwrap();

        let wt_path = temp.path().join("new-wt");
        let returned = git_worktree_add(temp.path(), &wt_path, "feature/new").unwrap();

        // The returned path points to the created directory
        assert!(returned.exists());

        // The worktree appears in git_worktree_list
        let worktrees = git_worktree_list(temp.path()).unwrap();
        assert!(worktrees
            .iter()
            .any(|w| w.branch.as_deref() == Some("feature/new")));
    }

    #[test]
    fn git_worktree_add_fails_if_branch_exists() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path()).unwrap();

        // Get the default branch name
        let worktrees = git_worktree_list(temp.path()).unwrap();
        let main_branch = worktrees[0].branch.clone().unwrap();

        // Trying to create a worktree with the existing branch name should fail
        let wt_path = temp.path().join("conflict-wt");
        let result = git_worktree_add(temp.path(), &wt_path, &main_branch);
        assert!(result.is_err());
    }

    #[test]
    fn git_worktree_remove_removes_worktree() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path()).unwrap();

        let wt_path = temp.path().join("to-remove");
        git_worktree_add(temp.path(), &wt_path, "feature/to-remove").unwrap();
        assert_eq!(git_worktree_list(temp.path()).unwrap().len(), 2);

        git_worktree_remove(temp.path(), &wt_path).unwrap();
        let worktrees = git_worktree_list(temp.path()).unwrap();
        assert_eq!(worktrees.len(), 1);
        assert!(!worktrees
            .iter()
            .any(|w| w.branch.as_deref() == Some("feature/to-remove")));
    }

    #[test]
    fn git_worktree_remove_fails_for_nonexistent() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path()).unwrap();

        let result = git_worktree_remove(temp.path(), temp.path().join("no-such-worktree"));
        assert!(result.is_err());
    }

    #[test]
    fn git_branch_delete_removes_branch() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path()).unwrap();

        let wt_path = temp.path().join("branch-wt");
        git_worktree_add(temp.path(), &wt_path, "feature/to-delete").unwrap();
        git_worktree_remove(temp.path(), &wt_path).unwrap();

        // Branch still exists after worktree removal — now delete it
        git_branch_delete(temp.path(), "feature/to-delete").unwrap();

        // Verify it's gone
        let result = git_branch_delete(temp.path(), "feature/to-delete");
        assert!(result.is_err());
    }

    #[test]
    fn git_branch_delete_fails_for_nonexistent() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path()).unwrap();

        let result = git_branch_delete(temp.path(), "no-such-branch");
        assert!(result.is_err());
    }

    #[test]
    fn git_worktree_list_detached_head_has_none_branch() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path()).unwrap();

        let commit = get_current_commit(temp.path()).unwrap();
        let wt_path = temp.path().join("detached-wt");
        // Worktree at a specific commit → detached HEAD
        Command::new("git")
            .args([
                "worktree",
                "add",
                "--detach",
                wt_path.to_str().unwrap(),
                &commit,
            ])
            .current_dir(temp.path())
            .output()
            .unwrap();

        let worktrees = git_worktree_list(temp.path()).unwrap();
        let detached = worktrees
            .iter()
            .find(|w| w.path.ends_with("detached-wt"))
            .expect("detached worktree not found");
        assert!(detached.branch.is_none());
    }
}
