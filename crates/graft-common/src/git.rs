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
}
