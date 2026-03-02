//! Git operations with timeout protection.
//!
//! This module provides shared git primitives used by both graft and grove.
//! All operations apply a 30-second default timeout to prevent hangs on network
//! or I/O issues. The `GRAFT_PROCESS_TIMEOUT_MS` environment variable overrides
//! this default when set.

use crate::process::{run_to_completion_with_timeout, shell_quote, ProcessConfig, ProcessError};
use std::path::{Path, PathBuf};
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
            command: format!("git rev-parse {}", shell_quote(&ref_name)),
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
    pub path: PathBuf,
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
        let mut path: Option<PathBuf> = None;
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
) -> Result<PathBuf, GitError> {
    let repo = repo.as_ref();
    let path = path.as_ref();
    let path_str = path
        .to_str()
        .ok_or_else(|| GitError::CommandFailed("worktree path is not valid UTF-8".to_string()))?;
    let config = ProcessConfig {
        command: format!(
            "git worktree add {} -b {}",
            shell_quote(path_str),
            shell_quote(branch)
        ),
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
pub fn git_worktree_remove(repo: impl AsRef<Path>, path: impl AsRef<Path>) -> Result<(), GitError> {
    let repo = repo.as_ref();
    let path = path.as_ref();
    let path_str = path
        .to_str()
        .ok_or_else(|| GitError::CommandFailed("worktree path is not valid UTF-8".to_string()))?;
    let config = ProcessConfig {
        command: format!("git worktree remove {} --force", shell_quote(path_str)),
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
        command: format!("git branch -D {}", shell_quote(branch)),
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

/// Count commits ahead and behind between two refs.
///
/// Runs `git rev-list --left-right --count <branch>...<base>`.
///
/// # Returns
/// `(ahead, behind)` — commits in `branch` not in `base`, and vice versa.
///
/// # Arguments
/// * `repo`   - Path to the git repository
/// * `branch` - Branch to measure from
/// * `base`   - Reference to compare against (e.g. "main")
///
/// # Errors
/// Returns `GitError` if either ref is invalid or the git command fails.
pub fn git_ahead_behind(
    repo: impl AsRef<Path>,
    branch: &str,
    base: &str,
) -> Result<(usize, usize), GitError> {
    let repo = repo.as_ref();
    let config = ProcessConfig {
        command: format!(
            "git rev-list --left-right --count {}...{}",
            shell_quote(branch),
            shell_quote(base)
        ),
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
            "git rev-list --left-right --count failed: {}",
            output.stderr
        )));
    }
    let trimmed = output.stdout.trim();
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    if parts.len() != 2 {
        return Err(GitError::CommandFailed(format!(
            "Unexpected output from git rev-list: {trimmed}"
        )));
    }
    let ahead = parts[0]
        .parse::<usize>()
        .map_err(|e| GitError::CommandFailed(format!("Failed to parse ahead count: {e}")))?;
    let behind = parts[1]
        .parse::<usize>()
        .map_err(|e| GitError::CommandFailed(format!("Failed to parse behind count: {e}")))?;
    Ok((ahead, behind))
}

/// Get the Unix timestamp of the most recent commit on a branch.
///
/// Runs `git log -1 --format=%ct <branch>` to retrieve the committer timestamp.
///
/// # Arguments
/// * `repo`   - Path to the git repository
/// * `branch` - Branch name (or any rev) to query
///
/// # Returns
/// Unix timestamp as `i64`.
///
/// # Errors
/// Returns `GitError` if the branch has no commits or the ref is invalid.
pub fn git_last_commit_time(repo: impl AsRef<Path>, branch: &str) -> Result<i64, GitError> {
    let repo = repo.as_ref();
    let config = ProcessConfig {
        command: format!("git log -1 --format=%ct {}", shell_quote(branch)),
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
            "git log failed for branch '{branch}': {}",
            output.stderr
        )));
    }
    let trimmed = output.stdout.trim();
    if trimmed.is_empty() {
        return Err(GitError::CommandFailed(format!(
            "No commits found on branch '{branch}'"
        )));
    }
    trimmed
        .parse::<i64>()
        .map_err(|e| GitError::CommandFailed(format!("Failed to parse commit timestamp: {e}")))
}

/// Check whether a worktree has uncommitted changes.
///
/// Runs `git -C <worktree_path> status --porcelain` and returns `true` if the
/// output is non-empty (i.e., there are staged or unstaged changes or untracked
/// files).
///
/// # Arguments
/// * `worktree_path` - Absolute path to the worktree directory
///
/// # Errors
/// Returns `GitError` if the git command fails.
pub fn git_is_dirty(worktree_path: impl AsRef<Path>) -> Result<bool, GitError> {
    let path = worktree_path.as_ref();
    let path_str = path
        .to_str()
        .ok_or_else(|| GitError::CommandFailed("worktree path is not valid UTF-8".to_string()))?;
    let config = ProcessConfig {
        command: format!("git -C {} status --porcelain", shell_quote(path_str)),
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
            "git status failed in '{}': {}",
            path.display(),
            output.stderr
        )));
    }
    Ok(!output.stdout.trim().is_empty())
}

/// Check whether a worktree has staged or unstaged changes to tracked files.
///
/// Unlike [`git_is_dirty`], this ignores untracked files. This is useful for
/// checking if `git reset --hard` would destroy work, since reset does not
/// touch untracked files.
pub fn git_has_tracked_changes(worktree_path: impl AsRef<Path>) -> Result<bool, GitError> {
    let path = worktree_path.as_ref();
    let path_str = path
        .to_str()
        .ok_or_else(|| GitError::CommandFailed("worktree path is not valid UTF-8".to_string()))?;
    let config = ProcessConfig {
        command: format!(
            "git -C {} status --porcelain --untracked-files=no",
            shell_quote(path_str)
        ),
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
            "git status failed in '{}': {}",
            path.display(),
            output.stderr
        )));
    }
    Ok(!output.stdout.trim().is_empty())
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
        command: format!("git checkout {}", shell_quote(commit)),
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

/// Merge two branches into a ref without touching HEAD or the working tree.
///
/// Uses `git merge-tree --write-tree` (git 2.38+) to compute the merged tree,
/// `git commit-tree` to create a merge commit, and `git update-ref` to store it.
///
/// # Arguments
/// * `repo`     - Path to the git repository
/// * `source`   - Branch to merge (e.g. `feature/my-scion`)
/// * `target`   - Branch to merge into (e.g. `main`)
/// * `ref_name` - Ref to store the result (e.g. `refs/merge-temp/my-scion`)
///
/// # Returns
/// The commit hash of the new merge commit.
///
/// # Errors
/// Returns `GitError` if there are merge conflicts or any git command fails.
#[allow(clippy::too_many_lines)]
pub fn git_merge_to_ref(
    repo: impl AsRef<Path>,
    source: &str,
    target: &str,
    ref_name: &str,
) -> Result<String, GitError> {
    let repo = repo.as_ref();

    // Resolve source and target to commit hashes for commit-tree parents
    let target_hash = {
        let config = ProcessConfig {
            command: format!("git rev-parse {}", shell_quote(target)),
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
                "Failed to resolve target ref '{target}': {}",
                output.stderr
            )));
        }
        output.stdout.trim().to_string()
    };

    let source_hash = {
        let config = ProcessConfig {
            command: format!("git rev-parse {}", shell_quote(source)),
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
                "Failed to resolve source ref '{source}': {}",
                output.stderr
            )));
        }
        output.stdout.trim().to_string()
    };

    // Compute merged tree via merge-tree --write-tree
    let tree_hash = {
        let config = ProcessConfig {
            command: format!(
                "git merge-tree --write-tree {} {}",
                shell_quote(target),
                shell_quote(source)
            ),
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
                "Merge conflicts between '{source}' and '{target}': {}",
                output.stdout.trim()
            )));
        }
        output.stdout.trim().to_string()
    };

    // Create merge commit
    let commit_hash = {
        let config = ProcessConfig {
            command: format!(
                "git commit-tree {} -p {} -p {} -m {}",
                shell_quote(&tree_hash),
                shell_quote(&target_hash),
                shell_quote(&source_hash),
                shell_quote(&format!("Merge {source} into {target}"))
            ),
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
                "git commit-tree failed: {}",
                output.stderr
            )));
        }
        output.stdout.trim().to_string()
    };

    // Store the merge commit at the requested ref
    {
        let config = ProcessConfig {
            command: format!(
                "git update-ref {} {}",
                shell_quote(ref_name),
                shell_quote(&commit_hash)
            ),
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
                "git update-ref failed: {}",
                output.stderr
            )));
        }
    }

    Ok(commit_hash)
}

/// Advance a branch to point at a given commit (fast-forward).
///
/// Runs `git update-ref refs/heads/<branch> <commit>`. This is a low-level
/// operation — it does not check that the update is actually a fast-forward.
///
/// # Arguments
/// * `repo`   - Path to the git repository
/// * `branch` - Branch name to advance (without `refs/heads/` prefix)
/// * `commit` - Commit hash to set the branch to
///
/// # Errors
/// Returns `GitError` if the git command fails.
pub fn git_fast_forward(
    repo: impl AsRef<Path>,
    branch: &str,
    commit: &str,
) -> Result<(), GitError> {
    let repo = repo.as_ref();
    let config = ProcessConfig {
        command: format!(
            "git update-ref {} {}",
            shell_quote(&format!("refs/heads/{branch}")),
            shell_quote(commit)
        ),
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
            "git update-ref failed for branch '{branch}': {}",
            output.stderr
        )));
    }
    Ok(())
}

/// Reset the working tree and index to match HEAD.
///
/// Runs `git reset --hard` in the given directory. This is useful after
/// advancing a branch pointer with `git_fast_forward` to sync the working
/// tree with the new commit.
///
/// # Arguments
/// * `path` - Path to the git working tree to reset
///
/// # Errors
/// Returns `GitError` if the git command fails.
pub fn git_reset_hard(path: impl AsRef<Path>) -> Result<(), GitError> {
    let path = path.as_ref();
    let config = ProcessConfig {
        command: "git reset --hard".to_string(),
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
            "git reset --hard failed in '{}': {}",
            path.display(),
            output.stderr
        )));
    }
    Ok(())
}

/// Delete a git ref.
///
/// Runs `git update-ref -d <ref_name>` to remove a ref.
///
/// # Arguments
/// * `repo`     - Path to the git repository
/// * `ref_name` - Full ref to delete (e.g. `refs/merge-temp/my-scion`)
///
/// # Errors
/// Returns `GitError` if the ref does not exist or the git command fails.
pub fn git_delete_ref(repo: impl AsRef<Path>, ref_name: &str) -> Result<(), GitError> {
    let repo = repo.as_ref();
    let config = ProcessConfig {
        command: format!("git update-ref -d {}", shell_quote(ref_name)),
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
            "git update-ref -d failed for ref '{ref_name}': {}",
            output.stderr
        )));
    }
    Ok(())
}

/// Get a compact diff summary between two refs.
///
/// Runs `git diff --stat <base>...<head>`.
///
/// # Arguments
/// * `repo` - Path to the git repository
/// * `base` - Base ref (e.g. "main")
/// * `head` - Head ref (e.g. "feature/my-feature")
///
/// # Errors
/// Returns `GitError` if the git command fails.
pub fn git_diff_stat(repo: impl AsRef<Path>, base: &str, head: &str) -> Result<String, GitError> {
    let repo = repo.as_ref();
    let config = ProcessConfig {
        command: format!(
            "git diff --stat {}...{}",
            shell_quote(base),
            shell_quote(head)
        ),
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
            "git diff --stat failed: {}",
            output.stderr
        )));
    }
    Ok(output.stdout.trim().to_string())
}

/// Get the full diff between two refs.
///
/// Runs `git diff <base>...<head>`.
///
/// # Arguments
/// * `repo` - Path to the git repository
/// * `base` - Base ref (e.g. "main")
/// * `head` - Head ref (e.g. "feature/my-feature")
///
/// # Errors
/// Returns `GitError` if the git command fails.
pub fn git_diff_output(repo: impl AsRef<Path>, base: &str, head: &str) -> Result<String, GitError> {
    let repo = repo.as_ref();
    let config = ProcessConfig {
        command: format!("git diff {}...{}", shell_quote(base), shell_quote(head)),
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
            "git diff failed: {}",
            output.stderr
        )));
    }
    Ok(output.stdout.trim().to_string())
}

/// Get a oneline commit log between two refs.
///
/// Runs `git log <base>..<head> --oneline`.
///
/// # Arguments
/// * `repo` - Path to the git repository
/// * `base` - Base ref (e.g. "main")
/// * `head` - Head ref (e.g. "feature/my-feature")
///
/// # Errors
/// Returns `GitError` if the git command fails.
pub fn git_log_output(repo: impl AsRef<Path>, base: &str, head: &str) -> Result<String, GitError> {
    let repo = repo.as_ref();
    let config = ProcessConfig {
        command: format!(
            "git log {}..{} --oneline",
            shell_quote(base),
            shell_quote(head)
        ),
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
            "git log failed: {}",
            output.stderr
        )));
    }
    Ok(output.stdout.trim().to_string())
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

    /// Create a helper that makes an additional commit in the repo at `path`.
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
    fn git_ahead_behind_same_branch_is_zero_zero() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path()).unwrap();

        let worktrees = git_worktree_list(temp.path()).unwrap();
        let main_branch = worktrees[0].branch.clone().unwrap();

        let (ahead, behind) = git_ahead_behind(temp.path(), &main_branch, &main_branch).unwrap();
        assert_eq!(ahead, 0);
        assert_eq!(behind, 0);
    }

    #[test]
    fn git_ahead_behind_feature_ahead_of_main() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path()).unwrap();

        // Get main branch name
        let worktrees = git_worktree_list(temp.path()).unwrap();
        let main_branch = worktrees[0].branch.clone().unwrap();

        // Create a feature branch and add 2 commits
        Command::new("git")
            .args(["checkout", "-b", "feature/ahead"])
            .current_dir(temp.path())
            .output()
            .unwrap();
        make_commit(temp.path(), "a.txt", "feat: a");
        make_commit(temp.path(), "b.txt", "feat: b");

        let (ahead, behind) = git_ahead_behind(temp.path(), "feature/ahead", &main_branch).unwrap();
        assert_eq!(ahead, 2);
        assert_eq!(behind, 0);
    }

    #[test]
    fn git_ahead_behind_diverged_branches() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path()).unwrap();

        let worktrees = git_worktree_list(temp.path()).unwrap();
        let main_branch = worktrees[0].branch.clone().unwrap();

        // Add 1 commit to feature
        Command::new("git")
            .args(["checkout", "-b", "feature/diverged"])
            .current_dir(temp.path())
            .output()
            .unwrap();
        make_commit(temp.path(), "feature.txt", "feat: feature");

        // Go back to main and add 2 commits
        Command::new("git")
            .args(["checkout", &main_branch])
            .current_dir(temp.path())
            .output()
            .unwrap();
        make_commit(temp.path(), "main1.txt", "chore: main 1");
        make_commit(temp.path(), "main2.txt", "chore: main 2");

        let (ahead, behind) =
            git_ahead_behind(temp.path(), "feature/diverged", &main_branch).unwrap();
        assert_eq!(ahead, 1);
        assert_eq!(behind, 2);
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

    #[test]
    fn git_last_commit_time_returns_timestamp() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path()).unwrap();

        let worktrees = git_worktree_list(temp.path()).unwrap();
        let branch = worktrees[0].branch.clone().unwrap();

        let ts = git_last_commit_time(temp.path(), &branch).unwrap();
        // Timestamp should be in a reasonable range (after 2020-01-01)
        assert!(ts > 1_577_836_800);
    }

    #[test]
    fn git_last_commit_time_fails_for_nonexistent_branch() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path()).unwrap();

        let result = git_last_commit_time(temp.path(), "no-such-branch");
        assert!(result.is_err());
    }

    #[test]
    fn git_is_dirty_clean_repo_returns_false() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path()).unwrap();

        let dirty = git_is_dirty(temp.path()).unwrap();
        assert!(!dirty);
    }

    #[test]
    fn git_is_dirty_with_modified_file_returns_true() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path()).unwrap();

        // Modify a tracked file without staging
        fs::write(temp.path().join("README.md"), "modified content").unwrap();

        let dirty = git_is_dirty(temp.path()).unwrap();
        assert!(dirty);
    }

    #[test]
    fn git_is_dirty_with_new_untracked_file_returns_true() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path()).unwrap();

        fs::write(temp.path().join("untracked.txt"), "new file").unwrap();

        let dirty = git_is_dirty(temp.path()).unwrap();
        assert!(dirty);
    }

    #[test]
    fn git_merge_to_ref_clean_merge() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path()).unwrap();

        let worktrees = git_worktree_list(temp.path()).unwrap();
        let main_branch = worktrees[0].branch.clone().unwrap();

        // Create a feature branch with a non-conflicting change
        Command::new("git")
            .args(["checkout", "-b", "feature/clean"])
            .current_dir(temp.path())
            .output()
            .unwrap();
        make_commit(temp.path(), "feature.txt", "feat: add feature file");

        // Go back to main
        Command::new("git")
            .args(["checkout", &main_branch])
            .current_dir(temp.path())
            .output()
            .unwrap();

        // Merge to a temp ref
        let commit = git_merge_to_ref(
            temp.path(),
            "feature/clean",
            &main_branch,
            "refs/merge-temp/test",
        )
        .unwrap();

        // The merge commit should be a valid hash
        assert_eq!(commit.len(), 40);
        assert!(commit.chars().all(|c| c.is_ascii_hexdigit()));

        // The temp ref should exist and point to the merge commit
        let config = ProcessConfig {
            command: "git rev-parse refs/merge-temp/test".to_string(),
            working_dir: temp.path().to_path_buf(),
            env: None,
            env_remove: vec![],
            log_path: None,
            timeout: Some(Duration::from_secs(5)),
            stdin: None,
        };
        let output = run_to_completion_with_timeout(&config).unwrap();
        assert_eq!(output.stdout.trim(), commit);

        // Main should NOT have moved
        let main_commit = get_current_commit(temp.path()).unwrap();
        assert_ne!(main_commit, commit);
    }

    #[test]
    fn git_merge_to_ref_conflict_returns_error() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path()).unwrap();

        let worktrees = git_worktree_list(temp.path()).unwrap();
        let main_branch = worktrees[0].branch.clone().unwrap();

        // Create a feature branch that modifies the same file
        Command::new("git")
            .args(["checkout", "-b", "feature/conflict"])
            .current_dir(temp.path())
            .output()
            .unwrap();
        fs::write(temp.path().join("README.md"), "feature version").unwrap();
        Command::new("git")
            .args(["add", "README.md"])
            .current_dir(temp.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "feat: modify readme"])
            .current_dir(temp.path())
            .output()
            .unwrap();

        // Go back to main and modify the same file differently
        Command::new("git")
            .args(["checkout", &main_branch])
            .current_dir(temp.path())
            .output()
            .unwrap();
        fs::write(temp.path().join("README.md"), "main version").unwrap();
        Command::new("git")
            .args(["add", "README.md"])
            .current_dir(temp.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "chore: modify readme on main"])
            .current_dir(temp.path())
            .output()
            .unwrap();

        // Merge should fail with conflict
        let result = git_merge_to_ref(
            temp.path(),
            "feature/conflict",
            &main_branch,
            "refs/merge-temp/conflict",
        );
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Merge conflicts"),
            "Expected merge conflict error, got: {err_msg}"
        );
    }

    #[test]
    fn git_fast_forward_advances_branch() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path()).unwrap();

        let worktrees = git_worktree_list(temp.path()).unwrap();
        let main_branch = worktrees[0].branch.clone().unwrap();

        // Create a feature branch with a commit
        Command::new("git")
            .args(["checkout", "-b", "feature/ff"])
            .current_dir(temp.path())
            .output()
            .unwrap();
        make_commit(temp.path(), "ff.txt", "feat: ff commit");
        let feature_commit = get_current_commit(temp.path()).unwrap();

        // Go back to main
        Command::new("git")
            .args(["checkout", &main_branch])
            .current_dir(temp.path())
            .output()
            .unwrap();
        let main_before = get_current_commit(temp.path()).unwrap();

        // Fast-forward main to the feature commit
        git_fast_forward(temp.path(), &main_branch, &feature_commit).unwrap();

        // Verify main now points to the feature commit
        let config = ProcessConfig {
            command: format!("git rev-parse refs/heads/{main_branch}"),
            working_dir: temp.path().to_path_buf(),
            env: None,
            env_remove: vec![],
            log_path: None,
            timeout: Some(Duration::from_secs(5)),
            stdin: None,
        };
        let output = run_to_completion_with_timeout(&config).unwrap();
        assert_eq!(output.stdout.trim(), feature_commit);
        assert_ne!(main_before, feature_commit);
    }

    #[test]
    fn git_delete_ref_removes_ref() {
        let temp = TempDir::new().unwrap();
        init_test_repo(temp.path()).unwrap();

        let commit = get_current_commit(temp.path()).unwrap();

        // Create a ref
        let config = ProcessConfig {
            command: format!("git update-ref refs/test/temp {commit}"),
            working_dir: temp.path().to_path_buf(),
            env: None,
            env_remove: vec![],
            log_path: None,
            timeout: Some(Duration::from_secs(5)),
            stdin: None,
        };
        run_to_completion_with_timeout(&config).unwrap();

        // Delete it
        git_delete_ref(temp.path(), "refs/test/temp").unwrap();

        // Verify it's gone
        let config = ProcessConfig {
            command: "git rev-parse refs/test/temp".to_string(),
            working_dir: temp.path().to_path_buf(),
            env: None,
            env_remove: vec![],
            log_path: None,
            timeout: Some(Duration::from_secs(5)),
            stdin: None,
        };
        let output = run_to_completion_with_timeout(&config).unwrap();
        assert!(!output.success);
    }

    #[test]
    fn operations_work_with_spaces_in_path() {
        let temp = TempDir::new().unwrap();
        let spaced_path = temp.path().join("repo with spaces");
        fs::create_dir_all(&spaced_path).unwrap();
        init_test_repo(&spaced_path).unwrap();

        // get_current_commit uses working_dir (no interpolation risk), but
        // git_is_dirty and git_has_tracked_changes use -C with the path.
        let commit = get_current_commit(&spaced_path).unwrap();
        assert_eq!(commit.len(), 40);

        assert!(!git_is_dirty(&spaced_path).unwrap());
        assert!(!git_has_tracked_changes(&spaced_path).unwrap());

        // Worktree operations interpolate paths and branch names.
        let wt_path = spaced_path.join("worktree dir");
        let returned = git_worktree_add(&spaced_path, &wt_path, "feature/spaced-test").unwrap();
        assert!(returned.exists());

        let worktrees = git_worktree_list(&spaced_path).unwrap();
        assert_eq!(worktrees.len(), 2);

        git_worktree_remove(&spaced_path, &wt_path).unwrap();
        let worktrees = git_worktree_list(&spaced_path).unwrap();
        assert_eq!(worktrees.len(), 1);

        git_branch_delete(&spaced_path, "feature/spaced-test").unwrap();
    }
}
