//! Reading cached state query results from graft cache directory.
//!
//! This module provides grove-specific wrappers that accept pre-computed workspace hashes
//! for backward compatibility with existing code and tests.
use super::query::StateResult;
use std::fs;
use std::path::PathBuf;

// Re-export compute_workspace_hash directly (same signature)
pub use graft_common::state::compute_workspace_hash;

/// Get cache path given a pre-computed workspace hash.
#[allow(dead_code)]
fn get_cache_path_from_hash(
    workspace_hash: &str,
    repo_name: &str,
    query_name: &str,
    commit_hash: &str,
) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home)
        .join(".cache/graft")
        .join(workspace_hash)
        .join(repo_name)
        .join("state")
        .join(query_name)
        .join(format!("{commit_hash}.json"))
}

/// Get query cache directory given a pre-computed workspace hash.
fn get_query_cache_dir_from_hash(
    workspace_hash: &str,
    repo_name: &str,
    query_name: &str,
) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home)
        .join(".cache/graft")
        .join(workspace_hash)
        .join(repo_name)
        .join("state")
        .join(query_name)
}

/// Read a cached state query result for a specific query and commit.
///
/// Cache location: `~/.cache/graft/{workspace_hash}/{repo_name}/state/{query_name}/{commit_hash}.json`
#[allow(dead_code)]
pub fn read_cached_state(
    workspace_hash: &str,
    repo_name: &str,
    query_name: &str,
    commit_hash: &str,
) -> Result<StateResult, String> {
    let cache_path = get_cache_path_from_hash(workspace_hash, repo_name, query_name, commit_hash);

    if !cache_path.exists() {
        return Err(format!("No cached result found for query: {query_name}"));
    }

    let content = fs::read_to_string(&cache_path)
        .map_err(|e| format!("Failed to read cache file {}: {e}", cache_path.display()))?;

    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse cache file {}: {e}", cache_path.display()))
}

/// Get all cached results for a query across all commits.
pub fn read_all_cached_for_query(
    workspace_hash: &str,
    repo_name: &str,
    query_name: &str,
) -> Result<Vec<StateResult>, String> {
    let query_dir = get_query_cache_dir_from_hash(workspace_hash, repo_name, query_name);

    if !query_dir.exists() {
        return Ok(Vec::new());
    }

    let mut results: Vec<StateResult> = Vec::new();

    for entry in
        fs::read_dir(&query_dir).map_err(|e| format!("Failed to read query cache dir: {e}"))?
    {
        let entry = entry.map_err(|e| format!("Failed to read dir entry: {e}"))?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(result) = serde_json::from_str(&content) {
                    results.push(result);
                }
                // Skip invalid cache files
            }
            // Skip unreadable files
        }
    }

    // Sort by timestamp (newest first)
    results.sort_by(|a, b| {
        b.metadata
            .timestamp_parsed()
            .cmp(&a.metadata.timestamp_parsed())
    });

    Ok(results)
}

/// Find the most recent cached result for a query.
pub fn read_latest_cached(
    workspace_hash: &str,
    repo_name: &str,
    query_name: &str,
) -> Result<StateResult, String> {
    let results = read_all_cached_for_query(workspace_hash, repo_name, query_name)?;

    results
        .into_iter()
        .next()
        .ok_or_else(|| format!("No cached results found for query: {query_name}"))
}
