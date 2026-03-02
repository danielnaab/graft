//! State query result types and cache management.
//!
//! This module provides shared types for state query results and cache management
//! used by both graft and grove.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

/// Metadata for a state query result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMetadata {
    pub query_name: String,
    /// The cache key used to store this result.
    /// May be a commit hash (clean tree) or a SHA256 content hash (dirty tree).
    pub commit_hash: String,
    pub timestamp: String, // ISO 8601 format
    pub command: String,
}

impl StateMetadata {
    /// Parse timestamp as `DateTime`.
    pub fn timestamp_parsed(&self) -> Option<DateTime<Utc>> {
        DateTime::parse_from_rfc3339(&self.timestamp)
            .ok()
            .map(|dt| dt.with_timezone(&Utc))
    }

    /// Get human-readable time ago string.
    pub fn time_ago(&self) -> String {
        crate::format_time_ago(&self.timestamp)
    }
}

/// A state query result with data and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateResult {
    pub metadata: StateMetadata,
    pub data: Value,
}

/// Compute workspace hash (SHA256 of workspace name, first 16 hex chars).
pub fn compute_workspace_hash(workspace_name: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(workspace_name.as_bytes());
    let result = hasher.finalize();
    format!("{result:x}")[..16].to_string()
}

/// Get cache file path for a state query result.
///
/// Format: `~/.cache/graft/{workspace-hash}/{repo-name}/state/{query-name}/{commit-hash}.json`
pub fn get_cache_path(
    workspace_name: &str,
    repo_name: &str,
    query_name: &str,
    commit_hash: &str,
) -> PathBuf {
    let workspace_hash = compute_workspace_hash(workspace_name);
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());

    PathBuf::from(home)
        .join(".cache/graft")
        .join(workspace_hash)
        .join(repo_name)
        .join("state")
        .join(query_name)
        .join(format!("{commit_hash}.json"))
}

/// Get the query cache directory (contains all commits for this query).
pub fn get_query_cache_dir(workspace_name: &str, repo_name: &str, query_name: &str) -> PathBuf {
    let workspace_hash = compute_workspace_hash(workspace_name);
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());

    PathBuf::from(home)
        .join(".cache/graft")
        .join(workspace_hash)
        .join(repo_name)
        .join("state")
        .join(query_name)
}

/// Read cached state result if it exists.
pub fn read_cached_state(
    workspace_name: &str,
    repo_name: &str,
    query_name: &str,
    commit_hash: &str,
) -> Option<StateResult> {
    let cache_path = get_cache_path(workspace_name, repo_name, query_name, commit_hash);

    if !cache_path.exists() {
        return None;
    }

    match fs::read_to_string(&cache_path) {
        Ok(content) => match serde_json::from_str(&content) {
            Ok(result) => Some(result),
            Err(e) => {
                eprintln!(
                    "Warning: Corrupted cache for {query_name} at commit {}: {e}",
                    &commit_hash[..7.min(commit_hash.len())]
                );
                // Delete corrupted cache file
                let _ = fs::remove_file(&cache_path);
                None
            }
        },
        Err(_) => None,
    }
}

/// Get all cached results for a query across all commits.
pub fn read_all_cached_for_query(
    workspace_name: &str,
    repo_name: &str,
    query_name: &str,
) -> Vec<StateResult> {
    let query_dir = get_query_cache_dir(workspace_name, repo_name, query_name);

    if !query_dir.exists() {
        return Vec::new();
    }

    let mut results = Vec::new();

    let Ok(entries) = fs::read_dir(&query_dir) else {
        return results;
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(result) = serde_json::from_str(&content) {
                    results.push(result);
                }
            }
        }
    }

    // Sort by timestamp (newest first)
    results.sort_by(|a, b| {
        b.metadata
            .timestamp_parsed()
            .cmp(&a.metadata.timestamp_parsed())
    });

    results
}

/// Find the most recent cached result for a query.
pub fn read_latest_cached(
    workspace_name: &str,
    repo_name: &str,
    query_name: &str,
) -> Option<StateResult> {
    read_all_cached_for_query(workspace_name, repo_name, query_name)
        .into_iter()
        .next()
}

/// Write state result to cache.
pub fn write_cached_state(
    workspace_name: &str,
    repo_name: &str,
    result: &StateResult,
) -> std::io::Result<()> {
    let cache_path = get_cache_path(
        workspace_name,
        repo_name,
        &result.metadata.query_name,
        &result.metadata.commit_hash,
    );

    // Ensure directory exists
    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write cache file
    let content = serde_json::to_string_pretty(result)?;
    fs::write(&cache_path, content)?;

    Ok(())
}

/// Count files recursively in a directory.
fn count_files_recursive(dir: &Path) -> usize {
    fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(std::result::Result::ok)
                .map(|entry| {
                    let path = entry.path();
                    if path.is_dir() {
                        count_files_recursive(&path)
                    } else {
                        1
                    }
                })
                .sum()
        })
        .unwrap_or(0)
}

/// Invalidate cached state for a query or all queries.
///
/// Returns the number of cache files deleted.
pub fn invalidate_cached_state(
    workspace_name: &str,
    repo_name: &str,
    query_name: Option<&str>,
) -> std::io::Result<usize> {
    let workspace_hash = compute_workspace_hash(workspace_name);
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let cache_root = PathBuf::from(home)
        .join(".cache/graft")
        .join(workspace_hash)
        .join(repo_name)
        .join("state");

    if let Some(query_name) = query_name {
        // Delete specific query cache
        let query_dir = cache_root.join(query_name);
        if query_dir.exists() {
            let count = fs::read_dir(&query_dir)
                .map(|entries| entries.filter_map(std::result::Result::ok).count())
                .unwrap_or(0);

            fs::remove_dir_all(&query_dir)?;
            Ok(count)
        } else {
            Ok(0)
        }
    } else {
        // Delete entire state cache directory
        if cache_root.exists() {
            let count = count_files_recursive(&cache_root);
            fs::remove_dir_all(&cache_root)?;
            Ok(count)
        } else {
            Ok(0)
        }
    }
}

use crate::process::shell_quote;

/// Compute the cache key for a state query given its declared input file globs.
///
/// Returns:
/// - `None` if `inputs` is empty (always run fresh, never cache)
/// - The current `commit_hash` if `git status --porcelain -- {inputs}` shows no local edits
/// - A SHA256 content hash of all tracked input files if the working tree is dirty
///
/// Callers should pass the current commit hash (from `git rev-parse HEAD`) so the
/// clean-tree path avoids a second git invocation.
pub fn compute_input_cache_key(
    inputs: &[String],
    repo_path: &Path,
    commit_hash: &str,
) -> Option<String> {
    if inputs.is_empty() {
        return None;
    }

    // Shell-quote each pattern to prevent word-splitting and glob expansion by the shell.
    let quoted = inputs
        .iter()
        .map(|s| shell_quote(s))
        .collect::<Vec<_>>()
        .join(" ");

    // Build `git status --porcelain -- <inputs>` to check for local edits
    let status_cmd = format!("git status --porcelain -- {quoted}");
    let config = crate::process::ProcessConfig {
        command: status_cmd,
        working_dir: repo_path.to_path_buf(),
        env: None,
        env_remove: vec![],
        log_path: None,
        timeout: Some(std::time::Duration::from_secs(30)),
        stdin: None,
    };

    let output = crate::process::run_to_completion_with_timeout(&config).ok()?;
    if !output.success {
        // git failed (e.g. not a git repo) — can't determine cleanliness, so don't cache.
        return None;
    }

    if output.stdout.trim().is_empty() {
        // Clean tree: commit hash is sufficient
        return Some(commit_hash.to_string());
    }

    // Dirty tree: hash content of all tracked input files for a stable content key
    let ls_cmd = format!("git ls-files -- {quoted}");
    let ls_config = crate::process::ProcessConfig {
        command: ls_cmd,
        working_dir: repo_path.to_path_buf(),
        env: None,
        env_remove: vec![],
        log_path: None,
        timeout: Some(std::time::Duration::from_secs(30)),
        stdin: None,
    };

    let ls_output = crate::process::run_to_completion_with_timeout(&ls_config).ok()?;
    if !ls_output.success {
        return None;
    }

    let mut hasher = Sha256::new();
    for file in ls_output.stdout.lines() {
        let filename = file.trim();
        // Include the filename so that content-swaps between files produce different keys.
        hasher.update(filename.as_bytes());
        hasher.update(b"\0");
        let file_path = repo_path.join(filename);
        match fs::read(&file_path) {
            Ok(content) => {
                // Length-prefix the content. No valid file can have length u64::MAX, so
                // present files and deleted files are always distinguishable regardless of
                // what bytes the file contains (including the literal string "<deleted>").
                hasher.update((content.len() as u64).to_le_bytes());
                hasher.update(&content);
            }
            Err(_) => {
                // File is tracked (in index) but deleted from disk.
                // u64::MAX is an impossible valid file length — unambiguous tombstone.
                hasher.update(u64::MAX.to_le_bytes());
            }
        }
        // Null separator so file entries don't bleed into each other.
        hasher.update(b"\0");
    }
    let hash = hasher.finalize();
    Some(format!("{hash:x}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_workspace_hash() {
        let hash = compute_workspace_hash("my-workspace");
        assert_eq!(hash.len(), 16); // First 16 chars of SHA256
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));

        // Same input produces same hash
        let hash2 = compute_workspace_hash("my-workspace");
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_get_cache_path() {
        let path = get_cache_path("my-workspace", "my-repo", "coverage", "abc123");
        let path_str = path.to_string_lossy();

        assert!(path_str.contains(".cache/graft"));
        assert!(path_str.contains("my-repo"));
        assert!(path_str.contains("state"));
        assert!(path_str.contains("coverage"));
        assert!(path_str.ends_with("abc123.json"));
    }

    // ===== shell_quote =====

    #[test]
    fn test_shell_quote_simple() {
        assert_eq!(shell_quote("foo"), "'foo'");
    }

    #[test]
    fn test_shell_quote_glob() {
        assert_eq!(shell_quote("**/*.rs"), "'**/*.rs'");
    }

    #[test]
    fn test_shell_quote_empty() {
        assert_eq!(shell_quote(""), "''");
    }

    #[test]
    fn test_shell_quote_embedded_single_quote() {
        // "foo'bar" → 'foo'\''bar'
        assert_eq!(shell_quote("foo'bar"), "'foo'\\''bar'");
    }

    #[test]
    fn test_shell_quote_multiple_single_quotes() {
        // "a'b'c" → 'a'\''b'\''c'
        assert_eq!(shell_quote("a'b'c"), "'a'\\''b'\\''c'");
    }

    // ===== compute_input_cache_key =====

    #[test]
    fn test_compute_input_cache_key_empty_inputs_returns_none() {
        // Empty inputs → always run fresh, never cache. No git operation is performed,
        // so the repo_path is irrelevant; we pass a fresh temp dir for consistency.
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let result = compute_input_cache_key(&[], dir.path(), "abc123");
        assert!(result.is_none());
    }

    #[test]
    fn test_compute_input_cache_key_non_git_dir_returns_none() {
        // A fresh temp dir has no .git → git status exits non-zero → must return None.
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let result = compute_input_cache_key(&["**/*.rs".to_string()], dir.path(), "abc123");
        assert!(result.is_none());
    }

    /// Run a git command in `dir`, asserting success; return stdout.
    fn git(dir: &Path, args: &[&str]) -> String {
        let out = std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            .env("GIT_AUTHOR_NAME", "test")
            .env("GIT_AUTHOR_EMAIL", "test@test.com")
            .env("GIT_COMMITTER_NAME", "test")
            .env("GIT_COMMITTER_EMAIL", "test@test.com")
            .output()
            .unwrap_or_else(|_| panic!("git {args:?} failed to spawn"));
        assert!(
            out.status.success(),
            "git {args:?} exited {}: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    #[test]
    fn test_compute_input_cache_key_clean_tree_returns_commit() {
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let repo = dir.path();

        git(repo, &["init"]);
        fs::write(repo.join("file.rs"), b"initial content").expect("write");
        git(repo, &["add", "file.rs"]);
        git(repo, &["commit", "-m", "init", "--no-gpg-sign"]);
        let commit = git(repo, &["rev-parse", "HEAD"]);

        let result = compute_input_cache_key(&["file.rs".to_string()], repo, &commit);
        assert_eq!(
            result,
            Some(commit),
            "clean tree: cache key must equal commit hash"
        );
    }

    #[test]
    fn test_compute_input_cache_key_dirty_tree_returns_content_hash() {
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let repo = dir.path();

        git(repo, &["init"]);
        fs::write(repo.join("file.rs"), b"initial content").expect("write");
        git(repo, &["add", "file.rs"]);
        git(repo, &["commit", "-m", "init", "--no-gpg-sign"]);
        let commit = git(repo, &["rev-parse", "HEAD"]);

        // Dirty the working tree.
        fs::write(repo.join("file.rs"), b"modified content").expect("modify");

        let result = compute_input_cache_key(&["file.rs".to_string()], repo, &commit);
        let key = result.expect("dirty tracked file must yield Some");

        assert_ne!(key, commit, "dirty-tree key must not equal commit hash");
        assert_eq!(key.len(), 64, "dirty-tree key must be a 64-char SHA256 hex");
        assert!(
            key.chars().all(|c| c.is_ascii_hexdigit()),
            "dirty-tree key must be hex: {key}"
        );

        // Different content → different key.
        fs::write(repo.join("file.rs"), b"other modification").expect("modify");
        let key2 = compute_input_cache_key(&["file.rs".to_string()], repo, &commit)
            .expect("must return Some");
        assert_ne!(key, key2, "different content must produce different keys");
    }

    #[test]
    fn test_compute_input_cache_key_filename_included_in_hash() {
        // Swapping identical content between two files must produce different keys.
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let repo = dir.path();

        git(repo, &["init"]);
        fs::write(repo.join("a.rs"), b"content A").expect("write a");
        fs::write(repo.join("b.rs"), b"content B").expect("write b");
        git(repo, &["add", "a.rs", "b.rs"]);
        git(repo, &["commit", "-m", "init", "--no-gpg-sign"]);
        let commit = git(repo, &["rev-parse", "HEAD"]);

        // Dirty both files with identical content — position-in-file matters, not just bytes.
        fs::write(repo.join("a.rs"), b"SAME").expect("write");
        fs::write(repo.join("b.rs"), b"SAME").expect("write");
        let key_same =
            compute_input_cache_key(&["a.rs".to_string(), "b.rs".to_string()], repo, &commit)
                .expect("must return Some");

        // Give each file distinct content: a.rs="value X", b.rs="value Y".
        fs::write(repo.join("a.rs"), b"value X").expect("write");
        fs::write(repo.join("b.rs"), b"value Y").expect("write");
        let key_xy =
            compute_input_cache_key(&["a.rs".to_string(), "b.rs".to_string()], repo, &commit)
                .expect("must return Some");

        // Now swap content between a and b: a="value Y", b="value X"
        fs::write(repo.join("a.rs"), b"value Y").expect("write");
        fs::write(repo.join("b.rs"), b"value X").expect("write");
        let key_yx =
            compute_input_cache_key(&["a.rs".to_string(), "b.rs".to_string()], repo, &commit)
                .expect("must return Some");

        // All three keys must be distinct (filenames are part of the hash).
        assert_ne!(key_same, key_xy);
        assert_ne!(
            key_xy, key_yx,
            "swapping content between files must change the key"
        );
        assert_ne!(key_same, key_yx);
    }

    #[test]
    fn test_compute_input_cache_key_deleted_file_differs_from_empty() {
        // A tracked file deleted from disk must hash differently from the same file with
        // empty content, demonstrating the tombstone is included.
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let repo = dir.path();

        git(repo, &["init"]);
        fs::write(repo.join("file.rs"), b"content").expect("write");
        git(repo, &["add", "file.rs"]);
        git(repo, &["commit", "-m", "init", "--no-gpg-sign"]);
        let commit = git(repo, &["rev-parse", "HEAD"]);

        // Dirty: file present but with empty content.
        fs::write(repo.join("file.rs"), b"").expect("write empty");
        let key_empty = compute_input_cache_key(&["file.rs".to_string()], repo, &commit)
            .expect("must return Some");

        // Dirty: file deleted from disk (but still tracked in index via the commit).
        fs::remove_file(repo.join("file.rs")).expect("delete file");
        let key_deleted = compute_input_cache_key(&["file.rs".to_string()], repo, &commit)
            .expect("must return Some");

        assert_ne!(
            key_empty, key_deleted,
            "deleted file must hash differently from empty file"
        );
    }

    #[test]
    fn test_compute_input_cache_key_tombstone_not_confused_with_file_content() {
        // A file whose content is the old string tombstone must not hash the same as a
        // deleted file. This test would have failed before switching to the length-prefix scheme.
        let dir = tempfile::TempDir::new().expect("create temp dir");
        let repo = dir.path();

        git(repo, &["init"]);
        fs::write(repo.join("file.rs"), b"initial").expect("write");
        git(repo, &["add", "file.rs"]);
        git(repo, &["commit", "-m", "init", "--no-gpg-sign"]);
        let commit = git(repo, &["rev-parse", "HEAD"]);

        // Dirty: file exists with content that was the old tombstone string.
        fs::write(repo.join("file.rs"), b"<deleted>").expect("write tombstone bytes");
        let key_content = compute_input_cache_key(&["file.rs".to_string()], repo, &commit)
            .expect("must return Some");

        // Dirty: file actually deleted from disk.
        fs::remove_file(repo.join("file.rs")).expect("delete file");
        let key_deleted = compute_input_cache_key(&["file.rs".to_string()], repo, &commit)
            .expect("must return Some");

        assert_ne!(
            key_content, key_deleted,
            "file containing b\"<deleted>\" must not hash the same as a deleted file"
        );
    }

    #[test]
    fn test_time_ago_formats_correctly() {
        let metadata = StateMetadata {
            query_name: "test".to_string(),
            commit_hash: "abc123".to_string(),
            timestamp: Utc::now().to_rfc3339(),
            command: "test".to_string(),
        };

        let time_ago = metadata.time_ago();
        assert_eq!(time_ago, "just now");
    }
}
