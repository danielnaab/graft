//! Query service for read-only dependency operations.
//!
//! Provides functions for querying dependency status from lock files
//! and changes from graft.yaml files.

use graft_core::domain::{Change, Command, CommitHash, GraftConfig, LockFile};
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

/// Get all changes from a dependency's config.
///
/// # Arguments
///
/// * `config` - Parsed graft.yaml configuration
///
/// # Returns
///
/// Vector of changes in declaration order
pub fn get_changes_for_dependency(config: &GraftConfig) -> Vec<Change> {
    config.changes.values().cloned().collect()
}

/// Filter changes by type.
///
/// # Arguments
///
/// * `changes` - Slice of changes to filter
/// * `change_type` - Type to filter by (e.g., "breaking", "feature", "fix")
///
/// # Returns
///
/// Vector of changes matching the type
pub fn filter_changes_by_type(changes: &[Change], change_type: &str) -> Vec<Change> {
    changes
        .iter()
        .filter(|c| c.change_type.as_deref() == Some(change_type))
        .cloned()
        .collect()
}

/// Filter to only breaking changes.
///
/// # Arguments
///
/// * `changes` - Slice of changes to filter
///
/// # Returns
///
/// Vector of breaking changes
pub fn filter_breaking_changes(changes: &[Change]) -> Vec<Change> {
    changes
        .iter()
        .filter(|c| c.is_breaking())
        .cloned()
        .collect()
}

/// Get a specific change by ref.
///
/// # Arguments
///
/// * `config` - Parsed graft.yaml configuration
/// * `ref_name` - Ref to look up
///
/// # Returns
///
/// `Some(Change)` if found, `None` otherwise
pub fn get_change_by_ref(config: &GraftConfig, ref_name: &str) -> Option<Change> {
    config.changes.get(ref_name).cloned()
}

/// Detailed information about a change, including resolved command details.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeDetails {
    /// The change object
    pub change: Change,
    /// Migration command details (if any)
    pub migration_command: Option<Command>,
    /// Verification command details (if any)
    pub verify_command: Option<Command>,
}

/// Get detailed information about a change.
///
/// # Arguments
///
/// * `config` - Parsed graft.yaml configuration
/// * `ref_name` - Ref of change to get details for
///
/// # Returns
///
/// `Some(ChangeDetails)` if change found, `None` otherwise
///
/// # Note
///
/// Command existence is validated by `GraftConfig` parsing,
/// so we can safely assume commands exist if referenced.
pub fn get_change_details(config: &GraftConfig, ref_name: &str) -> Option<ChangeDetails> {
    let change = get_change_by_ref(config, ref_name)?;

    // Get command details (validated by GraftConfig, so safe to use .get())
    let migration_command = change
        .migration
        .as_ref()
        .and_then(|name| config.commands.get(name).cloned());

    let verify_command = change
        .verify
        .as_ref()
        .and_then(|name| config.commands.get(name).cloned());

    Some(ChangeDetails {
        change,
        migration_command,
        verify_command,
    })
}

#[cfg(test)]
mod change_tests {
    use super::*;
    use graft_core::domain::Metadata;
    use std::collections::HashMap;

    fn build_test_config() -> GraftConfig {
        let mut changes = HashMap::new();
        changes.insert(
            "v1.0.0".to_string(),
            Change::new("v1.0.0")
                .unwrap()
                .with_type("feature")
                .with_description("Initial release"),
        );
        changes.insert(
            "v2.0.0".to_string(),
            Change::new("v2.0.0")
                .unwrap()
                .with_type("breaking")
                .with_description("Breaking API change")
                .with_migration("migrate-v2")
                .with_verify("verify-v2"),
        );
        changes.insert(
            "v2.1.0".to_string(),
            Change::new("v2.1.0")
                .unwrap()
                .with_type("feature")
                .with_description("New feature"),
        );

        let mut commands = HashMap::new();
        commands.insert(
            "migrate-v2".to_string(),
            Command::new("migrate-v2", "migrate.sh")
                .unwrap()
                .with_description("Migrate to v2"),
        );
        commands.insert(
            "verify-v2".to_string(),
            Command::new("verify-v2", "test.sh")
                .unwrap()
                .with_description("Verify v2 migration"),
        );

        GraftConfig {
            api_version: "graft/v0".to_string(),
            metadata: Some(Metadata::default()),
            dependencies: HashMap::new(),
            changes,
            commands,
        }
    }

    #[test]
    fn test_get_changes_for_dependency() {
        let config = build_test_config();
        let changes = get_changes_for_dependency(&config);

        assert_eq!(changes.len(), 3);
    }

    #[test]
    fn test_filter_changes_by_type() {
        let config = build_test_config();
        let all_changes = get_changes_for_dependency(&config);

        let breaking = filter_changes_by_type(&all_changes, "breaking");
        assert_eq!(breaking.len(), 1);
        assert_eq!(breaking[0].ref_name, "v2.0.0");

        let features = filter_changes_by_type(&all_changes, "feature");
        assert_eq!(features.len(), 2);
    }

    #[test]
    fn test_filter_breaking_changes() {
        let config = build_test_config();
        let all_changes = get_changes_for_dependency(&config);

        let breaking = filter_breaking_changes(&all_changes);
        assert_eq!(breaking.len(), 1);
        assert_eq!(breaking[0].ref_name, "v2.0.0");
        assert!(breaking[0].is_breaking());
    }

    #[test]
    fn test_get_change_by_ref() {
        let config = build_test_config();

        let change = get_change_by_ref(&config, "v2.0.0");
        assert!(change.is_some());
        assert_eq!(change.unwrap().ref_name, "v2.0.0");

        let missing = get_change_by_ref(&config, "v99.0.0");
        assert!(missing.is_none());
    }

    #[test]
    fn test_get_change_details() {
        let config = build_test_config();

        // Change with commands
        let details = get_change_details(&config, "v2.0.0");
        assert!(details.is_some());

        let details = details.unwrap();
        assert_eq!(details.change.ref_name, "v2.0.0");
        assert!(details.migration_command.is_some());
        assert_eq!(details.migration_command.unwrap().name, "migrate-v2");
        assert!(details.verify_command.is_some());
        assert_eq!(details.verify_command.unwrap().name, "verify-v2");

        // Change without commands
        let details = get_change_details(&config, "v1.0.0");
        assert!(details.is_some());

        let details = details.unwrap();
        assert_eq!(details.change.ref_name, "v1.0.0");
        assert!(details.migration_command.is_none());
        assert!(details.verify_command.is_none());

        // Missing change
        let details = get_change_details(&config, "v99.0.0");
        assert!(details.is_none());
    }
}
