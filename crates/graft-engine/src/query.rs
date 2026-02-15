//! Query service for read-only dependency operations.
//!
//! Provides functions for querying dependency status from lock files.

use graft_core::domain::{CommitHash, LockFile};
use indexmap::IndexMap;

/// Status information for a single dependency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyStatus {
    /// Dependency name
    pub name: String,
    /// Currently consumed git ref
    pub current_ref: String,
    /// Commit hash at consumed ref
    pub commit: CommitHash,
    /// Timestamp when consumed (ISO 8601 string)
    pub consumed_at: String,
}

/// Get status for all dependencies from a lock file.
///
/// # Arguments
///
/// * `lock_file` - Parsed lock file
///
/// # Returns
///
/// Map of dependency name to status, in alphabetical order by name
pub fn get_all_status(lock_file: &LockFile) -> IndexMap<String, DependencyStatus> {
    let mut statuses: Vec<_> = lock_file
        .dependencies
        .iter()
        .map(|(name, entry)| {
            let status = DependencyStatus {
                name: name.clone(),
                current_ref: entry.git_ref.as_str().to_string(),
                commit: entry.commit.clone(),
                consumed_at: entry.consumed_at.clone(),
            };
            (name.clone(), status)
        })
        .collect();

    // Sort alphabetically by name
    statuses.sort_by(|a, b| a.0.cmp(&b.0));

    statuses.into_iter().collect()
}

/// Get status for a single dependency from a lock file.
///
/// # Arguments
///
/// * `lock_file` - Parsed lock file
/// * `dep_name` - Name of dependency to query
///
/// # Returns
///
/// `Some(DependencyStatus)` if found, `None` otherwise
pub fn get_dependency_status(lock_file: &LockFile, dep_name: &str) -> Option<DependencyStatus> {
    lock_file
        .dependencies
        .get(dep_name)
        .map(|entry| DependencyStatus {
            name: dep_name.to_string(),
            current_ref: entry.git_ref.as_str().to_string(),
            commit: entry.commit.clone(),
            consumed_at: entry.consumed_at.clone(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use graft_core::domain::{GitRef, GitUrl, LockEntry};
    use std::collections::HashMap;

    #[test]
    fn test_get_all_status_empty() {
        let lock = LockFile {
            api_version: "graft/v0".to_string(),
            dependencies: HashMap::new(),
        };

        let statuses = get_all_status(&lock);
        assert!(statuses.is_empty());
    }

    #[test]
    fn test_get_all_status_multiple() {
        let mut deps = HashMap::new();

        deps.insert(
            "meta-kb".to_string(),
            LockEntry {
                source: GitUrl::new("git@github.com:org/meta.git").unwrap(),
                git_ref: GitRef::new("v1.0.0").unwrap(),
                commit: CommitHash::new("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap(),
                consumed_at: "2026-01-01T10:30:00Z".to_string(),
            },
        );

        deps.insert(
            "coding-standards".to_string(),
            LockEntry {
                source: GitUrl::new("git@github.com:org/standards.git").unwrap(),
                git_ref: GitRef::new("v2.0.0").unwrap(),
                commit: CommitHash::new("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb").unwrap(),
                consumed_at: "2026-01-05T14:20:00Z".to_string(),
            },
        );

        let lock = LockFile {
            api_version: "graft/v0".to_string(),
            dependencies: deps,
        };

        let statuses = get_all_status(&lock);
        assert_eq!(statuses.len(), 2);

        // Check alphabetical ordering
        let names: Vec<_> = statuses.keys().map(String::as_str).collect();
        assert_eq!(names, vec!["coding-standards", "meta-kb"]);

        // Check status details
        let meta_status = &statuses["meta-kb"];
        assert_eq!(meta_status.name, "meta-kb");
        assert_eq!(meta_status.current_ref, "v1.0.0");
        assert_eq!(
            meta_status.commit.as_str(),
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        );
        assert_eq!(meta_status.consumed_at, "2026-01-01T10:30:00Z");

        let standards_status = &statuses["coding-standards"];
        assert_eq!(standards_status.name, "coding-standards");
        assert_eq!(standards_status.current_ref, "v2.0.0");
        assert_eq!(standards_status.consumed_at, "2026-01-05T14:20:00Z");
    }

    #[test]
    fn test_get_dependency_status_found() {
        let mut deps = HashMap::new();

        deps.insert(
            "meta-kb".to_string(),
            LockEntry {
                source: GitUrl::new("git@github.com:org/meta.git").unwrap(),
                git_ref: GitRef::new("v1.0.0").unwrap(),
                commit: CommitHash::new("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap(),
                consumed_at: "2026-01-01T10:30:00Z".to_string(),
            },
        );

        let lock = LockFile {
            api_version: "graft/v0".to_string(),
            dependencies: deps,
        };

        let status = get_dependency_status(&lock, "meta-kb");
        assert!(status.is_some());

        let status = status.unwrap();
        assert_eq!(status.name, "meta-kb");
        assert_eq!(status.current_ref, "v1.0.0");
        assert_eq!(
            status.commit.as_str(),
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        );
        assert_eq!(status.consumed_at, "2026-01-01T10:30:00Z");
    }

    #[test]
    fn test_get_dependency_status_not_found() {
        let lock = LockFile {
            api_version: "graft/v0".to_string(),
            dependencies: HashMap::new(),
        };

        let status = get_dependency_status(&lock, "nonexistent");
        assert!(status.is_none());
    }
}
