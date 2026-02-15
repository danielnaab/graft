//! Lock file mutation operations.
//!
//! Functions for updating lock file state without running migrations.

use graft_core::domain::{CommitHash, GraftConfig, LockEntry, LockFile};
use graft_core::error::{GraftError, Result};
use std::path::Path;

use crate::lock::{parse_lock_file, write_lock_file};

/// Apply a dependency version to lock file without running migrations.
///
/// Updates the lock file to record a specific version of a dependency.
/// Does NOT run migration or verification commands - intended for manual
/// migration workflows or initial setup.
///
/// # Arguments
///
/// * `config` - Graft configuration containing dependency specs
/// * `lock_path` - Path to graft.lock file
/// * `dep_name` - Name of dependency to apply
/// * `target_ref` - Git ref to apply (e.g., "main", "v1.0.0")
/// * `deps_directory` - Path to .graft directory
///
/// # Returns
///
/// * `Ok(ApplyResult)` - Successfully applied, with commit hash
/// * `Err(GraftError)` - If dependency not found, not resolved, or ref invalid
///
/// # Examples
///
/// ```no_run
/// use graft_engine::{parse_graft_yaml, apply_lock};
///
/// let config = parse_graft_yaml("graft.yaml").unwrap();
/// let result = apply_lock(&config, "graft.lock", "meta-kb", "v2.0.0", ".graft").unwrap();
/// println!("Applied {}: {}", result.name, result.commit.as_str());
/// ```
pub fn apply_lock(
    config: &GraftConfig,
    lock_path: impl AsRef<Path>,
    dep_name: &str,
    target_ref: &str,
    deps_directory: &str,
) -> Result<ApplyResult> {
    let lock_path = lock_path.as_ref();

    // Check dependency exists in config
    let dep_spec =
        config
            .dependencies
            .get(dep_name)
            .ok_or_else(|| GraftError::DependencyNotFound {
                name: dep_name.to_string(),
            })?;

    // Check dependency is resolved (directory exists)
    let dep_path = Path::new(deps_directory).join(dep_name);
    if !dep_path.exists() {
        return Err(GraftError::Resolution(format!(
            "Dependency '{dep_name}' not resolved (expected at {})",
            dep_path.display()
        )));
    }

    // Fetch the target ref (best effort - may fail for local-only repos)
    let _ = fetch_ref(&dep_path, target_ref);

    // Resolve ref to commit hash
    let commit_str = resolve_ref(&dep_path, target_ref)?;
    let commit = CommitHash::new(commit_str)?;

    // Create or load lock file
    let mut lock = if lock_path.exists() {
        parse_lock_file(lock_path)?
    } else {
        LockFile::new()
    };

    // Get current timestamp in ISO 8601 format
    let consumed_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    // Create/update lock entry
    let lock_entry = LockEntry {
        source: dep_spec.git_url.clone(),
        git_ref: graft_core::domain::GitRef::new(target_ref)?,
        commit: commit.clone(),
        consumed_at,
    };

    lock.dependencies.insert(dep_name.to_string(), lock_entry);

    // Write lock file
    write_lock_file(lock_path, &lock)?;

    Ok(ApplyResult {
        name: dep_name.to_string(),
        source: dep_spec.git_url.as_str().to_string(),
        git_ref: target_ref.to_string(),
        commit,
    })
}

/// Result of applying a dependency version.
#[derive(Debug, Clone)]
pub struct ApplyResult {
    /// Dependency name
    pub name: String,
    /// Git source URL
    pub source: String,
    /// Git ref that was applied
    pub git_ref: String,
    /// Resolved commit hash
    pub commit: CommitHash,
}

/// Fetch a ref from remote origin (best effort).
///
/// Returns success/failure but doesn't error - fetching may fail for
/// local-only repositories.
fn fetch_ref(repo_path: &Path, git_ref: &str) -> Result<()> {
    use std::process::Command;

    let output = Command::new("git")
        .args([
            "-C",
            repo_path.to_str().unwrap(),
            "fetch",
            "origin",
            git_ref,
        ])
        .output()?;

    if output.status.success() {
        Ok(())
    } else {
        // Don't error - just return failure for local-only repos
        Err(GraftError::Git(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }
}

/// Resolve a git ref to a commit hash.
///
/// Tries multiple strategies:
/// 1. Direct resolution of ref
/// 2. Resolution of origin/<ref> for branches
fn resolve_ref(repo_path: &Path, git_ref: &str) -> Result<String> {
    use std::process::Command;

    // Try direct resolution first
    let output = Command::new("git")
        .args(["-C", repo_path.to_str().unwrap(), "rev-parse", git_ref])
        .output()?;

    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
    }

    // Try origin/<ref> for branches
    let origin_ref = format!("origin/{git_ref}");
    let output = Command::new("git")
        .args(["-C", repo_path.to_str().unwrap(), "rev-parse", &origin_ref])
        .output()?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(GraftError::Git(format!(
            "Could not resolve ref '{git_ref}'"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use graft_core::domain::{DependencySpec, GitRef, GitUrl};
    use std::collections::HashMap;

    #[test]
    fn apply_result_created() {
        let commit = CommitHash::new("abc123def456789012345678901234567890abcd").unwrap();
        let result = ApplyResult {
            name: "test-dep".to_string(),
            source: "https://github.com/org/repo.git".to_string(),
            git_ref: "v1.0.0".to_string(),
            commit: commit.clone(),
        };

        assert_eq!(result.name, "test-dep");
        assert_eq!(result.source, "https://github.com/org/repo.git");
        assert_eq!(result.git_ref, "v1.0.0");
        assert_eq!(
            result.commit.as_str(),
            "abc123def456789012345678901234567890abcd"
        );
    }

    #[test]
    fn apply_lock_fails_on_missing_dependency() {
        let config = GraftConfig::new("graft/v0").unwrap();
        let result = apply_lock(&config, "graft.lock", "nonexistent", "v1.0.0", ".graft");

        assert!(result.is_err());
        if let Err(GraftError::DependencyNotFound { name }) = result {
            assert_eq!(name, "nonexistent");
        } else {
            panic!("Expected DependencyNotFound error");
        }
    }

    #[test]
    fn apply_lock_fails_on_unresolved_dependency() {
        let mut config = GraftConfig::new("graft/v0").unwrap();
        let spec = DependencySpec {
            name: "test-dep".to_string(),
            git_url: GitUrl::new("https://github.com/org/repo.git").unwrap(),
            git_ref: GitRef::new("main").unwrap(),
        };
        let mut deps = HashMap::new();
        deps.insert("test-dep".to_string(), spec);
        config.dependencies = deps;

        let result = apply_lock(
            &config,
            "graft.lock",
            "test-dep",
            "v1.0.0",
            "/nonexistent/deps",
        );

        assert!(result.is_err());
        if let Err(GraftError::Resolution(msg)) = result {
            assert!(msg.contains("not resolved"));
        } else {
            panic!("Expected Resolution error");
        }
    }
}
