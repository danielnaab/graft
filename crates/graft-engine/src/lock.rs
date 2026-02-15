//! Lock file parsing and writing.
//!
//! Implements reading, writing, and validation of graft.lock files.

use graft_core::domain::LockFile;
use graft_core::error::{GraftError, Result};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Parse graft.lock from a file path.
///
/// # Arguments
///
/// * `path` - Path to graft.lock file
///
/// # Returns
///
/// * `Ok(LockFile)` - Successfully parsed and validated lock file
/// * `Err(GraftError)` - If file doesn't exist, can't be parsed, or validation fails
///
/// # Examples
///
/// ```no_run
/// use graft_engine::lock::parse_lock_file;
///
/// let lock = parse_lock_file("graft.lock").unwrap();
/// println!("Found {} dependencies", lock.dependencies.len());
/// ```
pub fn parse_lock_file(path: impl AsRef<Path>) -> Result<LockFile> {
    let path = path.as_ref();

    if !path.exists() {
        return Err(GraftError::LockFileNotFound {
            path: path.display().to_string(),
        });
    }

    let contents = std::fs::read_to_string(path).map_err(|e| GraftError::LockFileParse {
        path: path.display().to_string(),
        reason: e.to_string(),
    })?;

    parse_lock_file_str(&contents, path.display().to_string())
}

/// Parse graft.lock from a string.
///
/// # Arguments
///
/// * `yaml_str` - YAML content as string
/// * `path_for_errors` - Path string to include in error messages
///
/// # Returns
///
/// * `Ok(LockFile)` - Successfully parsed and validated lock file
/// * `Err(GraftError)` - If YAML can't be parsed or validation fails
///
/// # Examples
///
/// ```
/// use graft_engine::lock::parse_lock_file_str;
///
/// let yaml = r#"
/// apiVersion: graft/v0
/// dependencies:
///   meta-kb:
///     source: "https://github.com/org/meta-kb.git"
///     ref: "v2.0.0"
///     commit: "abc123def456789012345678901234567890abcd"
///     consumed_at: "2026-01-31T10:30:00Z"
/// "#;
///
/// let lock = parse_lock_file_str(yaml, "graft.lock").unwrap();
/// assert_eq!(lock.dependencies.len(), 1);
/// ```
pub fn parse_lock_file_str(yaml_str: &str, path_for_errors: impl Into<String>) -> Result<LockFile> {
    let path_for_errors = path_for_errors.into();

    // Parse YAML
    let lock: LockFile = serde_yml::from_str(yaml_str).map_err(|e| GraftError::LockFileParse {
        path: path_for_errors.clone(),
        reason: e.to_string(),
    })?;

    // Validate
    lock.validate().map_err(|e| GraftError::LockFileParse {
        path: path_for_errors,
        reason: e.to_string(),
    })?;

    Ok(lock)
}

/// Write lock file with round-trip fidelity.
///
/// Dependencies are written in alphabetical order per spec.
///
/// # Arguments
///
/// * `path` - Path to write graft.lock file
/// * `lock` - `LockFile` to write
///
/// # Returns
///
/// * `Ok(())` - Successfully written
/// * `Err(GraftError)` - If unable to write file
///
/// # Examples
///
/// ```no_run
/// use graft_core::domain::LockFile;
/// use graft_engine::lock::write_lock_file;
///
/// let lock = LockFile::new();
/// write_lock_file("graft.lock", &lock).unwrap();
/// ```
pub fn write_lock_file(path: impl AsRef<Path>, lock: &LockFile) -> Result<()> {
    let path = path.as_ref();

    // Validate before writing
    lock.validate()?;

    // Create parent directory if needed
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Convert to ordered structure for alphabetical output
    let ordered = to_ordered_lock(lock);

    // Serialize to YAML
    let yaml = serde_yml::to_string(&ordered).map_err(|e| GraftError::Yaml(e.to_string()))?;

    // Write to file
    std::fs::write(path, yaml)?;

    Ok(())
}

/// Convert `LockFile` to alphabetically ordered structure for serialization.
///
/// The spec requires dependencies to be written in alphabetical order.
/// We use `IndexMap` to preserve the order during serialization.
#[derive(Debug, Serialize, Deserialize)]
struct OrderedLockFile {
    #[serde(rename = "apiVersion")]
    api_version: String,
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    dependencies: IndexMap<String, graft_core::domain::LockEntry>,
}

fn to_ordered_lock(lock: &LockFile) -> OrderedLockFile {
    let mut dependencies = IndexMap::new();

    // Sort keys alphabetically
    let mut names: Vec<_> = lock.dependencies.keys().collect();
    names.sort();

    for name in names {
        if let Some(entry) = lock.dependencies.get(name) {
            dependencies.insert(name.clone(), entry.clone());
        }
    }

    OrderedLockFile {
        api_version: lock.api_version.clone(),
        dependencies,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use graft_core::domain::{CommitHash, GitRef, GitUrl, LockEntry};

    #[test]
    fn parse_lock_file_str_basic() {
        let yaml = r#"
apiVersion: graft/v0
dependencies:
  meta-kb:
    source: "https://github.com/org/meta-kb.git"
    ref: "v2.0.0"
    commit: "abc123def456789012345678901234567890abcd"
    consumed_at: "2026-01-31T10:30:00Z"
"#;

        let lock = parse_lock_file_str(yaml, "test.lock").unwrap();
        assert_eq!(lock.api_version, "graft/v0");
        assert_eq!(lock.dependencies.len(), 1);

        let entry = lock.get("meta-kb").unwrap();
        assert_eq!(entry.source.as_str(), "https://github.com/org/meta-kb.git");
        assert_eq!(entry.git_ref.as_str(), "v2.0.0");
        assert_eq!(
            entry.commit.as_str(),
            "abc123def456789012345678901234567890abcd"
        );
        assert_eq!(entry.consumed_at, "2026-01-31T10:30:00Z");
    }

    #[test]
    fn parse_lock_file_str_multiple_deps() {
        let yaml = r#"
apiVersion: graft/v0
dependencies:
  coding-standards:
    source: "https://github.com/org/standards.git"
    ref: "v1.5.0"
    commit: "def456abc123789012345678901234567890abcd"
    consumed_at: "2026-01-31T09:15:00Z"
  meta-kb:
    source: "https://github.com/org/meta-kb.git"
    ref: "v2.0.0"
    commit: "abc123def456789012345678901234567890abcd"
    consumed_at: "2026-01-31T10:30:00Z"
"#;

        let lock = parse_lock_file_str(yaml, "test.lock").unwrap();
        assert_eq!(lock.dependencies.len(), 2);
        assert!(lock.get("meta-kb").is_some());
        assert!(lock.get("coding-standards").is_some());
    }

    #[test]
    fn parse_lock_file_str_rejects_invalid_api_version() {
        let yaml = r#"
apiVersion: v1
dependencies: {}
"#;

        assert!(parse_lock_file_str(yaml, "test.lock").is_err());
    }

    #[test]
    fn parse_lock_file_str_rejects_invalid_commit_hash() {
        let yaml = r#"
apiVersion: graft/v0
dependencies:
  meta-kb:
    source: "https://github.com/org/meta-kb.git"
    ref: "v2.0.0"
    commit: "not-a-valid-hash"
    consumed_at: "2026-01-31T10:30:00Z"
"#;

        assert!(parse_lock_file_str(yaml, "test.lock").is_err());
    }

    #[test]
    fn parse_lock_file_str_rejects_missing_fields() {
        let yaml = r#"
apiVersion: graft/v0
dependencies:
  meta-kb:
    source: "https://github.com/org/meta-kb.git"
    ref: "v2.0.0"
"#;

        assert!(parse_lock_file_str(yaml, "test.lock").is_err());
    }

    #[test]
    fn write_lock_file_alphabetical_order() {
        let mut lock = LockFile::new();

        // Add in non-alphabetical order
        let entry_z = LockEntry::new(
            GitUrl::new("https://github.com/org/z.git").unwrap(),
            GitRef::new("v1.0.0").unwrap(),
            CommitHash::new("a".repeat(40)).unwrap(),
            "2026-01-31T10:30:00Z",
        );
        lock.insert("z-dep".to_string(), entry_z);

        let entry_a = LockEntry::new(
            GitUrl::new("https://github.com/org/a.git").unwrap(),
            GitRef::new("v2.0.0").unwrap(),
            CommitHash::new("b".repeat(40)).unwrap(),
            "2026-01-31T10:30:00Z",
        );
        lock.insert("a-dep".to_string(), entry_a);

        let entry_m = LockEntry::new(
            GitUrl::new("https://github.com/org/m.git").unwrap(),
            GitRef::new("v3.0.0").unwrap(),
            CommitHash::new("c".repeat(40)).unwrap(),
            "2026-01-31T10:30:00Z",
        );
        lock.insert("m-dep".to_string(), entry_m);

        // Convert to ordered
        let ordered = to_ordered_lock(&lock);

        // Check order
        let keys: Vec<_> = ordered.dependencies.keys().cloned().collect();
        assert_eq!(keys, vec!["a-dep", "m-dep", "z-dep"]);
    }

    #[test]
    fn round_trip_preserves_data() {
        let mut lock = LockFile::new();

        let entry = LockEntry::new(
            GitUrl::new("https://github.com/org/meta-kb.git").unwrap(),
            GitRef::new("v2.0.0").unwrap(),
            CommitHash::new("abc123def456789012345678901234567890abcd").unwrap(),
            "2026-01-31T10:30:00Z",
        );
        lock.insert("meta-kb".to_string(), entry);

        // Serialize
        let ordered = to_ordered_lock(&lock);
        let yaml = serde_yml::to_string(&ordered).unwrap();

        // Deserialize
        let parsed = parse_lock_file_str(&yaml, "test.lock").unwrap();

        // Compare
        assert_eq!(lock.api_version, parsed.api_version);
        assert_eq!(lock.dependencies.len(), parsed.dependencies.len());

        let original_entry = lock.get("meta-kb").unwrap();
        let parsed_entry = parsed.get("meta-kb").unwrap();
        assert_eq!(original_entry.source.as_str(), parsed_entry.source.as_str());
        assert_eq!(
            original_entry.git_ref.as_str(),
            parsed_entry.git_ref.as_str()
        );
        assert_eq!(original_entry.commit.as_str(), parsed_entry.commit.as_str());
        assert_eq!(original_entry.consumed_at, parsed_entry.consumed_at);
    }
}
