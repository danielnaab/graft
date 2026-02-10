//! Git status adapter using gitoxide and git command.

use grove_core::{CoreError, GitStatus as GitStatusTrait, RepoPath, RepoStatus, Result};
use std::path::Path;
use std::process::{Command, Output};
use std::time::Duration;
use wait_timeout::ChildExt;

/// Default timeout for git subprocess commands (5 seconds)
const DEFAULT_GIT_TIMEOUT_MS: u64 = 5000;

/// Get the git timeout from environment variable or use default.
fn get_git_timeout() -> u64 {
    std::env::var("GROVE_GIT_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(DEFAULT_GIT_TIMEOUT_MS)
}

/// Run a git command with a timeout.
///
/// Returns the command output on success, or an error if the command fails or times out.
fn run_git_with_timeout(mut cmd: Command, operation: &str) -> Result<Output> {
    // Spawn with piped stdout/stderr
    let mut child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| CoreError::GitError {
            details: format!("Failed to spawn git {operation}: {e}"),
        })?;

    let timeout_ms = get_git_timeout();
    let timeout = Duration::from_millis(timeout_ms);

    match child.wait_timeout(timeout) {
        Ok(Some(_status)) => {
            // Process completed within timeout - get output
            child.wait_with_output().map_err(|e| CoreError::GitError {
                details: format!("Failed to read git output for {operation}: {e}"),
            })
        }
        Ok(None) => {
            // Timeout occurred, kill the process
            let _ = child.kill();
            let _ = child.wait();
            Err(CoreError::GitTimeout {
                operation: operation.to_string(),
                timeout_ms,
            })
        }
        Err(e) => Err(CoreError::GitError {
            details: format!("Git process error for {operation}: {e}"),
        }),
    }
}

/// Gitoxide-based git status implementation.
#[derive(Debug)]
pub struct GitoxideStatus;

impl GitoxideStatus {
    pub fn new() -> Self {
        Self
    }

    fn query_status(repo_path: &Path) -> Result<RepoStatus> {
        log::trace!("git::query_status: {}", repo_path.display());

        // Open the repository
        let repo = gix::discover(repo_path).map_err(|e| {
            let path = repo_path.display();
            CoreError::GitError {
                details: format!("Failed to open repository at {path}: {e}"),
            }
        })?;

        // Get current branch name
        let head = repo.head().map_err(|e| CoreError::GitError {
            details: format!("Failed to get HEAD: {e}"),
        })?;

        let branch = head
            .referent_name()
            .and_then(|name| name.shorten().to_string().into());

        // Check if working tree is dirty by shelling out to git
        let is_dirty = Self::check_dirty(repo_path)?;

        // Check ahead/behind counts by shelling out to git
        let (ahead, behind) = Self::check_ahead_behind(repo_path);

        let repo_path = RepoPath::new(&repo_path.display().to_string())?;

        Ok(RepoStatus {
            path: repo_path,
            branch,
            is_dirty,
            ahead,
            behind,
            error: None,
        })
    }

    /// Check if working tree is dirty using git status --porcelain
    fn check_dirty(repo_path: &Path) -> Result<bool> {
        let mut cmd = Command::new("git");
        cmd.args(["status", "--porcelain"]).current_dir(repo_path);

        let output = run_git_with_timeout(cmd, "status --porcelain")?;
        Ok(!output.stdout.is_empty())
    }

    /// Check ahead/behind counts using git rev-list
    fn check_ahead_behind(repo_path: &Path) -> (Option<usize>, Option<usize>) {
        // Get the upstream branch
        let mut cmd = Command::new("git");
        cmd.args(["rev-parse", "--abbrev-ref", "@{upstream}"])
            .current_dir(repo_path);

        let upstream = run_git_with_timeout(cmd, "rev-parse upstream")
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    String::from_utf8(o.stdout)
                        .ok()
                        .map(|s| s.trim().to_string())
                } else {
                    None
                }
            });

        let Some(upstream) = upstream else {
            // No upstream configured or timeout/error
            return (None, None);
        };

        // Count commits ahead
        let mut cmd = Command::new("git");
        cmd.args(["rev-list", "--count", &format!("{upstream}..HEAD")])
            .current_dir(repo_path);

        let ahead = run_git_with_timeout(cmd, "rev-list ahead count")
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|s| s.trim().parse::<usize>().ok())
            .filter(|&n| n > 0);

        // Count commits behind
        let mut cmd = Command::new("git");
        cmd.args(["rev-list", "--count", &format!("HEAD..{upstream}")])
            .current_dir(repo_path);

        let behind = run_git_with_timeout(cmd, "rev-list behind count")
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|s| s.trim().parse::<usize>().ok())
            .filter(|&n| n > 0);

        (ahead, behind)
    }
}

impl Default for GitoxideStatus {
    fn default() -> Self {
        Self::new()
    }
}

impl GitStatusTrait for GitoxideStatus {
    fn get_status(&self, repo_path: &RepoPath) -> Result<RepoStatus> {
        Self::query_status(repo_path.as_path())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fails_on_non_git_directory() {
        let status = GitoxideStatus::new();
        let path = RepoPath::new("/tmp").unwrap();
        let result = status.get_status(&path);
        // This should fail since /tmp is typically not a git repo
        assert!(result.is_err() || result.unwrap().error.is_some());
    }

    #[test]
    fn detects_clean_working_tree() {
        // This test requires a clean git repo to exist
        // Create a temporary git repo for testing
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        // Initialize git repo
        Command::new("git")
            .args(["init"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Configure user (required for commits)
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Create and commit a file
        fs::write(repo_path.join("README.md"), "test").unwrap();
        Command::new("git")
            .args(["add", "README.md"])
            .current_dir(repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Check status
        let status = GitoxideStatus::new();
        let path = RepoPath::new(repo_path.to_str().unwrap()).unwrap();
        let result = status.get_status(&path).unwrap();

        assert!(!result.is_dirty, "Clean repo should not be dirty");
        assert_eq!(
            result.branch,
            Some("main".to_string()),
            "Should be on main branch"
        );
    }

    #[test]
    fn detects_dirty_working_tree() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        // Initialize git repo
        Command::new("git")
            .args(["init"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Configure user
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Create and commit a file
        fs::write(repo_path.join("README.md"), "test").unwrap();
        Command::new("git")
            .args(["add", "README.md"])
            .current_dir(repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Modify the file (make it dirty)
        fs::write(repo_path.join("README.md"), "modified").unwrap();

        // Check status
        let status = GitoxideStatus::new();
        let path = RepoPath::new(repo_path.to_str().unwrap()).unwrap();
        let result = status.get_status(&path).unwrap();

        assert!(result.is_dirty, "Modified repo should be dirty");
    }

    #[test]
    fn handles_detached_head_state() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        // Initialize repo with a commit
        Command::new("git")
            .args(["init"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Configure user
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Create initial commit
        std::fs::write(repo_path.join("file.txt"), "content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Get the commit SHA
        let sha_output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(repo_path)
            .output()
            .unwrap();
        let sha = String::from_utf8(sha_output.stdout)
            .unwrap()
            .trim()
            .to_string();

        // Detach HEAD by checking out the commit SHA
        Command::new("git")
            .args(["checkout", &sha])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Check status
        let status = GitoxideStatus::new();
        let path = RepoPath::new(repo_path.to_str().unwrap()).unwrap();
        let result = status.get_status(&path).unwrap();

        // In detached HEAD state, branch should be None
        assert!(
            result.branch.is_none(),
            "Detached HEAD should have no branch name, got: {:?}",
            result.branch
        );
    }

    #[test]
    fn handles_repository_without_upstream() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        // Initialize repo without remote
        Command::new("git")
            .args(["init"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Configure user
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Create initial commit
        std::fs::write(repo_path.join("file.txt"), "content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Check ahead/behind - should return (None, None) since no upstream
        let (ahead, behind) = GitoxideStatus::check_ahead_behind(repo_path);

        assert_eq!(ahead, None, "No upstream means no ahead count");
        assert_eq!(behind, None, "No upstream means no behind count");
    }

    #[test]
    fn handles_repository_with_upstream_no_divergence() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        // Initialize bare remote repo
        let remote_dir = TempDir::new().unwrap();
        let remote_path = remote_dir.path();
        Command::new("git")
            .args(["init", "--bare"])
            .current_dir(remote_path)
            .output()
            .unwrap();

        // Initialize local repo
        Command::new("git")
            .args(["init"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Configure user
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Create initial commit
        std::fs::write(repo_path.join("file.txt"), "content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Add remote and push
        Command::new("git")
            .args(["remote", "add", "origin", remote_path.to_str().unwrap()])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Set branch name to main (git init might use master or main)
        Command::new("git")
            .args(["branch", "-M", "main"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        Command::new("git")
            .args(["push", "-u", "origin", "main"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Check ahead/behind - should be (None, None) or (Some(0), Some(0)) when in sync
        let (ahead, behind) = GitoxideStatus::check_ahead_behind(repo_path);

        // The implementation filters out 0 values, so we expect None for both
        assert_eq!(
            ahead, None,
            "In sync with upstream means no ahead commits (0 filtered to None)"
        );
        assert_eq!(
            behind, None,
            "In sync with upstream means no behind commits (0 filtered to None)"
        );
    }

    #[test]
    fn detects_ahead_commits() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        // Initialize bare remote repo
        let remote_dir = TempDir::new().unwrap();
        let remote_path = remote_dir.path();
        Command::new("git")
            .args(["init", "--bare"])
            .current_dir(remote_path)
            .output()
            .unwrap();

        // Initialize local repo
        Command::new("git")
            .args(["init"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Configure user
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Create and push initial commit
        std::fs::write(repo_path.join("file1.txt"), "content1").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "First commit"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        Command::new("git")
            .args(["remote", "add", "origin", remote_path.to_str().unwrap()])
            .current_dir(repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["branch", "-M", "main"])
            .current_dir(repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["push", "-u", "origin", "main"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Create local commit (ahead of remote)
        std::fs::write(repo_path.join("file2.txt"), "content2").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(repo_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Second commit (local)"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Check ahead/behind
        let (ahead, behind) = GitoxideStatus::check_ahead_behind(repo_path);

        assert_eq!(ahead, Some(1), "Should be 1 commit ahead of remote");
        assert_eq!(behind, None, "Should not be behind remote");
    }
}
