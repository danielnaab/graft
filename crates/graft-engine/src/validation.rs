//! Validation service for graft configuration and state.
//!
//! Provides functions for validating graft.yaml, graft.lock,
//! and .graft/ directory integrity.

use graft_core::domain::{CommitHash, GraftConfig, LockFile};
use std::path::Path;

/// A validation error with severity.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationError {
    pub message: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Severity {
    Error,
    Warning,
}

impl ValidationError {
    /// Create a new error.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            severity: Severity::Error,
        }
    }

    /// Create a new warning.
    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            severity: Severity::Warning,
        }
    }
}

/// Validate graft.yaml schema and business rules.
///
/// Most schema validation (API version, command references) happens
/// during parsing, so this function validates business rules that
/// can't be enforced at the domain level.
pub fn validate_config_schema(config: &GraftConfig) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // Check dependencies exist (business rule: require at least one dependency)
    if config.dependencies.is_empty() {
        errors.push(ValidationError::error(
            "No dependencies defined (at least one dependency required)",
        ));
    }

    errors
}

/// Result of integrity validation for a single dependency.
#[derive(Debug, Clone, PartialEq)]
pub struct IntegrityResult {
    pub name: String,
    pub valid: bool,
    pub expected_commit: CommitHash,
    pub actual_commit: Option<CommitHash>,
    pub message: String,
}

/// Validate that .graft/ directory matches the lock file.
///
/// Compares actual commit hashes in .graft/ against expected
/// commits in graft.lock.
///
/// # Arguments
/// * `deps_directory` - Path to dependencies directory (e.g., ".graft")
/// * `lock_file` - Parsed lock file
///
/// # Returns
/// List of integrity results, one per dependency in lock file
pub fn validate_integrity(
    deps_directory: impl AsRef<Path>,
    lock_file: &LockFile,
) -> Vec<IntegrityResult> {
    let deps_dir = deps_directory.as_ref();
    let mut results = Vec::new();

    // Sort dependencies by name for consistent output
    let mut entries: Vec<_> = lock_file.dependencies.iter().collect();
    entries.sort_by_key(|(name, _)| *name);

    for (name, entry) in entries {
        let local_path = deps_dir.join(name.as_str());

        // Check if dependency exists
        if !local_path.exists() {
            results.push(IntegrityResult {
                name: name.clone(),
                valid: false,
                expected_commit: entry.commit.clone(),
                actual_commit: None,
                message: "Dependency not found in .graft/".to_string(),
            });
            continue;
        }

        // Check if it's a git repository (has .git subdirectory or file)
        let git_path = local_path.join(".git");
        if !git_path.exists() {
            results.push(IntegrityResult {
                name: name.clone(),
                valid: false,
                expected_commit: entry.commit.clone(),
                actual_commit: None,
                message: "Path exists but is not a git repository".to_string(),
            });
            continue;
        }

        // Get current commit from git repository
        match get_current_commit(&local_path) {
            Ok(actual_commit) => {
                // Compare commits
                if actual_commit == entry.commit {
                    results.push(IntegrityResult {
                        name: name.clone(),
                        valid: true,
                        expected_commit: entry.commit.clone(),
                        actual_commit: Some(actual_commit),
                        message: "Commit matches".to_string(),
                    });
                } else {
                    let expected_short = &entry.commit.as_str()[..7];
                    let actual_short = &actual_commit.as_str()[..7];
                    let message =
                        format!("Commit mismatch: expected {expected_short}, got {actual_short}");
                    results.push(IntegrityResult {
                        name: name.clone(),
                        valid: false,
                        expected_commit: entry.commit.clone(),
                        actual_commit: Some(actual_commit),
                        message,
                    });
                }
            }
            Err(e) => {
                results.push(IntegrityResult {
                    name: name.clone(),
                    valid: false,
                    expected_commit: entry.commit.clone(),
                    actual_commit: None,
                    message: format!("Failed to get commit: {e}"),
                });
            }
        }
    }

    results
}

/// Get the current commit hash from a git repository.
///
/// Runs `git rev-parse HEAD` in the repository directory.
fn get_current_commit(repo_path: &Path) -> Result<CommitHash, String> {
    use std::process::Command;

    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("Failed to run git: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git rev-parse failed: {stderr}"));
    }

    let commit_str = String::from_utf8_lossy(&output.stdout);
    let commit_str = commit_str.trim();

    CommitHash::new(commit_str).map_err(|e| format!("Invalid commit hash from git: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use graft_core::domain::{DependencySpec, GitRef, GitUrl, LockEntry, Metadata};
    use std::collections::HashMap;

    #[test]
    fn test_validate_config_no_dependencies() {
        let config = GraftConfig {
            api_version: "graft/v3".to_string(),
            metadata: Some(Metadata::default()),
            dependencies: HashMap::new(),
            changes: HashMap::new(),
            commands: HashMap::new(),
        };

        let errors = validate_config_schema(&config);
        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0].message,
            "No dependencies defined (at least one dependency required)"
        );
        assert_eq!(errors[0].severity, Severity::Error);
    }

    #[test]
    fn test_validate_config_with_dependencies() {
        let dep_spec = DependencySpec::new(
            "test-dep",
            GitUrl::new("https://github.com/test/repo").unwrap(),
            GitRef::new("main").unwrap(),
        )
        .unwrap();

        let mut deps = HashMap::new();
        deps.insert("test-dep".to_string(), dep_spec);

        let config = GraftConfig {
            api_version: "graft/v3".to_string(),
            metadata: Some(Metadata::default()),
            dependencies: deps,
            changes: HashMap::new(),
            commands: HashMap::new(),
        };

        let errors = validate_config_schema(&config);
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn test_validate_integrity_missing_dependency() {
        let mut deps = HashMap::new();
        deps.insert(
            "missing-dep".to_string(),
            LockEntry::new(
                GitUrl::new("https://github.com/test/repo").unwrap(),
                GitRef::new("main").unwrap(),
                CommitHash::new("0123456789abcdef0123456789abcdef01234567").unwrap(),
                "2026-01-01T00:00:00Z",
            ),
        );

        let lock_file = LockFile {
            api_version: "graft/v3".to_string(),
            dependencies: deps,
        };

        let temp_dir = std::env::temp_dir().join("graft-test-missing-dep");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let results = validate_integrity(&temp_dir, &lock_file);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "missing-dep");
        assert!(!results[0].valid);
        assert_eq!(results[0].message, "Dependency not found in .graft/");

        std::fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_validate_integrity_not_a_git_repo() {
        let mut deps = HashMap::new();
        deps.insert(
            "not-a-repo".to_string(),
            LockEntry::new(
                GitUrl::new("https://github.com/test/repo").unwrap(),
                GitRef::new("main").unwrap(),
                CommitHash::new("0123456789abcdef0123456789abcdef01234567").unwrap(),
                "2026-01-01T00:00:00Z",
            ),
        );

        let lock_file = LockFile {
            api_version: "graft/v3".to_string(),
            dependencies: deps,
        };

        let temp_dir = std::env::temp_dir().join("graft-test-not-a-repo");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Create directory but not a git repo
        let dep_dir = temp_dir.join("not-a-repo");
        std::fs::create_dir_all(&dep_dir).unwrap();

        let results = validate_integrity(&temp_dir, &lock_file);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "not-a-repo");
        assert!(!results[0].valid);
        assert_eq!(
            results[0].message,
            "Path exists but is not a git repository"
        );

        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
}
