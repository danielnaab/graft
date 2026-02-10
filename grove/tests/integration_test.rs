//! Integration tests for Grove.
//!
//! These tests validate end-to-end behavior across the full stack
//! (core → engine → integration).

mod common;

use grove_core::{ConfigLoader, RepoRegistry};
use grove_engine::{GitoxideStatus, WorkspaceRegistry, YamlConfigLoader};
use std::fs;
use std::io::Write;
use std::process::Command;
use tempfile::{NamedTempFile, TempDir};

/// Test end-to-end workspace loading and status querying
#[test]
fn end_to_end_workspace_with_real_repos() {
    // Create temporary workspace with two git repos
    let temp_dir = TempDir::new().unwrap();
    let workspace_dir = temp_dir.path();

    // Create first repo (clean)
    let repo1_path = workspace_dir.join("repo1");
    fs::create_dir(&repo1_path).unwrap();
    init_git_repo(&repo1_path, "repo1 content", false);

    // Create second repo (dirty)
    let repo2_path = workspace_dir.join("repo2");
    fs::create_dir(&repo2_path).unwrap();
    init_git_repo(&repo2_path, "repo2 content", true);

    // Create workspace config
    let mut config_file = NamedTempFile::new().unwrap();
    writeln!(
        config_file,
        r"
name: test-workspace
repositories:
  - path: {}
    tags: [clean]
  - path: {}
    tags: [dirty]
",
        repo1_path.display(),
        repo2_path.display()
    )
    .unwrap();

    // Load config
    let loader = YamlConfigLoader::new();
    let config = loader
        .load_workspace(config_file.path().to_str().unwrap())
        .expect("Failed to load config");

    assert_eq!(config.name.as_str(), "test-workspace");
    assert_eq!(config.repositories.len(), 2);

    // Create registry and refresh status
    let git_status = GitoxideStatus::new();
    let mut registry = WorkspaceRegistry::new(config, git_status);
    registry.refresh_all().expect("Failed to refresh status");

    // Verify repos are listed
    let repos = registry.list_repos();
    assert_eq!(repos.len(), 2);

    // Check status of first repo (clean)
    let status1 = registry
        .get_status(&repos[0])
        .expect("Should have status for repo1");
    assert_eq!(status1.branch, Some("main".to_string()));
    assert!(!status1.is_dirty, "repo1 should be clean");

    // Check status of second repo (dirty)
    let status2 = registry
        .get_status(&repos[1])
        .expect("Should have status for repo2");
    assert_eq!(status2.branch, Some("main".to_string()));
    assert!(status2.is_dirty, "repo2 should be dirty");
}

/// Test handling of missing config file
#[test]
fn handles_missing_config_gracefully() {
    let loader = YamlConfigLoader::new();
    let result = loader.load_workspace("/nonexistent/workspace.yaml");

    assert!(result.is_err(), "Should fail on missing config");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Failed to read config file"),
        "Error message should be helpful: {err_msg}"
    );
}

/// Test handling of malformed YAML
#[test]
fn handles_malformed_yaml_gracefully() {
    let mut config_file = NamedTempFile::new().unwrap();
    writeln!(
        config_file,
        r"
name: test
this: is: not: valid: yaml
"
    )
    .unwrap();

    let loader = YamlConfigLoader::new();
    let result = loader.load_workspace(config_file.path().to_str().unwrap());

    assert!(result.is_err(), "Should fail on malformed YAML");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Failed to parse config file"),
        "Error message should mention parsing: {err_msg}"
    );
}

/// Test handling of non-git directory
#[test]
fn handles_non_git_repo_gracefully() {
    let temp_dir = TempDir::new().unwrap();
    let non_git_path = temp_dir.path().join("not-a-repo");
    fs::create_dir(&non_git_path).unwrap();

    // Create workspace config pointing to non-git directory
    let mut config_file = NamedTempFile::new().unwrap();
    writeln!(
        config_file,
        r"
name: test-workspace
repositories:
  - path: {}
",
        non_git_path.display()
    )
    .unwrap();

    // Load config and create registry
    let loader = YamlConfigLoader::new();
    let config = loader
        .load_workspace(config_file.path().to_str().unwrap())
        .unwrap();

    let git_status = GitoxideStatus::new();
    let mut registry = WorkspaceRegistry::new(config, git_status);

    // Should not fail - graceful degradation
    let result = registry.refresh_all();
    assert!(result.is_ok(), "Should handle non-git repo gracefully");

    // Check that error is recorded in status
    let repos = registry.list_repos();
    let status = registry.get_status(&repos[0]).unwrap();
    assert!(status.error.is_some(), "Should have error for non-git repo");
}

/// Test handling of empty repositories list
#[test]
fn handles_empty_repositories_list() {
    let mut config_file = NamedTempFile::new().unwrap();
    writeln!(
        config_file,
        r"
name: empty-workspace
repositories: []
"
    )
    .unwrap();

    let loader = YamlConfigLoader::new();
    let config = loader
        .load_workspace(config_file.path().to_str().unwrap())
        .unwrap();

    assert_eq!(config.repositories.len(), 0);

    let git_status = GitoxideStatus::new();
    let mut registry = WorkspaceRegistry::new(config, git_status);
    registry.refresh_all().unwrap();

    let repos = registry.list_repos();
    assert_eq!(repos.len(), 0, "Should handle empty repo list");
}

/// Test workspace with mix of valid and invalid repos
#[test]
fn handles_mixed_valid_invalid_repos() {
    let temp_dir = TempDir::new().unwrap();
    let workspace_dir = temp_dir.path();

    // Create one valid git repo
    let valid_repo = workspace_dir.join("valid");
    fs::create_dir(&valid_repo).unwrap();
    init_git_repo(&valid_repo, "valid content", false);

    // Create one non-git directory
    let invalid_repo = workspace_dir.join("invalid");
    fs::create_dir(&invalid_repo).unwrap();

    // Create workspace config with both
    let mut config_file = NamedTempFile::new().unwrap();
    writeln!(
        config_file,
        r"
name: mixed-workspace
repositories:
  - path: {}
  - path: {}
",
        valid_repo.display(),
        invalid_repo.display()
    )
    .unwrap();

    let loader = YamlConfigLoader::new();
    let config = loader
        .load_workspace(config_file.path().to_str().unwrap())
        .unwrap();

    let git_status = GitoxideStatus::new();
    let mut registry = WorkspaceRegistry::new(config, git_status);
    registry.refresh_all().unwrap();

    let repos = registry.list_repos();
    assert_eq!(repos.len(), 2);

    // Valid repo should have status
    let status1 = registry.get_status(&repos[0]).unwrap();
    assert!(status1.error.is_none(), "Valid repo should have no error");
    assert_eq!(status1.branch, Some("main".to_string()));

    // Invalid repo should have error
    let status2 = registry.get_status(&repos[1]).unwrap();
    assert!(
        status2.error.is_some(),
        "Invalid repo should have error: {status2:?}"
    );
}

// Helper functions

/// Initialize a git repository with a commit, optionally make it dirty
fn init_git_repo(path: &std::path::Path, content: &str, make_dirty: bool) {
    // Initialize
    Command::new("git")
        .args(["init"])
        .current_dir(path)
        .output()
        .unwrap();

    // Configure user
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

    // Create and commit file
    fs::write(path.join("README.md"), content).unwrap();
    Command::new("git")
        .args(["add", "README.md"])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(path)
        .output()
        .unwrap();

    // Optionally make dirty
    if make_dirty {
        fs::write(path.join("README.md"), format!("{content} modified")).unwrap();
    }
}
