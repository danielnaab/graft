//! State query execution and caching.
//!
//! This module implements the state query infrastructure that works with any domain.
//! It provides:
//! - Query execution (subprocess commands)
//! - Commit-based caching
//! - Cache invalidation
//!
//! This has NO domain-specific knowledge. It treats all state queries as:
//!     command → JSON output → cache by commit hash

use graft_core::{GraftError, Result, StateQuery};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

/// Metadata for a state query result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMetadata {
    pub query_name: String,
    pub commit_hash: String,
    pub timestamp: String, // ISO 8601 format
    pub command: String,
    pub deterministic: bool,
}

/// A state query result with data and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateResult {
    pub metadata: StateMetadata,
    pub data: Value,
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
    // Compute workspace hash to avoid path conflicts
    let mut hasher = Sha256::new();
    hasher.update(workspace_name.as_bytes());
    let hash_result = hasher.finalize();
    let workspace_hash = format!("{hash_result:x}")[..16].to_string();

    // Build cache path
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home)
        .join(".cache/graft")
        .join(workspace_hash)
        .join(repo_name)
        .join("state")
        .join(query_name)
        .join(format!("{commit_hash}.json"))
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
                    &commit_hash[..7]
                );
                // Delete corrupted cache file
                let _ = fs::remove_file(&cache_path);
                None
            }
        },
        Err(_) => None,
    }
}

/// Write state result to cache.
pub fn write_cached_state(
    workspace_name: &str,
    repo_name: &str,
    result: &StateResult,
) -> Result<()> {
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
    let content = serde_json::to_string_pretty(result)
        .map_err(|e| GraftError::Validation(format!("Failed to serialize cache: {e}")))?;

    fs::write(&cache_path, content)?;

    Ok(())
}

/// Invalidate cached state for a query or all queries.
///
/// Returns the number of cache files deleted.
pub fn invalidate_cached_state(
    workspace_name: &str,
    repo_name: &str,
    query_name: Option<&str>,
) -> Result<usize> {
    let mut hasher = Sha256::new();
    hasher.update(workspace_name.as_bytes());
    let hash_result = hasher.finalize();
    let workspace_hash = format!("{hash_result:x}")[..16].to_string();

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

/// Execute a state query and return the result.
///
/// # Security Model
///
/// This function uses shell execution which allows shell features like pipes,
/// redirects, and variable expansion. This is safe because:
///
/// 1. Commands come from user's own graft.yaml config file
/// 2. Users version-control and review their own commands
/// 3. Similar trust model to Makefile, package.json, .bashrc
/// 4. No remote or untrusted input is executed
pub fn execute_state_query(
    query: &StateQuery,
    repo_path: &Path,
    commit_hash: &str,
) -> Result<StateResult> {
    let _timeout_seconds = query.timeout.unwrap_or(300); // Default 5 minutes

    // Execute command in repo directory
    // SECURITY: Commands from user's graft.yaml (trusted source).
    let output = ProcessCommand::new("sh")
        .arg("-c")
        .arg(&query.run)
        .current_dir(repo_path)
        .output()
        .map_err(|e| GraftError::CommandExecution(format!("Failed to execute command: {e}")))?;

    // Check exit code
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let exit_code = output.status.code().unwrap_or(-1);

        let mut error_msg = format!(
            "State query '{}' failed with exit code {exit_code}",
            query.name
        );
        if !stderr.is_empty() {
            error_msg.push_str("\nstderr: ");
            error_msg.push_str(&stderr);
        }
        if !stdout.is_empty() {
            error_msg.push_str("\nstdout: ");
            error_msg.push_str(&stdout);
        }

        return Err(GraftError::CommandExecution(error_msg));
    }

    // Parse JSON output
    let stdout = String::from_utf8_lossy(&output.stdout);
    let data: Value = serde_json::from_str(&stdout).map_err(|e| {
        let preview = if stdout.len() > 500 {
            &stdout[..500]
        } else {
            &stdout
        };
        GraftError::Validation(format!(
            "State query '{}' output is not valid JSON: {e}\nOutput was: {preview}",
            query.name
        ))
    })?;

    // Validate it's a JSON object (dict)
    if !data.is_object() {
        return Err(GraftError::Validation(format!(
            "State query '{}' must output a JSON object, got {}",
            query.name,
            if data.is_array() {
                "array"
            } else if data.is_null() {
                "null"
            } else {
                "primitive"
            }
        )));
    }

    // Create result
    let timestamp = chrono::Utc::now().to_rfc3339();

    Ok(StateResult {
        metadata: StateMetadata {
            query_name: query.name.clone(),
            commit_hash: commit_hash.to_string(),
            timestamp,
            command: query.run.clone(),
            deterministic: query.cache.deterministic,
        },
        data,
    })
}

/// Get state query result, using cache if available.
pub fn get_state(
    query: &StateQuery,
    workspace_name: &str,
    repo_name: &str,
    repo_path: &Path,
    commit_hash: &str,
    refresh: bool,
) -> Result<StateResult> {
    // Check cache (unless refresh requested)
    if !refresh && query.cache.deterministic {
        if let Some(cached) = read_cached_state(workspace_name, repo_name, &query.name, commit_hash)
        {
            return Ok(cached);
        }
    }

    // Execute query
    let result = execute_state_query(query, repo_path, commit_hash)?;

    // Cache result if deterministic
    if result.metadata.deterministic {
        write_cached_state(workspace_name, repo_name, &result)?;
    }

    Ok(result)
}

/// List all state queries with cache status.
pub fn list_state_queries<S: ::std::hash::BuildHasher>(
    queries: &HashMap<String, StateQuery, S>,
    workspace_name: &str,
    repo_name: &str,
    commit_hash: &str,
) -> Vec<StateQueryStatus> {
    queries
        .iter()
        .map(|(name, query)| {
            let cached = read_cached_state(workspace_name, repo_name, name, commit_hash);

            StateQueryStatus {
                name: name.clone(),
                command: query.run.clone(),
                cached: cached.is_some(),
                cache_timestamp: cached.as_ref().map(|r| r.metadata.timestamp.clone()),
            }
        })
        .collect()
}

/// Status information for a state query.
pub struct StateQueryStatus {
    pub name: String,
    pub command: String,
    pub cached: bool,
    pub cache_timestamp: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use graft_core::StateCache;

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

    #[test]
    fn test_execute_state_query_success() {
        let query = StateQuery::new("test", "echo '{\"result\": \"ok\"}'")
            .unwrap()
            .with_cache(StateCache {
                deterministic: true,
            });

        let result = execute_state_query(&query, Path::new("/tmp"), "abc123").unwrap();

        assert_eq!(result.metadata.query_name, "test");
        assert_eq!(result.metadata.commit_hash, "abc123");
        assert_eq!(result.data["result"], "ok");
    }

    #[test]
    fn test_execute_state_query_invalid_json() {
        let query = StateQuery::new("test", "echo 'not json'").unwrap();

        let result = execute_state_query(&query, Path::new("/tmp"), "abc123");

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not valid JSON"));
    }

    #[test]
    fn test_execute_state_query_non_object() {
        let query = StateQuery::new("test", "echo '\"string\"'").unwrap();

        let result = execute_state_query(&query, Path::new("/tmp"), "abc123");

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must output a JSON object"));
    }

    #[test]
    fn test_execute_state_query_array() {
        let query = StateQuery::new("test", "echo '[]'").unwrap();

        let result = execute_state_query(&query, Path::new("/tmp"), "abc123");

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("got array"));
    }
}
