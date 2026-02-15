///! Reading cached state query results from graft cache directory.

use super::query::StateResult;
use std::fs;
use std::path::PathBuf;

/// Read a cached state query result for a specific query and commit.
///
/// Cache location: `~/.cache/graft/{workspace_hash}/{repo_name}/state/{query_name}/{commit_hash}.json`
pub fn read_cached_state(
    workspace_hash: &str,
    repo_name: &str,
    query_name: &str,
    commit_hash: &str,
) -> Result<StateResult, String> {
    let cache_path = get_cache_path(workspace_hash, repo_name, query_name, commit_hash);

    let content = fs::read_to_string(&cache_path).map_err(|e| {
        format!(
            "Failed to read cache file {}: {}",
            cache_path.display(),
            e
        )
    })?;

    serde_json::from_str(&content).map_err(|e| {
        format!(
            "Failed to parse cache file {}: {}",
            cache_path.display(),
            e
        )
    })
}

/// Get all cached results for a query across all commits.
pub fn read_all_cached_for_query(
    workspace_hash: &str,
    repo_name: &str,
    query_name: &str,
) -> Result<Vec<StateResult>, String> {
    let query_dir = get_query_cache_dir(workspace_hash, repo_name, query_name);

    if !query_dir.exists() {
        return Ok(Vec::new());
    }

    let mut results: Vec<StateResult> = Vec::new();

    for entry in fs::read_dir(&query_dir)
        .map_err(|e| format!("Failed to read query cache dir: {}", e))?
    {
        let entry = entry.map_err(|e| format!("Failed to read dir entry: {}", e))?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            match fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str(&content) {
                    Ok(result) => results.push(result),
                    Err(_) => continue, // Skip invalid cache files
                },
                Err(_) => continue, // Skip unreadable files
            }
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
        .ok_or_else(|| format!("No cached results found for query: {}", query_name))
}

/// Get the cache file path for a specific query and commit.
fn get_cache_path(
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
        .join(format!("{}.json", commit_hash))
}

/// Get the query cache directory (contains all commits for this query).
fn get_query_cache_dir(workspace_hash: &str, repo_name: &str, query_name: &str) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home)
        .join(".cache/graft")
        .join(workspace_hash)
        .join(repo_name)
        .join("state")
        .join(query_name)
}

/// Compute workspace hash (SHA256 of workspace name).
pub fn compute_workspace_hash(workspace_name: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(workspace_name.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)[..16].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_workspace_hash() {
        let hash = compute_workspace_hash("my-workspace");
        assert_eq!(hash.len(), 16); // First 16 chars of SHA256
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_get_cache_path() {
        let path = get_cache_path("abc123", "my-repo", "coverage", "def456");
        let path_str = path.to_string_lossy();

        assert!(path_str.contains(".cache/graft"));
        assert!(path_str.contains("abc123"));
        assert!(path_str.contains("my-repo"));
        assert!(path_str.contains("state"));
        assert!(path_str.contains("coverage"));
        assert!(path_str.ends_with("def456.json"));
    }
}
