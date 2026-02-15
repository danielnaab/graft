//! Dependency management operations (add, remove).

use graft_core::{DependencySpec, GitRef, GitUrl, GraftError, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::lock::{parse_lock_file, write_lock_file};

/// Add a dependency to graft.yaml.
///
/// This function:
/// 1. Parses and validates the source#ref format
/// 2. Checks the dependency doesn't already exist
/// 3. Adds to the config and writes back to file
///
/// It does NOT resolve the dependency (clone/fetch). Use `resolve_dependency`
/// after calling this function if you want to clone immediately.
pub fn add_dependency_to_config(
    config_path: impl AsRef<Path>,
    name: impl Into<String>,
    source: impl Into<String>,
    git_ref: impl Into<String>,
) -> Result<()> {
    let config_path = config_path.as_ref();
    let name = name.into();
    let source = source.into();
    let git_ref = git_ref.into();

    // Read existing config
    let content = fs::read_to_string(config_path).map_err(|e| GraftError::ConfigParse {
        path: config_path.display().to_string(),
        reason: format!("failed to read file: {e}"),
    })?;

    // Parse as YAML value to preserve structure
    let mut yaml_data: serde_yaml::Value =
        serde_yaml::from_str(&content).map_err(|e| GraftError::ConfigParse {
            path: config_path.display().to_string(),
            reason: format!("invalid YAML: {e}"),
        })?;

    // Validate name (will be validated in DependencySpec::new, but check early)
    if name.is_empty() {
        return Err(GraftError::InvalidDependencyName(
            "dependency name cannot be empty".to_string(),
        ));
    }

    // Get or create deps section
    let deps_section = if let Some(mapping) = yaml_data.as_mapping_mut() {
        let deps_key = serde_yaml::Value::String("deps".to_string());
        mapping
            .entry(deps_key)
            .or_insert_with(|| serde_yaml::Value::Mapping(serde_yaml::Mapping::new()))
    } else {
        return Err(GraftError::ConfigParse {
            path: config_path.display().to_string(),
            reason: "config root must be a mapping".to_string(),
        });
    };

    // Check if dependency already exists
    let deps_mapping = deps_section
        .as_mapping_mut()
        .ok_or_else(|| GraftError::ConfigParse {
            path: config_path.display().to_string(),
            reason: "deps must be a mapping".to_string(),
        })?;

    let dep_key = serde_yaml::Value::String(name.clone());
    if deps_mapping.contains_key(&dep_key) {
        return Err(GraftError::Validation(format!(
            "dependency '{name}' already exists in config"
        )));
    }

    // Create DependencySpec to validate (but we won't use it for serialization)
    let git_url = GitUrl::new(&source)?;
    let git_ref_obj = GitRef::new(&git_ref)?;
    let _spec = DependencySpec::new(&name, git_url, git_ref_obj)?;

    // Add to deps section (as a string value)
    let dep_value = format!("{source}#{git_ref}");
    deps_mapping.insert(dep_key, serde_yaml::Value::String(dep_value));

    // Write back
    let new_content =
        serde_yaml::to_string(&yaml_data).map_err(|e| GraftError::Yaml(e.to_string()))?;
    fs::write(config_path, new_content)?;

    Ok(())
}

/// Remove a dependency from graft.yaml.
///
/// This function:
/// 1. Validates the dependency exists
/// 2. Removes it from the config
/// 3. Writes back to file
///
/// It does NOT remove the dependency directory or submodule. Use
/// `remove_submodule` for that.
pub fn remove_dependency_from_config(
    config_path: impl AsRef<Path>,
    name: impl Into<String>,
) -> Result<()> {
    let config_path = config_path.as_ref();
    let name = name.into();

    // Read existing config
    let content = fs::read_to_string(config_path).map_err(|e| GraftError::ConfigParse {
        path: config_path.display().to_string(),
        reason: format!("failed to read file: {e}"),
    })?;

    // Parse as YAML value to preserve structure
    let mut yaml_data: serde_yaml::Value =
        serde_yaml::from_str(&content).map_err(|e| GraftError::ConfigParse {
            path: config_path.display().to_string(),
            reason: format!("invalid YAML: {e}"),
        })?;

    // Get deps section
    let deps_section = yaml_data
        .as_mapping_mut()
        .and_then(|m| m.get_mut(serde_yaml::Value::String("deps".to_string())))
        .ok_or_else(|| GraftError::DependencyNotFound { name: name.clone() })?;

    let deps_mapping = deps_section
        .as_mapping_mut()
        .ok_or_else(|| GraftError::ConfigParse {
            path: config_path.display().to_string(),
            reason: "deps must be a mapping".to_string(),
        })?;

    // Check if dependency exists
    let dep_key = serde_yaml::Value::String(name.clone());
    if !deps_mapping.contains_key(&dep_key) {
        return Err(GraftError::DependencyNotFound { name });
    }

    // Remove the dependency
    deps_mapping.remove(&dep_key);

    // Write back
    let new_content =
        serde_yaml::to_string(&yaml_data).map_err(|e| GraftError::Yaml(e.to_string()))?;
    fs::write(config_path, new_content)?;

    Ok(())
}

/// Remove a dependency from graft.lock.
///
/// This is a separate operation from removing from config. It silently
/// succeeds if the lock file doesn't exist or the dependency isn't in it.
pub fn remove_dependency_from_lock(
    lock_path: impl AsRef<Path>,
    name: impl Into<String>,
) -> Result<()> {
    let lock_path = lock_path.as_ref();
    let name = name.into();

    // If lock file doesn't exist, nothing to do
    if !lock_path.exists() {
        return Ok(());
    }

    // Parse lock file
    let mut lock_file = parse_lock_file(lock_path)?;

    // Remove dependency if it exists
    lock_file.dependencies.remove(&name);

    // Write back
    write_lock_file(lock_path, &lock_file)?;

    Ok(())
}

/// Remove a git submodule.
///
/// This runs `git submodule deinit` and `git rm` to properly remove the
/// submodule from both the working tree and git's tracking.
pub fn remove_submodule(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    let path_str = path
        .to_str()
        .ok_or_else(|| GraftError::Git(format!("invalid UTF-8 in path: {}", path.display())))?;

    // Run git submodule deinit
    let deinit_output = Command::new("git")
        .args(["submodule", "deinit", "-f", path_str])
        .output()
        .map_err(|e| GraftError::Git(format!("failed to run git submodule deinit: {e}")))?;

    if !deinit_output.status.success() {
        let stderr = String::from_utf8_lossy(&deinit_output.stderr);
        return Err(GraftError::Git(format!(
            "git submodule deinit failed: {stderr}"
        )));
    }

    // Run git rm
    let rm_output = Command::new("git")
        .args(["rm", "-f", path_str])
        .output()
        .map_err(|e| GraftError::Git(format!("failed to run git rm: {e}")))?;

    if !rm_output.status.success() {
        let stderr = String::from_utf8_lossy(&rm_output.stderr);
        return Err(GraftError::Git(format!("git rm failed: {stderr}")));
    }

    Ok(())
}

/// Check if a path is a git submodule.
pub fn is_submodule(path: impl AsRef<Path>) -> bool {
    let path = path.as_ref();
    let Some(path_str) = path.to_str() else {
        return false;
    };

    let Ok(output) = Command::new("git")
        .args(["submodule", "status", path_str])
        .output()
    else {
        return false;
    };

    // If git submodule status returns successfully and has output, it's a submodule
    output.status.success() && !output.stdout.is_empty()
}

#[derive(Debug)]
pub struct AddResult {
    pub name: String,
    pub source: String,
    pub git_ref: String,
}

#[derive(Debug)]
#[allow(clippy::struct_excessive_bools)]
pub struct RemoveResult {
    pub name: String,
    pub removed_from_config: bool,
    pub removed_from_lock: bool,
    pub removed_submodule: bool,
    pub kept_files: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_add_dependency_to_empty_config() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "apiVersion: graft/v0").unwrap();
        file.flush().unwrap();

        let result = add_dependency_to_config(
            file.path(),
            "test-dep",
            "https://example.com/repo.git",
            "main",
        );
        assert!(result.is_ok());

        let content = fs::read_to_string(file.path()).unwrap();
        assert!(content.contains("deps:"));
        assert!(content.contains("test-dep"));
        assert!(content.contains("https://example.com/repo.git#main"));
    }

    #[test]
    fn test_add_dependency_duplicate_fails() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            "apiVersion: graft/v0\ndeps:\n  test-dep: \"https://example.com/repo.git#main\""
        )
        .unwrap();
        file.flush().unwrap();

        let result = add_dependency_to_config(
            file.path(),
            "test-dep",
            "https://example.com/repo.git",
            "v1.0.0",
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_remove_dependency_from_config() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            "apiVersion: graft/v0\ndeps:\n  test-dep: \"https://example.com/repo.git#main\""
        )
        .unwrap();
        file.flush().unwrap();

        let result = remove_dependency_from_config(file.path(), "test-dep");
        assert!(result.is_ok());

        let content = fs::read_to_string(file.path()).unwrap();
        assert!(!content.contains("test-dep"));
    }

    #[test]
    fn test_remove_nonexistent_dependency_fails() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "apiVersion: graft/v0\ndeps: {{}}").unwrap();
        file.flush().unwrap();

        let result = remove_dependency_from_config(file.path(), "nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}
