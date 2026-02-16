//! Dependency resolution service.
//!
//! Implements the flat-only resolution model: only direct dependencies
//! declared in graft.yaml are resolved as git submodules.

use graft_common::command::run_command_with_timeout;
use graft_common::git::{get_current_commit as git_get_current_commit, is_git_repo};
use graft_core::domain::{CommitHash, LockEntry, LockFile};
use graft_core::{DependencySpec, GraftConfig, GraftError};
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

/// Result of resolving a single dependency
#[derive(Debug, Clone)]
pub struct ResolutionResult {
    /// Dependency name
    pub name: String,
    /// Whether resolution succeeded
    pub success: bool,
    /// Local path where dependency was placed
    pub local_path: Option<PathBuf>,
    /// Error message if resolution failed
    pub error: Option<String>,
    /// Whether the dependency was newly cloned
    pub newly_cloned: bool,
}

impl ResolutionResult {
    /// Create a successful resolution result
    #[must_use]
    pub fn success(name: impl Into<String>, local_path: PathBuf, newly_cloned: bool) -> Self {
        Self {
            name: name.into(),
            success: true,
            local_path: Some(local_path),
            error: None,
            newly_cloned,
        }
    }

    /// Create a failed resolution result
    #[must_use]
    pub fn failure(name: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            success: false,
            local_path: None,
            error: Some(error.into()),
            newly_cloned: false,
        }
    }
}

/// Check if a path is a git submodule
fn is_submodule(path: &Path) -> Result<bool, GraftError> {
    let mut cmd = ProcessCommand::new("git");
    cmd.args(["submodule", "status", &path.display().to_string()]);

    let output = run_command_with_timeout(cmd, "git submodule status", None)
        .map_err(|e| GraftError::Resolution(format!("Failed to check submodule status: {e}")))?;

    Ok(output.status.success() && !output.stdout.is_empty())
}

/// Check if a path is a git repository
fn is_repository(path: &Path) -> bool {
    is_git_repo(path)
}

/// Add a new git submodule
fn add_submodule(url: &str, path: &Path) -> Result<(), GraftError> {
    let mut cmd = ProcessCommand::new("git");
    cmd.args(["submodule", "add", url, &path.display().to_string()]);

    let output = run_command_with_timeout(cmd, "git submodule add", None)
        .map_err(|e| GraftError::Resolution(format!("Failed to add submodule: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GraftError::Resolution(format!(
            "git submodule add failed: {stderr}"
        )));
    }

    Ok(())
}

/// Update existing submodule (init if needed)
fn update_submodule(path: &Path) -> Result<(), GraftError> {
    let mut cmd = ProcessCommand::new("git");
    cmd.args(["submodule", "update", "--init", &path.display().to_string()]);

    let output = run_command_with_timeout(cmd, "git submodule update", None)
        .map_err(|e| GraftError::Resolution(format!("Failed to update submodule: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GraftError::Resolution(format!(
            "git submodule update failed: {stderr}"
        )));
    }

    Ok(())
}

/// Fetch all refs from remote
fn fetch_all(path: &Path) -> Result<(), GraftError> {
    graft_common::git::git_fetch(path).map_err(|e| GraftError::Resolution(e.to_string()))
}

/// Resolve a git ref to a commit hash
pub(crate) fn resolve_ref(path: &Path, git_ref: &str) -> Result<String, GraftError> {
    graft_common::git::git_rev_parse(path, git_ref)
        .map_err(|e| GraftError::Resolution(e.to_string()))
}

/// Get current commit hash
pub(crate) fn get_current_commit(path: &Path) -> Result<String, GraftError> {
    git_get_current_commit(path).map_err(|e| GraftError::Resolution(e.to_string()))
}

/// Checkout a specific commit
fn checkout(path: &Path, commit: &str) -> Result<(), GraftError> {
    graft_common::git::git_checkout(path, commit).map_err(|e| GraftError::Resolution(e.to_string()))
}

/// Resolve a single dependency as a git submodule
///
/// This implements the core resolution logic:
/// 1. If submodule exists: fetch and checkout ref
/// 2. If path exists but isn't submodule: error (legacy clone)
/// 3. If path doesn't exist: add as submodule and checkout ref
pub fn resolve_dependency(
    spec: &DependencySpec,
    deps_directory: &str,
) -> Result<ResolutionResult, GraftError> {
    let local_path = PathBuf::from(format!("{deps_directory}/{}", spec.name));

    // Check if submodule already exists
    if is_submodule(&local_path)? {
        // Update existing submodule
        update_submodule(&local_path)?;

        // Fetch all refs
        fetch_all(&local_path)?;

        // Resolve ref to commit
        let resolved_commit = resolve_ref(&local_path, spec.git_ref.as_str())?;

        // Check if we need to checkout
        let current_commit = get_current_commit(&local_path)?;
        if current_commit != resolved_commit {
            checkout(&local_path, &resolved_commit)?;
        }

        Ok(ResolutionResult::success(
            &spec.name, local_path, false, // Not newly cloned
        ))
    } else if local_path.exists() {
        // Path exists but isn't a submodule
        if is_repository(&local_path) {
            // Legacy clone detected
            Err(GraftError::Resolution(format!(
                "Legacy clone detected at {}. Delete it and re-run resolve: rm -rf {}",
                local_path.display(),
                local_path.display()
            )))
        } else {
            Err(GraftError::Resolution(format!(
                "Path exists but is not a git repository: {}",
                local_path.display()
            )))
        }
    } else {
        // Add new submodule
        add_submodule(spec.git_url.as_str(), &local_path)?;

        // Resolve ref to commit and checkout
        let resolved_commit = resolve_ref(&local_path, spec.git_ref.as_str())?;
        checkout(&local_path, &resolved_commit)?;

        Ok(ResolutionResult::success(
            &spec.name, local_path, true, // Newly cloned
        ))
    }
}

/// Resolve all dependencies from configuration
///
/// Continues on failure to attempt all dependencies.
/// Returns results for all dependencies.
pub fn resolve_all_dependencies(
    config: &GraftConfig,
    deps_directory: &str,
) -> Vec<ResolutionResult> {
    let mut results = Vec::new();

    for spec in config.dependencies.values() {
        let result = match resolve_dependency(spec, deps_directory) {
            Ok(res) => res,
            Err(e) => ResolutionResult::failure(&spec.name, e.to_string()),
        };
        results.push(result);
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolution_result_success() {
        let result = ResolutionResult::success("test-dep", PathBuf::from("/path"), true);
        assert!(result.success);
        assert_eq!(result.name, "test-dep");
        assert!(result.newly_cloned);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_resolution_result_failure() {
        let result = ResolutionResult::failure("test-dep", "connection failed");
        assert!(!result.success);
        assert_eq!(result.name, "test-dep");
        assert!(!result.newly_cloned);
        assert_eq!(result.error.as_deref(), Some("connection failed"));
    }

    #[test]
    fn test_is_repository() {
        // Repo root should be a git repo (find it by going up from current dir)
        let mut path = std::env::current_dir().unwrap();
        while !is_repository(&path) && path.parent().is_some() {
            path = path.parent().unwrap().to_path_buf();
        }
        // At least one parent should be a git repo (or current dir is already)
        assert!(is_repository(&path) || is_repository(Path::new(".")));

        // Non-existent path should not be a repo
        assert!(!is_repository(Path::new("/nonexistent/path")));
    }
}

/// Resolve all dependencies and create a lock file
///
/// This combines resolution with lock file creation, ensuring that
/// the lock file accurately reflects the resolved state of all dependencies.
///
/// Returns the lock file on success, or an error if any dependency fails to resolve.
pub fn resolve_and_create_lock(
    config: &GraftConfig,
    deps_directory: &str,
) -> Result<LockFile, GraftError> {
    use std::collections::HashMap;

    let mut lock_entries = HashMap::new();

    // Get current timestamp in ISO 8601 format
    let consumed_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    for spec in config.dependencies.values() {
        let local_path = PathBuf::from(format!("{deps_directory}/{}", spec.name));

        // Resolve the dependency (will error if resolution fails)
        let _result = resolve_dependency(spec, deps_directory)?;

        // Get the current commit hash
        let commit_str = get_current_commit(&local_path)?;
        let commit = CommitHash::new(commit_str)?;

        // Create lock entry
        let lock_entry = LockEntry {
            source: spec.git_url.clone(),
            git_ref: spec.git_ref.clone(),
            commit,
            consumed_at: consumed_at.clone(),
        };

        lock_entries.insert(spec.name.clone(), lock_entry);
    }

    Ok(LockFile {
        api_version: config.api_version.clone(),
        dependencies: lock_entries,
    })
}

/// Result of fetching a single dependency
#[derive(Debug, Clone)]
pub struct FetchResult {
    /// Dependency name
    pub name: String,
    /// Whether fetch succeeded
    pub success: bool,
    /// Error message if fetch failed
    pub error: Option<String>,
}

impl FetchResult {
    /// Create a successful fetch result
    #[must_use]
    pub fn success(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            success: true,
            error: None,
        }
    }

    /// Create a failed fetch result
    #[must_use]
    pub fn failure(name: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            success: false,
            error: Some(error.into()),
        }
    }
}

/// Fetch a single dependency's remote state
///
/// Updates the local cache of remote refs without modifying the checked-out commit.
pub fn fetch_dependency(name: &str, deps_directory: &str) -> Result<FetchResult, GraftError> {
    let local_path = PathBuf::from(format!("{deps_directory}/{name}"));

    // Check if dependency exists
    if !local_path.exists() {
        return Ok(FetchResult::failure(
            name,
            "not cloned (run 'graft resolve')",
        ));
    }

    // Check if it's a git repository
    if !is_repository(&local_path) {
        return Ok(FetchResult::failure(name, "not a git repository"));
    }

    // Fetch from remote
    fetch_all(&local_path)?;

    Ok(FetchResult::success(name))
}

/// Fetch all dependencies from configuration
///
/// Continues on failure to attempt all dependencies.
pub fn fetch_all_dependencies(config: &GraftConfig, deps_directory: &str) -> Vec<FetchResult> {
    let mut results = Vec::new();

    for name in config.dependencies.keys() {
        let result = match fetch_dependency(name, deps_directory) {
            Ok(res) => res,
            Err(e) => FetchResult::failure(name, e.to_string()),
        };
        results.push(result);
    }

    results
}

/// Result of syncing a single dependency
#[derive(Debug, Clone)]
pub struct SyncResult {
    /// Dependency name
    pub name: String,
    /// Whether sync succeeded
    pub success: bool,
    /// Action taken: "cloned", "`checked_out`", "`up_to_date`"
    pub action: String,
    /// Human-readable message
    pub message: String,
}

impl SyncResult {
    /// Create a successful sync result
    #[must_use]
    pub fn success(
        name: impl Into<String>,
        action: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            success: true,
            action: action.into(),
            message: message.into(),
        }
    }

    /// Create a failed sync result
    #[must_use]
    pub fn failure(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            success: false,
            action: "failed".to_string(),
            message: message.into(),
        }
    }
}

/// Sync a single dependency to match lock file state
///
/// Ensures the dependency exists and is checked out to the commit
/// specified in the lock file.
pub fn sync_dependency(
    name: &str,
    entry: &LockEntry,
    deps_directory: &str,
) -> Result<SyncResult, GraftError> {
    let local_path = PathBuf::from(format!("{deps_directory}/{name}"));

    // Check if it's a submodule
    if is_submodule(&local_path)? {
        // Update submodule (init if needed)
        update_submodule(&local_path)?;

        // Get current commit
        let current_commit = get_current_commit(&local_path)?;

        // Check if already at correct commit
        if current_commit == entry.commit.as_str() {
            return Ok(SyncResult::success(
                name,
                "up_to_date",
                format!("Already at {}", &entry.commit.as_str()[..7]),
            ));
        }

        // Checkout the locked commit
        checkout(&local_path, entry.commit.as_str())?;

        Ok(SyncResult::success(
            name,
            "checked_out",
            format!("Checked out to {}", &entry.commit.as_str()[..7]),
        ))
    } else if local_path.exists() {
        // Path exists but isn't a submodule
        if is_repository(&local_path) {
            // Legacy clone - sync it but warn
            let current_commit = get_current_commit(&local_path)?;

            if current_commit == entry.commit.as_str() {
                return Ok(SyncResult::success(
                    name,
                    "up_to_date",
                    format!(
                        "Already at {} (legacy clone - delete and re-resolve)",
                        &entry.commit.as_str()[..7]
                    ),
                ));
            }

            // Fetch and checkout
            fetch_all(&local_path)?;
            checkout(&local_path, entry.commit.as_str())?;

            Ok(SyncResult::success(
                name,
                "checked_out",
                format!(
                    "Checked out {} (legacy clone - delete and re-resolve)",
                    &entry.commit.as_str()[..7]
                ),
            ))
        } else {
            Ok(SyncResult::failure(
                name,
                format!(
                    "Path exists but is not a git repository: {}",
                    local_path.display()
                ),
            ))
        }
    } else {
        // Dependency doesn't exist - add as submodule
        add_submodule(entry.source.as_str(), &local_path)?;

        // Checkout the exact commit
        checkout(&local_path, entry.commit.as_str())?;

        Ok(SyncResult::success(
            name,
            "cloned",
            format!(
                "Added submodule and checked out {}",
                &entry.commit.as_str()[..7]
            ),
        ))
    }
}

/// Sync all dependencies to match lock file state
///
/// Continues on failure to attempt all dependencies.
pub fn sync_all_dependencies(lock_file: &LockFile, deps_directory: &str) -> Vec<SyncResult> {
    let mut results = Vec::new();

    for (name, entry) in &lock_file.dependencies {
        let result = match sync_dependency(name, entry, deps_directory) {
            Ok(res) => res,
            Err(e) => SyncResult::failure(name, e.to_string()),
        };
        results.push(result);
    }

    results
}
