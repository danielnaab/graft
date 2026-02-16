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
    pub commit_hash: String,
    pub timestamp: String, // ISO 8601 format
    pub command: String,
    pub deterministic: bool,
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
        match self.timestamp_parsed() {
            Some(ts) => {
                let now = Utc::now();
                let duration = now.signed_duration_since(ts);

                if duration.num_seconds() < 60 {
                    "just now".to_string()
                } else if duration.num_minutes() < 60 {
                    let mins = duration.num_minutes();
                    format!("{mins}m ago")
                } else if duration.num_hours() < 24 {
                    let hours = duration.num_hours();
                    format!("{hours}h ago")
                } else {
                    let days = duration.num_days();
                    format!("{days}d ago")
                }
            }
            None => "unknown".to_string(),
        }
    }
}

/// A state query result with data and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateResult {
    pub metadata: StateMetadata,
    pub data: Value,
}

impl StateResult {
    /// Get a summary string for display (type-specific formatting).
    pub fn summary(&self) -> String {
        // Try to format based on common query types
        if let Some(obj) = self.data.as_object() {
            // Writing metrics
            if let (Some(total_words), Some(words_today)) =
                (obj.get("total_words"), obj.get("words_today"))
            {
                return format!(
                    "{} words total, {} today",
                    total_words.as_u64().unwrap_or(0),
                    words_today.as_u64().unwrap_or(0)
                );
            }

            // Task metrics
            if let (Some(open), Some(completed)) = (obj.get("open"), obj.get("completed")) {
                return format!(
                    "{} open, {} done",
                    open.as_u64().unwrap_or(0),
                    completed.as_u64().unwrap_or(0)
                );
            }

            // Graph metrics
            if let (Some(broken), Some(orphaned)) = (obj.get("broken_links"), obj.get("orphaned")) {
                return format!(
                    "{} broken links, {} orphans",
                    broken.as_u64().unwrap_or(0),
                    orphaned.as_u64().unwrap_or(0)
                );
            }

            // Recent activity
            if let Some(modified_today) = obj.get("modified_today") {
                return format!("{} modified today", modified_today.as_u64().unwrap_or(0));
            }
        }

        // Fallback: Generic JSON summary
        format!(
            "{} fields",
            self.data.as_object().map_or(0, serde_json::Map::len)
        )
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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

    #[test]
    fn test_time_ago_formats_correctly() {
        let metadata = StateMetadata {
            query_name: "test".to_string(),
            commit_hash: "abc123".to_string(),
            timestamp: Utc::now().to_rfc3339(),
            command: "test".to_string(),
            deterministic: true,
        };

        let time_ago = metadata.time_ago();
        assert_eq!(time_ago, "just now");
    }

    #[test]
    fn test_writing_metrics_summary() {
        let result = StateResult {
            metadata: StateMetadata {
                query_name: "writing".to_string(),
                commit_hash: "abc123".to_string(),
                timestamp: Utc::now().to_rfc3339(),
                command: "test".to_string(),
                deterministic: false,
            },
            data: json!({
                "total_words": 5000,
                "words_today": 250,
                "notes_created": 1,
                "notes_modified": 3,
                "date": "2026-02-14"
            }),
        };

        assert_eq!(result.summary(), "5000 words total, 250 today");
    }

    #[test]
    fn test_task_metrics_summary() {
        let result = StateResult {
            metadata: StateMetadata {
                query_name: "tasks".to_string(),
                commit_hash: "abc123".to_string(),
                timestamp: Utc::now().to_rfc3339(),
                command: "test".to_string(),
                deterministic: true,
            },
            data: json!({
                "open": 59,
                "completed": 49,
                "total": 108
            }),
        };

        assert_eq!(result.summary(), "59 open, 49 done");
    }

    #[test]
    fn test_graph_metrics_summary() {
        let result = StateResult {
            metadata: StateMetadata {
                query_name: "graph".to_string(),
                commit_hash: "abc123".to_string(),
                timestamp: Utc::now().to_rfc3339(),
                command: "test".to_string(),
                deterministic: true,
            },
            data: json!({
                "total_notes": 2019,
                "total_links": 4910,
                "broken_links": 2223,
                "orphaned": 463
            }),
        };

        assert_eq!(result.summary(), "2223 broken links, 463 orphans");
    }

    #[test]
    fn test_generic_summary() {
        let result = StateResult {
            metadata: StateMetadata {
                query_name: "custom".to_string(),
                commit_hash: "abc123".to_string(),
                timestamp: Utc::now().to_rfc3339(),
                command: "test".to_string(),
                deterministic: true,
            },
            data: json!({
                "foo": 1,
                "bar": 2,
                "baz": 3
            }),
        };

        assert_eq!(result.summary(), "3 fields");
    }
}
