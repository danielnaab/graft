//! Git status adapter using gitoxide and git command.

use graft_common::command::{run_command_with_timeout, CommandError};
use grove_core::{
    CommitInfo, CoreError, FileChange, FileChangeStatus, GitStatus as GitStatusTrait, RepoDetail,
    RepoDetailProvider, RepoPath, RepoStatus, Result,
};
use std::path::Path;
use std::process::{Command, Output};

/// Run a git command with a timeout, converting `CommandError` to `CoreError`.
fn run_git_with_timeout(cmd: Command, operation: &str) -> Result<Output> {
    run_command_with_timeout(cmd, operation, Some("GROVE_GIT_TIMEOUT_MS")).map_err(|e| match e {
        CommandError::SpawnError { details, .. } => CoreError::GitError {
            details: format!("Failed to spawn git {operation}: {details}"),
        },
        CommandError::Timeout {
            timeout_ms,
            operation,
        } => CoreError::GitTimeout {
            operation,
            timeout_ms,
        },
        CommandError::OutputError { details, .. } => CoreError::GitError {
            details: format!("Failed to read git output for {operation}: {details}"),
        },
        CommandError::ProcessError { details, .. } => CoreError::GitError {
            details: format!("Git process error for {operation}: {details}"),
        },
    })
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

    /// Query recent commits from a git repository.
    ///
    /// Runs `git log --format="%h%x00%s%x00%an%x00%ar" -n {limit}` and parses the output.
    fn query_commits(path: &Path, limit: usize) -> Result<Vec<CommitInfo>> {
        let mut cmd = Command::new("git");
        cmd.args([
            "log",
            "--format=%h%x00%s%x00%an%x00%ar",
            "-n",
            &limit.to_string(),
        ])
        .current_dir(path);

        let output = run_git_with_timeout(cmd, "log")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CoreError::GitError {
                details: format!("git log failed: {stderr}"),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let commits = stdout
            .lines()
            .filter(|line| !line.is_empty())
            .filter_map(|line| {
                let parts: Vec<&str> = line.splitn(4, '\0').collect();
                if parts.len() == 4 {
                    Some(CommitInfo {
                        hash: parts[0].to_string(),
                        subject: parts[1].to_string(),
                        author: parts[2].to_string(),
                        relative_date: parts[3].to_string(),
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(commits)
    }

    /// Parse a git status porcelain XY code into a `FileChangeStatus`.
    fn parse_file_change_status(xy: &str) -> FileChangeStatus {
        // git status --porcelain uses XY codes in first two columns
        // We check both X (index) and Y (working tree) columns
        let bytes = xy.as_bytes();
        let x = if bytes.is_empty() { b' ' } else { bytes[0] };
        let y = if bytes.len() < 2 { b' ' } else { bytes[1] };

        // Prefer working tree status (Y), fall back to index status (X)
        match (x, y) {
            (_, b'M') | (b'M', _) => FileChangeStatus::Modified,
            // Untracked files (b'?', b'?') shown as Added
            (_, b'A') | (b'A', _) | (b'?', b'?') => FileChangeStatus::Added,
            (_, b'D') | (b'D', _) => FileChangeStatus::Deleted,
            (b'R', _) => FileChangeStatus::Renamed,
            (b'C', _) => FileChangeStatus::Copied,
            _ => FileChangeStatus::Unknown,
        }
    }

    /// Query changed files from a git repository.
    ///
    /// Runs `git status --porcelain` and parses the XY status codes.
    fn query_changed_files(path: &Path) -> Result<Vec<FileChange>> {
        let mut cmd = Command::new("git");
        cmd.args(["status", "--porcelain"]).current_dir(path);

        let output = run_git_with_timeout(cmd, "status --porcelain (detail)")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CoreError::GitError {
                details: format!("git status failed: {stderr}"),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let changes = stdout
            .lines()
            .filter(|line| line.len() >= 4) // "XY filename" minimum
            .map(|line| {
                let xy = &line[..2];
                let file_path = line[3..].to_string();
                FileChange {
                    path: file_path,
                    status: Self::parse_file_change_status(xy),
                }
            })
            .collect();

        Ok(changes)
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

impl RepoDetailProvider for GitoxideStatus {
    fn get_detail(&self, path: &RepoPath, max_commits: usize) -> Result<RepoDetail> {
        // Verify it's a git repo first
        gix::discover(path.as_path()).map_err(|e| CoreError::GitError {
            details: format!(
                "Failed to open repository at {}: {e}",
                path.as_path().display()
            ),
        })?;

        let mut errors: Vec<String> = Vec::new();

        let commits = match Self::query_commits(path.as_path(), max_commits) {
            Ok(c) => c,
            Err(e) => {
                log::warn!("Failed to query commits for {path}: {e}");
                errors.push(format!("commits: {e}"));
                Vec::new()
            }
        };

        let changed_files = match Self::query_changed_files(path.as_path()) {
            Ok(f) => f,
            Err(e) => {
                log::warn!("Failed to query changed files for {path}: {e}");
                errors.push(format!("changed files: {e}"));
                Vec::new()
            }
        };

        let error = if errors.is_empty() {
            None
        } else {
            Some(errors.join("; "))
        };

        Ok(RepoDetail {
            commits,
            changed_files,
            error,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Initialize a git repo with user config. If `initial_commit` is true,
    /// creates a file and commits it.
    fn init_test_repo(path: &std::path::Path, initial_commit: bool) {
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

        if initial_commit {
            std::fs::write(path.join("README.md"), "test").unwrap();
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
    }

    /// Add a file and commit in an existing repo.
    fn add_commit(path: &std::path::Path, filename: &str, content: &str, message: &str) {
        std::fs::write(path.join(filename), content).unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(path)
            .output()
            .unwrap();
    }

    /// Set up a bare remote and push the local repo to it.
    fn push_to_bare_remote(repo_path: &std::path::Path) -> TempDir {
        let remote_dir = TempDir::new().unwrap();
        Command::new("git")
            .args(["init", "--bare"])
            .current_dir(remote_dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args([
                "remote",
                "add",
                "origin",
                remote_dir.path().to_str().unwrap(),
            ])
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
        remote_dir
    }

    #[test]
    fn fails_on_non_git_directory() {
        let status = GitoxideStatus::new();
        let path = RepoPath::new("/tmp").unwrap();
        let result = status.get_status(&path);
        assert!(result.is_err() || result.unwrap().error.is_some());
    }

    #[test]
    fn detects_clean_working_tree() {
        let temp_dir = TempDir::new().unwrap();
        init_test_repo(temp_dir.path(), true);

        let status = GitoxideStatus::new();
        let path = RepoPath::new(temp_dir.path().to_str().unwrap()).unwrap();
        let result = status.get_status(&path).unwrap();

        assert!(!result.is_dirty, "Clean repo should not be dirty");
        assert_eq!(result.branch, Some("main".to_string()));
    }

    #[test]
    fn detects_dirty_working_tree() {
        let temp_dir = TempDir::new().unwrap();
        init_test_repo(temp_dir.path(), true);

        std::fs::write(temp_dir.path().join("README.md"), "modified").unwrap();

        let status = GitoxideStatus::new();
        let path = RepoPath::new(temp_dir.path().to_str().unwrap()).unwrap();
        let result = status.get_status(&path).unwrap();

        assert!(result.is_dirty, "Modified repo should be dirty");
    }

    #[test]
    fn handles_detached_head_state() {
        let temp_dir = TempDir::new().unwrap();
        init_test_repo(temp_dir.path(), true);

        let sha_output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();
        let sha = String::from_utf8(sha_output.stdout)
            .unwrap()
            .trim()
            .to_string();

        Command::new("git")
            .args(["checkout", &sha])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        let status = GitoxideStatus::new();
        let path = RepoPath::new(temp_dir.path().to_str().unwrap()).unwrap();
        let result = status.get_status(&path).unwrap();

        assert!(
            result.branch.is_none(),
            "Detached HEAD should have no branch name, got: {:?}",
            result.branch
        );
    }

    #[test]
    fn handles_repository_without_upstream() {
        let temp_dir = TempDir::new().unwrap();
        init_test_repo(temp_dir.path(), true);

        let (ahead, behind) = GitoxideStatus::check_ahead_behind(temp_dir.path());

        assert_eq!(ahead, None, "No upstream means no ahead count");
        assert_eq!(behind, None, "No upstream means no behind count");
    }

    #[test]
    fn handles_repository_with_upstream_no_divergence() {
        let temp_dir = TempDir::new().unwrap();
        init_test_repo(temp_dir.path(), true);
        let _remote_dir = push_to_bare_remote(temp_dir.path());

        let (ahead, behind) = GitoxideStatus::check_ahead_behind(temp_dir.path());

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
    fn query_commits_returns_ordered_commits() {
        let temp_dir = TempDir::new().unwrap();
        init_test_repo(temp_dir.path(), false);

        add_commit(temp_dir.path(), "file1.txt", "first", "First commit");
        add_commit(temp_dir.path(), "file2.txt", "second", "Second commit");

        let commits = GitoxideStatus::query_commits(temp_dir.path(), 10).unwrap();
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].subject, "Second commit");
        assert_eq!(commits[1].subject, "First commit");
        assert!(!commits[0].author.is_empty(), "Author should not be empty");
        assert!(!commits[0].hash.is_empty());
    }

    #[test]
    fn query_commits_respects_limit() {
        let temp_dir = TempDir::new().unwrap();
        init_test_repo(temp_dir.path(), false);

        for i in 0..5 {
            add_commit(
                temp_dir.path(),
                &format!("file{i}.txt"),
                &format!("content{i}"),
                &format!("Commit {i}"),
            );
        }

        let commits = GitoxideStatus::query_commits(temp_dir.path(), 3).unwrap();
        assert_eq!(commits.len(), 3, "Should limit to 3 commits");
    }

    #[test]
    fn query_changed_files_detects_modifications() {
        let temp_dir = TempDir::new().unwrap();
        init_test_repo(temp_dir.path(), false);
        add_commit(temp_dir.path(), "tracked.txt", "original", "Initial");

        std::fs::write(temp_dir.path().join("tracked.txt"), "modified").unwrap();
        std::fs::write(temp_dir.path().join("untracked.txt"), "new").unwrap();

        let changes = GitoxideStatus::query_changed_files(temp_dir.path()).unwrap();
        assert_eq!(changes.len(), 2);

        let modified = changes.iter().find(|c| c.path == "tracked.txt").unwrap();
        assert_eq!(modified.status, FileChangeStatus::Modified);

        let untracked = changes.iter().find(|c| c.path == "untracked.txt").unwrap();
        assert_eq!(untracked.status, FileChangeStatus::Added);
    }

    #[test]
    fn query_changed_files_empty_for_clean_repo() {
        let temp_dir = TempDir::new().unwrap();
        init_test_repo(temp_dir.path(), true);

        let changes = GitoxideStatus::query_changed_files(temp_dir.path()).unwrap();
        assert!(changes.is_empty(), "Clean repo should have no changes");
    }

    #[test]
    fn get_detail_fails_for_non_git_directory() {
        let provider = GitoxideStatus::new();
        let path = RepoPath::new("/tmp").unwrap();
        let result = provider.get_detail(&path, 10);
        assert!(result.is_err(), "Non-git dir should error");
    }

    #[test]
    fn detects_ahead_commits() {
        let temp_dir = TempDir::new().unwrap();
        init_test_repo(temp_dir.path(), true);
        let _remote_dir = push_to_bare_remote(temp_dir.path());

        add_commit(
            temp_dir.path(),
            "file2.txt",
            "content2",
            "Second commit (local)",
        );

        let (ahead, behind) = GitoxideStatus::check_ahead_behind(temp_dir.path());

        assert_eq!(ahead, Some(1), "Should be 1 commit ahead of remote");
        assert_eq!(behind, None, "Should not be behind remote");
    }

    // --- parse_file_change_status unit tests ---

    #[test]
    fn parse_status_modified_in_working_tree() {
        assert_eq!(
            GitoxideStatus::parse_file_change_status(" M"),
            FileChangeStatus::Modified
        );
    }

    #[test]
    fn parse_status_modified_in_index() {
        assert_eq!(
            GitoxideStatus::parse_file_change_status("M "),
            FileChangeStatus::Modified
        );
    }

    #[test]
    fn parse_status_staged_and_modified() {
        assert_eq!(
            GitoxideStatus::parse_file_change_status("MM"),
            FileChangeStatus::Modified
        );
    }

    #[test]
    fn parse_status_added_in_index() {
        assert_eq!(
            GitoxideStatus::parse_file_change_status("A "),
            FileChangeStatus::Added
        );
    }

    #[test]
    fn parse_status_added_then_modified() {
        assert_eq!(
            GitoxideStatus::parse_file_change_status("AM"),
            FileChangeStatus::Modified
        );
    }

    #[test]
    fn parse_status_untracked() {
        assert_eq!(
            GitoxideStatus::parse_file_change_status("??"),
            FileChangeStatus::Added
        );
    }

    #[test]
    fn parse_status_deleted_in_working_tree() {
        assert_eq!(
            GitoxideStatus::parse_file_change_status(" D"),
            FileChangeStatus::Deleted
        );
    }

    #[test]
    fn parse_status_deleted_in_index() {
        assert_eq!(
            GitoxideStatus::parse_file_change_status("D "),
            FileChangeStatus::Deleted
        );
    }

    #[test]
    fn parse_status_renamed() {
        assert_eq!(
            GitoxideStatus::parse_file_change_status("R "),
            FileChangeStatus::Renamed
        );
    }

    #[test]
    fn parse_status_copied() {
        assert_eq!(
            GitoxideStatus::parse_file_change_status("C "),
            FileChangeStatus::Copied
        );
    }

    #[test]
    fn parse_status_unknown() {
        assert_eq!(
            GitoxideStatus::parse_file_change_status("!!"),
            FileChangeStatus::Unknown
        );
    }

    // --- empty repo / deleted file tests ---

    #[test]
    fn get_detail_on_empty_repo_no_commits() {
        let temp_dir = TempDir::new().unwrap();
        init_test_repo(temp_dir.path(), false);

        let provider = GitoxideStatus::new();
        let path = RepoPath::new(temp_dir.path().to_str().unwrap()).unwrap();
        let detail = provider.get_detail(&path, 10).unwrap();

        assert!(
            detail.commits.is_empty(),
            "Empty repo should have no commits"
        );
        assert!(
            detail.error.is_some(),
            "Should report error from git log on empty repo"
        );
    }

    #[test]
    fn query_changed_files_detects_deletion() {
        let temp_dir = TempDir::new().unwrap();
        init_test_repo(temp_dir.path(), false);
        add_commit(temp_dir.path(), "to_delete.txt", "content", "Add file");

        std::fs::remove_file(temp_dir.path().join("to_delete.txt")).unwrap();

        let changes = GitoxideStatus::query_changed_files(temp_dir.path()).unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "to_delete.txt");
        assert_eq!(changes[0].status, FileChangeStatus::Deleted);
    }
}
