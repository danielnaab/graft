//! Domain types for Graft.

use crate::error::{GraftError, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A validated git reference (branch, tag, or commit SHA).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct GitRef(String);

impl GitRef {
    pub fn new(ref_str: impl Into<String>) -> Result<Self> {
        let ref_str = ref_str.into();
        if ref_str.is_empty() {
            return Err(GraftError::InvalidGitRef(
                "git ref cannot be empty".to_string(),
            ));
        }
        if ref_str.trim().is_empty() {
            return Err(GraftError::InvalidGitRef(
                "git ref cannot be only whitespace".to_string(),
            ));
        }
        Ok(Self(ref_str))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for GitRef {
    type Error = GraftError;

    fn try_from(value: String) -> Result<Self> {
        Self::new(value)
    }
}

impl From<GitRef> for String {
    fn from(git_ref: GitRef) -> Self {
        git_ref.0
    }
}

impl std::fmt::Display for GitRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A validated git repository URL.
/// Supports ssh://, https://, http://, file://, and SCP-style URLs (git@host:path).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct GitUrl(String);

impl GitUrl {
    pub fn new(url: impl Into<String>) -> Result<Self> {
        let url = url.into();
        if url.is_empty() {
            return Err(GraftError::InvalidGitUrl(
                "git URL cannot be empty".to_string(),
            ));
        }

        let normalized = Self::normalize_git_url(&url);

        // Validate URL scheme if present
        if let Some(scheme_end) = normalized.find("://") {
            let scheme = &normalized[..scheme_end];
            if !matches!(scheme, "ssh" | "https" | "http" | "file" | "git") {
                return Err(GraftError::InvalidGitUrl(format!(
                    "invalid URL scheme: {scheme}. Supported: ssh, https, http, file, git"
                )));
            }
        }

        Ok(Self(normalized))
    }

    /// Normalize git URL to a format that git understands.
    /// Handles SCP-style URLs (git@host:path) and mixed format (ssh://git@host:path).
    fn normalize_git_url(url: &str) -> String {
        // SCP-style pattern: user@host:path (no scheme)
        // We manually check that the path doesn't start with "//" to avoid false matches
        let scp_pattern = Regex::new(r"^([^@]+)@([^:]+):(.+)$").unwrap();

        // Check if it's an SCP-style URL (no scheme)
        if !url.contains("://") {
            if let Some(captures) = scp_pattern.captures(url) {
                let user = &captures[1];
                let host = &captures[2];
                let path = &captures[3];
                // Don't match if path starts with "//" (that would be a URL with port)
                if !path.starts_with("//") {
                    return format!("ssh://{user}@{host}/{path}");
                }
            }
        }

        // Mixed format pattern: ssh://user@host:path (scheme with colon-separated path)
        // We manually check that path doesn't start with "//" or a digit (port number)
        let mixed_pattern = Regex::new(r"^(ssh|git)://([^@]+)@([^:/]+):(.+)$").unwrap();
        if let Some(captures) = mixed_pattern.captures(url) {
            let scheme = &captures[1];
            let user = &captures[2];
            let host = &captures[3];
            let path = &captures[4];
            // Don't match if path starts with "//" or a digit (indicating a port)
            if !path.starts_with("//") && !path.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                return format!("{scheme}://{user}@{host}/{path}");
            }
        }

        url.to_string()
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for GitUrl {
    type Error = GraftError;

    fn try_from(value: String) -> Result<Self> {
        Self::new(value)
    }
}

impl From<GitUrl> for String {
    fn from(url: GitUrl) -> Self {
        url.0
    }
}

impl std::fmt::Display for GitUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A dependency specification from graft.yaml.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencySpec {
    pub name: String,
    pub git_url: GitUrl,
    pub git_ref: GitRef,
}

impl DependencySpec {
    pub fn new(name: impl Into<String>, git_url: GitUrl, git_ref: GitRef) -> Result<Self> {
        let name = name.into();

        // Validate name
        if name.is_empty() {
            return Err(GraftError::InvalidDependencyName(
                "dependency name cannot be empty".to_string(),
            ));
        }
        if name.len() > 100 {
            return Err(GraftError::InvalidDependencyName(format!(
                "dependency name too long: {} chars",
                name.len()
            )));
        }
        if name.contains('/') || name.contains('\\') {
            return Err(GraftError::InvalidDependencyName(format!(
                "invalid dependency name: {name}. Cannot contain path separators."
            )));
        }

        Ok(Self {
            name,
            git_url,
            git_ref,
        })
    }
}

/// A semantic change in a dependency, identified by a git ref.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Change {
    pub ref_name: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub change_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub migration: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verify: Option<String>,
    #[serde(flatten)]
    pub metadata: HashMap<String, serde_yaml::Value>,
}

impl Change {
    pub fn new(ref_name: impl Into<String>) -> Result<Self> {
        let ref_name = ref_name.into();

        if ref_name.is_empty() {
            return Err(GraftError::Validation(
                "change ref cannot be empty".to_string(),
            ));
        }
        if ref_name.trim().is_empty() {
            return Err(GraftError::Validation(
                "change ref cannot be only whitespace".to_string(),
            ));
        }

        Ok(Self {
            ref_name,
            change_type: None,
            description: None,
            migration: None,
            verify: None,
            metadata: HashMap::new(),
        })
    }

    #[must_use]
    pub fn with_type(mut self, change_type: impl Into<String>) -> Self {
        self.change_type = Some(change_type.into());
        self
    }

    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        let desc = description.into();
        self.description = if desc.len() > 200 {
            // Validation happens during GraftConfig construction
            Some(desc)
        } else {
            Some(desc)
        };
        self
    }

    #[must_use]
    pub fn with_migration(mut self, migration: impl Into<String>) -> Self {
        self.migration = Some(migration.into());
        self
    }

    #[must_use]
    pub fn with_verify(mut self, verify: impl Into<String>) -> Self {
        self.verify = Some(verify.into());
        self
    }

    pub fn needs_migration(&self) -> bool {
        self.migration.is_some()
    }

    pub fn needs_verification(&self) -> bool {
        self.verify.is_some()
    }

    pub fn is_breaking(&self) -> bool {
        self.change_type.as_deref() == Some("breaking")
    }

    fn validate(&self) -> Result<()> {
        if let Some(desc) = &self.description {
            if desc.len() > 200 {
                return Err(GraftError::Validation(format!(
                    "change description too long: {} chars (max 200)",
                    desc.len()
                )));
            }
        }

        if let Some(migration) = &self.migration {
            if migration.trim().is_empty() {
                return Err(GraftError::Validation(
                    "migration command name cannot be only whitespace".to_string(),
                ));
            }
        }

        if let Some(verify) = &self.verify {
            if verify.trim().is_empty() {
                return Err(GraftError::Validation(
                    "verify command name cannot be only whitespace".to_string(),
                ));
            }
        }

        Ok(())
    }
}

/// An executable command defined in a dependency's graft.yaml.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Command {
    #[serde(skip)]
    pub name: String,
    pub run: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
}

impl Command {
    pub fn new(name: impl Into<String>, run: impl Into<String>) -> Result<Self> {
        let name = name.into();
        let run = run.into();

        // Validate name
        if name.is_empty() {
            return Err(GraftError::InvalidCommandName(
                "command name cannot be empty".to_string(),
            ));
        }
        if name.trim().is_empty() {
            return Err(GraftError::InvalidCommandName(
                "command name cannot be only whitespace".to_string(),
            ));
        }
        if name.len() > 100 {
            return Err(GraftError::InvalidCommandName(format!(
                "command name too long: {} chars (max 100)",
                name.len()
            )));
        }
        if name.contains(':') {
            return Err(GraftError::InvalidCommandName(format!(
                "command name '{name}' cannot contain ':' (reserved separator). Use '{}' instead.",
                name.replace(':', "-")
            )));
        }

        // Validate run
        if run.is_empty() {
            return Err(GraftError::Validation(format!(
                "command '{name}': 'run' field is required"
            )));
        }
        if run.trim().is_empty() {
            return Err(GraftError::Validation(format!(
                "command '{name}': 'run' field cannot be only whitespace"
            )));
        }

        Ok(Self {
            name,
            run,
            description: None,
            working_dir: None,
            env: None,
        })
    }

    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    #[must_use]
    pub fn with_working_dir(mut self, working_dir: impl Into<String>) -> Self {
        self.working_dir = Some(working_dir.into());
        self
    }

    #[must_use]
    pub fn with_env(mut self, env: HashMap<String, String>) -> Self {
        self.env = Some(env);
        self
    }

    pub fn has_env_vars(&self) -> bool {
        self.env.as_ref().is_some_and(|e| !e.is_empty())
    }

    fn validate(&self) -> Result<()> {
        if let Some(desc) = &self.description {
            if desc.len() > 500 {
                return Err(GraftError::Validation(format!(
                    "command '{}': description too long: {} chars (max 500)",
                    self.name,
                    desc.len()
                )));
            }
        }

        if let Some(working_dir) = &self.working_dir {
            if working_dir.trim().is_empty() {
                return Err(GraftError::Validation(format!(
                    "command '{}': working_dir cannot be only whitespace",
                    self.name
                )));
            }
            // Check for absolute paths (should be relative)
            if working_dir.starts_with('/')
                || (working_dir.len() > 1 && working_dir.chars().nth(1) == Some(':'))
            {
                return Err(GraftError::Validation(format!(
                    "command '{}': working_dir must be relative, not absolute",
                    self.name
                )));
            }
        }

        Ok(())
    }
}

/// Optional metadata section from graft.yaml.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Metadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changelog: Option<String>,
}

/// Complete graft.yaml configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraftConfig {
    #[serde(rename = "apiVersion")]
    pub api_version: String,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub dependencies: HashMap<String, DependencySpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub changes: HashMap<String, Change>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub commands: HashMap<String, Command>,
}

impl GraftConfig {
    pub fn new(api_version: impl Into<String>) -> Result<Self> {
        let api_version = api_version.into();

        if api_version.is_empty() {
            return Err(GraftError::Validation(
                "API version cannot be empty".to_string(),
            ));
        }
        if !api_version.starts_with("graft/") {
            return Err(GraftError::Validation(format!(
                "invalid API version: {api_version}. Must start with 'graft/'"
            )));
        }

        Ok(Self {
            api_version,
            dependencies: HashMap::new(),
            metadata: None,
            changes: HashMap::new(),
            commands: HashMap::new(),
        })
    }

    #[must_use]
    pub fn with_metadata(mut self, metadata: Metadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    #[must_use]
    pub fn add_dependency(mut self, spec: DependencySpec) -> Self {
        self.dependencies.insert(spec.name.clone(), spec);
        self
    }

    #[must_use]
    pub fn add_change(mut self, ref_name: String, change: Change) -> Self {
        self.changes.insert(ref_name, change);
        self
    }

    #[must_use]
    pub fn add_command(mut self, name: String, command: Command) -> Self {
        self.commands.insert(name, command);
        self
    }

    /// Validate the configuration.
    /// This checks cross-field constraints (e.g., migration commands must exist).
    pub fn validate(&self) -> Result<()> {
        // Validate that migration/verify commands exist
        for (ref_name, change) in &self.changes {
            change.validate()?;

            if let Some(migration) = &change.migration {
                if !self.commands.contains_key(migration) {
                    return Err(GraftError::ConfigValidation {
                        path: "graft.yaml".to_string(),
                        field: format!("changes.{ref_name}.migration"),
                        reason: format!(
                            "migration command '{migration}' not found in commands section"
                        ),
                    });
                }
            }

            if let Some(verify) = &change.verify {
                if !self.commands.contains_key(verify) {
                    return Err(GraftError::ConfigValidation {
                        path: "graft.yaml".to_string(),
                        field: format!("changes.{ref_name}.verify"),
                        reason: format!("verify command '{verify}' not found in commands section"),
                    });
                }
            }
        }

        // Validate commands
        for command in self.commands.values() {
            command.validate()?;
        }

        Ok(())
    }

    pub fn has_dependency(&self, name: &str) -> bool {
        self.dependencies.contains_key(name)
    }

    pub fn get_dependency(&self, name: &str) -> Option<&DependencySpec> {
        self.dependencies.get(name)
    }

    pub fn has_change(&self, ref_name: &str) -> bool {
        self.changes.contains_key(ref_name)
    }

    pub fn get_change(&self, ref_name: &str) -> Option<&Change> {
        self.changes.get(ref_name)
    }

    pub fn has_command(&self, name: &str) -> bool {
        self.commands.contains_key(name)
    }

    pub fn get_command(&self, name: &str) -> Option<&Command> {
        self.commands.get(name)
    }

    pub fn get_breaking_changes(&self) -> Vec<&Change> {
        self.changes.values().filter(|c| c.is_breaking()).collect()
    }

    pub fn get_changes_needing_migration(&self) -> Vec<&Change> {
        self.changes
            .values()
            .filter(|c| c.needs_migration())
            .collect()
    }
}

/// A validated git commit hash (40-character SHA-1).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CommitHash(String);

impl CommitHash {
    pub fn new(hash: impl Into<String>) -> Result<Self> {
        let hash = hash.into();

        // Must be exactly 40 lowercase hex characters
        if hash.len() != 40 {
            return Err(GraftError::InvalidCommitHash(format!(
                "commit hash must be 40 characters, got {}",
                hash.len()
            )));
        }

        // Validate hex characters (0-9, a-f lowercase)
        if !hash.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')) {
            return Err(GraftError::InvalidCommitHash(
                "commit hash must be lowercase hexadecimal (0-9, a-f)".to_string(),
            ));
        }

        Ok(Self(hash))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for CommitHash {
    type Error = GraftError;

    fn try_from(value: String) -> Result<Self> {
        Self::new(value)
    }
}

impl From<CommitHash> for String {
    fn from(hash: CommitHash) -> Self {
        hash.0
    }
}

impl std::fmt::Display for CommitHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A lock file entry tracking consumed dependency state.
///
/// Represents a single dependency in graft.lock with the exact commit
/// that has been consumed and when it was consumed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockEntry {
    /// Git URL or path to dependency repository
    pub source: GitUrl,
    /// Consumed git ref (tag, branch, or commit)
    #[serde(rename = "ref")]
    pub git_ref: GitRef,
    /// Full commit hash (40-char SHA-1)
    pub commit: CommitHash,
    /// ISO 8601 timestamp when version was consumed
    pub consumed_at: String,
}

impl LockEntry {
    #[must_use]
    pub fn new(
        source: GitUrl,
        git_ref: GitRef,
        commit: CommitHash,
        consumed_at: impl Into<String>,
    ) -> Self {
        Self {
            source,
            git_ref,
            commit,
            consumed_at: consumed_at.into(),
        }
    }

    /// Validate the timestamp format.
    ///
    /// Checks that `consumed_at` is a valid ISO 8601 timestamp.
    pub fn validate(&self) -> Result<()> {
        // Basic ISO 8601 validation - accept various formats
        let ts = &self.consumed_at;
        if ts.is_empty() {
            return Err(GraftError::InvalidTimestamp(
                "consumed_at cannot be empty".to_string(),
            ));
        }

        // Should contain a date part (YYYY-MM-DD)
        if !ts.contains('-') || ts.len() < 10 {
            return Err(GraftError::InvalidTimestamp(format!(
                "invalid ISO 8601 timestamp: {ts}"
            )));
        }

        Ok(())
    }
}

/// The graft.lock file structure.
///
/// Tracks exact state of consumed direct dependencies for reproducibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockFile {
    /// API version (e.g., "graft/v0")
    #[serde(rename = "apiVersion")]
    pub api_version: String,
    /// Map of dependency name to lock entry
    #[serde(default)]
    pub dependencies: HashMap<String, LockEntry>,
}

impl LockFile {
    const SUPPORTED_API_VERSION: &'static str = "graft/v0";

    #[must_use]
    pub fn new() -> Self {
        Self {
            api_version: Self::SUPPORTED_API_VERSION.to_string(),
            dependencies: HashMap::new(),
        }
    }

    /// Validate the lock file structure.
    pub fn validate(&self) -> Result<()> {
        // Validate API version
        if !self.api_version.starts_with("graft/") {
            return Err(GraftError::UnsupportedApiVersion(self.api_version.clone()));
        }

        if self.api_version != Self::SUPPORTED_API_VERSION {
            return Err(GraftError::UnsupportedApiVersion(self.api_version.clone()));
        }

        // Validate each lock entry
        for (name, entry) in &self.dependencies {
            entry
                .validate()
                .map_err(|e| GraftError::InvalidLockEntry(format!("dependency '{name}': {e}")))?;
        }

        Ok(())
    }

    /// Get a lock entry by dependency name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&LockEntry> {
        self.dependencies.get(name)
    }

    /// Insert or update a lock entry.
    pub fn insert(&mut self, name: String, entry: LockEntry) {
        self.dependencies.insert(name, entry);
    }

    /// Remove a lock entry by name.
    pub fn remove(&mut self, name: &str) -> Option<LockEntry> {
        self.dependencies.remove(name)
    }
}

impl Default for LockFile {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_ref_rejects_empty() {
        assert!(GitRef::new("").is_err());
        assert!(GitRef::new("  ").is_err());
    }

    #[test]
    fn git_ref_accepts_valid() {
        let git_ref = GitRef::new("main").unwrap();
        assert_eq!(git_ref.as_str(), "main");
    }

    #[test]
    fn git_url_rejects_empty() {
        assert!(GitUrl::new("").is_err());
    }

    #[test]
    fn git_url_normalizes_scp_style() {
        let url = GitUrl::new("git@github.com:user/repo.git").unwrap();
        assert_eq!(url.as_str(), "ssh://git@github.com/user/repo.git");
    }

    #[test]
    fn git_url_accepts_https() {
        let url = GitUrl::new("https://github.com/user/repo.git").unwrap();
        assert_eq!(url.as_str(), "https://github.com/user/repo.git");
    }

    #[test]
    fn dependency_spec_rejects_empty_name() {
        let url = GitUrl::new("https://github.com/user/repo.git").unwrap();
        let git_ref = GitRef::new("main").unwrap();
        assert!(DependencySpec::new("", url, git_ref).is_err());
    }

    #[test]
    fn dependency_spec_rejects_path_separators_in_name() {
        let url = GitUrl::new("https://github.com/user/repo.git").unwrap();
        let git_ref = GitRef::new("main").unwrap();
        assert!(DependencySpec::new("foo/bar", url, git_ref).is_err());
    }

    #[test]
    fn change_rejects_empty_ref() {
        assert!(Change::new("").is_err());
    }

    #[test]
    fn change_needs_migration_when_set() {
        let change = Change::new("v1.0.0").unwrap().with_migration("migrate-v1");
        assert!(change.needs_migration());
    }

    #[test]
    fn change_is_breaking_when_type_breaking() {
        let change = Change::new("v2.0.0").unwrap().with_type("breaking");
        assert!(change.is_breaking());
    }

    #[test]
    fn command_rejects_colon_in_name() {
        assert!(Command::new("test:unit", "npm test").is_err());
    }

    #[test]
    fn command_rejects_empty_run() {
        assert!(Command::new("test", "").is_err());
    }

    #[test]
    fn graft_config_validates_api_version() {
        assert!(GraftConfig::new("").is_err());
        assert!(GraftConfig::new("v1").is_err());
        assert!(GraftConfig::new("graft/v0").is_ok());
    }

    #[test]
    fn graft_config_validates_migration_command_exists() {
        let mut config = GraftConfig::new("graft/v0").unwrap();

        let change = Change::new("v1.0.0").unwrap().with_migration("migrate-v1");
        config.changes.insert("v1.0.0".to_string(), change);

        // Should fail because migrate-v1 command doesn't exist
        assert!(config.validate().is_err());

        // Add the command
        let command = Command::new("migrate-v1", "echo 'migrating'").unwrap();
        config.commands.insert("migrate-v1".to_string(), command);

        // Should now pass
        assert!(config.validate().is_ok());
    }

    #[test]
    fn commit_hash_validates_length() {
        assert!(CommitHash::new("abc").is_err());
        assert!(CommitHash::new("a".repeat(39)).is_err());
        assert!(CommitHash::new("a".repeat(41)).is_err());
        assert!(CommitHash::new("a".repeat(40)).is_ok());
    }

    #[test]
    fn commit_hash_validates_hex() {
        assert!(CommitHash::new("g".repeat(40)).is_err());
        assert!(CommitHash::new("ABCD".to_string() + &"a".repeat(36)).is_err()); // uppercase
        assert!(CommitHash::new("abc123def456789012345678901234567890abcd").is_ok());
    }

    #[test]
    fn lock_entry_validates_timestamp() {
        let source = GitUrl::new("https://github.com/user/repo.git").unwrap();
        let git_ref = GitRef::new("v1.0.0").unwrap();
        let commit = CommitHash::new("a".repeat(40)).unwrap();

        let entry = LockEntry::new(source.clone(), git_ref.clone(), commit.clone(), "");
        assert!(entry.validate().is_err());

        let entry = LockEntry::new(
            source.clone(),
            git_ref.clone(),
            commit.clone(),
            "2026-01-31T10:30:00Z",
        );
        assert!(entry.validate().is_ok());

        let entry = LockEntry::new(source, git_ref, commit, "2026-01-31T10:30:00.123456+00:00");
        assert!(entry.validate().is_ok());
    }

    #[test]
    fn lock_file_validates_api_version() {
        let mut lock = LockFile::new();
        lock.api_version = "v1".to_string();
        assert!(lock.validate().is_err());

        lock.api_version = "graft/v1".to_string();
        assert!(lock.validate().is_err());

        lock.api_version = "graft/v0".to_string();
        assert!(lock.validate().is_ok());
    }

    #[test]
    fn lock_file_crud_operations() {
        let mut lock = LockFile::new();
        assert!(lock.get("meta-kb").is_none());

        let source = GitUrl::new("https://github.com/org/meta-kb.git").unwrap();
        let git_ref = GitRef::new("v2.0.0").unwrap();
        let commit = CommitHash::new("abc123def456789012345678901234567890abcd").unwrap();
        let entry = LockEntry::new(source, git_ref, commit, "2026-01-31T10:30:00Z");

        lock.insert("meta-kb".to_string(), entry.clone());
        assert_eq!(lock.get("meta-kb"), Some(&entry));

        let removed = lock.remove("meta-kb");
        assert_eq!(removed, Some(entry));
        assert!(lock.get("meta-kb").is_none());
    }
}
