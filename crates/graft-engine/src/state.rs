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

use crate::{GraftError, Result, StateQuery};
use graft_common::process::{run_to_completion_with_timeout, ProcessConfig};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

// Re-export shared types from graft-common
pub use graft_common::state::{StateMetadata, StateResult};

// Re-export shared cache functions from graft-common
pub use graft_common::state::{get_cache_path, read_cached_state};

/// Write state result to cache.
pub fn write_cached_state(
    workspace_name: &str,
    repo_name: &str,
    result: &StateResult,
) -> Result<()> {
    graft_common::state::write_cached_state(workspace_name, repo_name, result)
        .map_err(GraftError::Io)?;
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
    let count = graft_common::state::invalidate_cached_state(workspace_name, repo_name, query_name)
        .map_err(GraftError::Io)?;
    Ok(count)
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
    // Default timeout: 5 minutes, per spec.
    let timeout_secs = query.timeout.unwrap_or(300);

    let config = ProcessConfig {
        command: query.run.clone(),
        working_dir: repo_path.to_path_buf(),
        env: None,
        log_path: None,
        timeout: Some(Duration::from_secs(timeout_secs)),
    };

    // Execute command in repo directory via shell to support pipes, redirects, etc.
    // SECURITY: Commands from user's graft.yaml (trusted source).
    let output = run_to_completion_with_timeout(&config)
        .map_err(|e| GraftError::CommandExecution(format!("Failed to execute command: {e}")))?;

    // Check exit code
    if !output.success {
        let mut error_msg = format!(
            "State query '{}' failed with exit code {}",
            query.name, output.exit_code
        );
        if !output.stderr.is_empty() {
            error_msg.push_str("\nstderr: ");
            error_msg.push_str(&output.stderr);
        }
        if !output.stdout.is_empty() {
            error_msg.push_str("\nstdout: ");
            error_msg.push_str(&output.stdout);
        }

        return Err(GraftError::CommandExecution(error_msg));
    }

    // Parse JSON output
    let stdout = &output.stdout;
    let data: Value = serde_json::from_str(stdout).map_err(|e| {
        let preview = if stdout.len() > 500 {
            &stdout[..500]
        } else {
            stdout.as_str()
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
    use crate::StateCache;

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
