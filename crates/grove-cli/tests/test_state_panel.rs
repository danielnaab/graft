//! Integration tests for state panel functionality.
//!
//! These tests verify state query discovery, cache reading, and summary formatting
//! using real file system interactions.

use grove::state::{
    compute_workspace_hash, discover_state_queries, format_state_summary,
    read_all_cached_for_query, read_latest_cached, StateMetadata, StateResult,
};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

// ===== State Discovery Tests =====

#[test]
fn discover_state_queries_from_graft_yaml() {
    let temp_dir = tempdir().unwrap();
    let graft_yaml = temp_dir.path().join("graft.yaml");

    fs::write(
        &graft_yaml,
        r#"
apiVersion: graft/v1beta1
name: test-repo

state:
  coverage:
    run: "pytest --cov"
    cache:
      deterministic: true
    timeout: 60
    description: "Code coverage metrics"

  tasks:
    run: "task-tracker status"
    cache:
      deterministic: false
"#,
    )
    .unwrap();

    let queries =
        discover_state_queries(graft_yaml.as_path()).expect("Failed to discover state queries");

    assert_eq!(queries.len(), 2);

    // Verify coverage query
    let coverage = queries
        .iter()
        .find(|q| q.name == "coverage")
        .expect("Coverage query not found");
    assert_eq!(coverage.deterministic, true);
    assert_eq!(coverage.timeout, Some(60));
    assert_eq!(
        coverage.description,
        Some("Code coverage metrics".to_string())
    );

    // Verify tasks query
    let tasks = queries
        .iter()
        .find(|q| q.name == "tasks")
        .expect("Tasks query not found");
    assert_eq!(tasks.deterministic, false);
    assert_eq!(tasks.timeout, None);
}

#[test]
fn discover_handles_missing_state_section() {
    let temp_dir = tempdir().unwrap();
    let graft_yaml = temp_dir.path().join("graft.yaml");

    fs::write(
        &graft_yaml,
        r#"
apiVersion: graft/v1beta1
name: test-repo
commands:
  test:
    run: "echo test"
"#,
    )
    .unwrap();

    let queries =
        discover_state_queries(graft_yaml.as_path()).expect("Should succeed with empty queries");

    assert_eq!(queries.len(), 0);
}

#[test]
fn discover_handles_malformed_yaml() {
    let temp_dir = tempdir().unwrap();
    let graft_yaml = temp_dir.path().join("graft.yaml");

    fs::write(
        &graft_yaml,
        r#"
invalid: [ yaml syntax
"#,
    )
    .unwrap();

    let result = discover_state_queries(graft_yaml.as_path());

    assert!(result.is_err(), "Should fail on malformed YAML");
}

#[test]
fn discover_handles_missing_run_field() {
    let temp_dir = tempdir().unwrap();
    let graft_yaml = temp_dir.path().join("graft.yaml");

    fs::write(
        &graft_yaml,
        r#"
apiVersion: graft/v1beta1
name: test-repo

state:
  incomplete:
    cache:
      deterministic: true
"#,
    )
    .unwrap();

    let queries =
        discover_state_queries(graft_yaml.as_path()).expect("Should skip invalid queries");

    // Invalid query should be skipped (warning logged)
    assert_eq!(queries.len(), 0);
}

// ===== Cache Reading Tests =====

/// Helper function to get test cache directory path
fn get_test_cache_dir(workspace_name: &str, repo_name: &str, query_name: &str) -> PathBuf {
    let workspace_hash = compute_workspace_hash(workspace_name);
    PathBuf::from(std::env::var("HOME").unwrap())
        .join(".cache/graft")
        .join(workspace_hash)
        .join(repo_name)
        .join("state")
        .join(query_name)
}

/// Helper function to cleanup test cache
fn cleanup_test_cache(workspace_name: &str) {
    let workspace_hash = compute_workspace_hash(workspace_name);
    let cache_root = PathBuf::from(std::env::var("HOME").unwrap())
        .join(".cache/graft")
        .join(workspace_hash);

    fs::remove_dir_all(cache_root).ok();
}

#[test]
fn read_cached_state_from_file() {
    let workspace_name = "test-read-cached-state";
    let repo_name = "my-repo";
    let query_name = "coverage";
    let commit_hash = "abc123def456";

    // Create cache directory
    let cache_dir = get_test_cache_dir(workspace_name, repo_name, query_name);
    fs::create_dir_all(&cache_dir).unwrap();

    // Write cache file
    let cache_file = cache_dir.join(format!("{}.json", commit_hash));
    let result = StateResult {
        metadata: StateMetadata {
            query_name: "coverage".to_string(),
            commit_hash: "abc123def456".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            command: "pytest --cov".to_string(),
            deterministic: true,
        },
        data: json!({"lines": 85, "branches": 72}),
    };

    fs::write(&cache_file, serde_json::to_string(&result).unwrap()).unwrap();

    // Read cache using graft-common (workspace_name, not hash)
    let loaded =
        graft_common::state::read_cached_state(workspace_name, repo_name, query_name, commit_hash)
            .expect("Failed to read cache");

    assert_eq!(loaded.metadata.query_name, "coverage");
    assert_eq!(loaded.data["lines"], 85);
    assert_eq!(loaded.data["branches"], 72);

    // Cleanup
    cleanup_test_cache(workspace_name);
}

#[test]
fn read_latest_cached_returns_newest() {
    use std::thread;
    use std::time::Duration;

    let workspace_name = "test-read-latest";
    let repo_name = "my-repo";
    let query_name = "tasks";

    let cache_dir = get_test_cache_dir(workspace_name, repo_name, query_name);
    fs::create_dir_all(&cache_dir).unwrap();

    // Write older result
    let old_result = StateResult {
        metadata: StateMetadata {
            query_name: "tasks".to_string(),
            commit_hash: "old123".to_string(),
            timestamp: (chrono::Utc::now() - chrono::Duration::hours(2)).to_rfc3339(),
            command: "task-list".to_string(),
            deterministic: true,
        },
        data: json!({"open": 50}),
    };

    fs::write(
        cache_dir.join("old123.json"),
        serde_json::to_string(&old_result).unwrap(),
    )
    .unwrap();

    thread::sleep(Duration::from_millis(10));

    // Write newer result
    let new_result = StateResult {
        metadata: StateMetadata {
            query_name: "tasks".to_string(),
            commit_hash: "new456".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            command: "task-list".to_string(),
            deterministic: true,
        },
        data: json!({"open": 59}),
    };

    fs::write(
        cache_dir.join("new456.json"),
        serde_json::to_string(&new_result).unwrap(),
    )
    .unwrap();

    // Read latest using graft-common
    let latest = read_latest_cached(workspace_name, repo_name, query_name)
        .expect("Should find latest cached result");

    assert_eq!(latest.metadata.commit_hash, "new456");
    assert_eq!(latest.data["open"], 59);

    // Cleanup
    cleanup_test_cache(workspace_name);
}

#[test]
fn read_all_cached_returns_sorted_by_time() {
    let workspace_name = "test-read-all";
    let repo_name = "my-repo";
    let query_name = "writing";

    let cache_dir = get_test_cache_dir(workspace_name, repo_name, query_name);
    fs::create_dir_all(&cache_dir).unwrap();

    // Create multiple cache files with different timestamps
    let commits = vec!["aaa", "bbb", "ccc"];
    for (i, commit) in commits.iter().enumerate() {
        let timestamp = (chrono::Utc::now() - chrono::Duration::hours((3 - i) as i64)).to_rfc3339();

        let result = StateResult {
            metadata: StateMetadata {
                query_name: "writing".to_string(),
                commit_hash: commit.to_string(),
                timestamp,
                command: "word-count".to_string(),
                deterministic: false,
            },
            data: json!({"words": 1000 * (i + 1)}),
        };

        fs::write(
            cache_dir.join(format!("{}.json", commit)),
            serde_json::to_string(&result).unwrap(),
        )
        .unwrap();
    }

    // Read all using graft-common
    let all_results = read_all_cached_for_query(workspace_name, repo_name, query_name);

    assert_eq!(all_results.len(), 3);

    // Verify sorted newest first
    assert_eq!(all_results[0].metadata.commit_hash, "ccc");
    assert_eq!(all_results[1].metadata.commit_hash, "bbb");
    assert_eq!(all_results[2].metadata.commit_hash, "aaa");

    // Cleanup
    cleanup_test_cache(workspace_name);
}

#[test]
fn compute_workspace_hash_consistent() {
    let hash1 = compute_workspace_hash("my-workspace");
    let hash2 = compute_workspace_hash("my-workspace");

    assert_eq!(hash1, hash2, "Same workspace name should hash the same");
    assert_eq!(hash1.len(), 16, "Hash should be 16 chars");

    let hash3 = compute_workspace_hash("other-workspace");
    assert_ne!(
        hash1, hash3,
        "Different workspaces should have different hashes"
    );
}

// ===== Summary Formatting Tests =====

#[test]
fn summary_formats_writing_metrics() {
    let result = StateResult {
        metadata: StateMetadata {
            query_name: "writing".to_string(),
            commit_hash: "abc123".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            command: "word-count".to_string(),
            deterministic: false,
        },
        data: json!({
            "total_words": 5000,
            "words_today": 250,
        }),
    };

    assert_eq!(format_state_summary(&result), "5000 words total, 250 today");
}

#[test]
fn summary_formats_task_metrics() {
    let result = StateResult {
        metadata: StateMetadata {
            query_name: "tasks".to_string(),
            commit_hash: "abc123".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            command: "task-list".to_string(),
            deterministic: true,
        },
        data: json!({"open": 59, "completed": 49}),
    };

    assert_eq!(format_state_summary(&result), "59 open, 49 done");
}

#[test]
fn summary_formats_graph_metrics() {
    let result = StateResult {
        metadata: StateMetadata {
            query_name: "graph".to_string(),
            commit_hash: "abc123".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            command: "graph-analyze".to_string(),
            deterministic: true,
        },
        data: json!({
            "broken_links": 2223,
            "orphaned": 463,
        }),
    };

    assert_eq!(
        format_state_summary(&result),
        "2223 broken links, 463 orphans"
    );
}

#[test]
fn summary_falls_back_for_unknown_format() {
    let result = StateResult {
        metadata: StateMetadata {
            query_name: "custom".to_string(),
            commit_hash: "abc123".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            command: "custom-query".to_string(),
            deterministic: true,
        },
        data: json!({"foo": 42, "bar": "baz"}),
    };

    // Should not panic, should return something reasonable
    let summary = format_state_summary(&result);
    assert!(!summary.is_empty(), "Summary should not be empty");
}
