//! YAML configuration loading adapter.

use grove_core::{ConfigLoader, CoreError, Result, WorkspaceConfig};
use std::fs;

/// YAML-based configuration loader.
#[derive(Debug)]
pub struct YamlConfigLoader;

impl YamlConfigLoader {
    pub fn new() -> Self {
        Self
    }
}

impl Default for YamlConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigLoader for YamlConfigLoader {
    fn load_workspace(&self, config_path: &str) -> Result<WorkspaceConfig> {
        let contents = fs::read_to_string(config_path).map_err(|e| CoreError::InvalidConfig {
            details: format!("Failed to read config file '{config_path}': {e}"),
        })?;

        serde_yml::from_str(&contents).map_err(|e| CoreError::InvalidConfig {
            details: format!("Failed to parse config file '{config_path}': {e}"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn loads_valid_config() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
name: test-workspace
repositories:
  - path: /tmp/repo1
    tags: [rust, cli]
  - path: /tmp/repo2
    tags: []
"#
        )
        .unwrap();

        let loader = YamlConfigLoader::new();
        let config = loader
            .load_workspace(file.path().to_str().unwrap())
            .unwrap();

        assert_eq!(config.name.as_str(), "test-workspace");
        assert_eq!(config.repositories.len(), 2);
    }

    #[test]
    fn fails_on_missing_file() {
        let loader = YamlConfigLoader::new();
        let result = loader.load_workspace("/nonexistent/config.yaml");
        assert!(result.is_err());
    }

    #[test]
    fn fails_on_invalid_yaml() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "not: valid: yaml: structure").unwrap();

        let loader = YamlConfigLoader::new();
        let result = loader.load_workspace(file.path().to_str().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn rejects_empty_workspace_name() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "name: ''").unwrap();
        writeln!(file, "repositories: []").unwrap();

        let loader = YamlConfigLoader::new();
        let result = loader.load_workspace(file.path().to_str().unwrap());

        assert!(
            result.is_err(),
            "Should reject empty workspace name, got: {:?}",
            result
        );

        // Verify it's the right kind of error
        if let Err(e) = result {
            let err_msg = e.to_string();
            assert!(
                err_msg.contains("Empty") || err_msg.contains("empty"),
                "Error should mention empty name, got: {err_msg}"
            );
        }
    }

    #[test]
    fn rejects_whitespace_only_workspace_name() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "name: '   '").unwrap();
        writeln!(file, "repositories: []").unwrap();

        let loader = YamlConfigLoader::new();
        let result = loader.load_workspace(file.path().to_str().unwrap());

        assert!(
            result.is_err(),
            "Should reject whitespace-only workspace name"
        );
    }

    #[test]
    fn handles_duplicate_repository_paths() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "name: test-workspace").unwrap();
        writeln!(file, "repositories:").unwrap();
        writeln!(file, "  - path: /tmp/repo1").unwrap();
        writeln!(file, "    tags: [tag1]").unwrap();
        writeln!(file, "  - path: /tmp/repo1").unwrap();
        writeln!(file, "    tags: [tag2]").unwrap();

        let loader = YamlConfigLoader::new();
        let result = loader.load_workspace(file.path().to_str().unwrap());

        // Current implementation: Accepts duplicates (both entries are loaded)
        // This test documents the current behavior - future: could warn or error
        assert!(result.is_ok(), "Currently accepts duplicate paths");

        let config = result.unwrap();
        assert_eq!(
            config.repositories.len(),
            2,
            "Should load both entries even if paths are duplicates"
        );
    }

    #[test]
    fn handles_path_with_spaces() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "name: test-workspace").unwrap();
        writeln!(file, "repositories:").unwrap();
        writeln!(file, "  - path: '/tmp/path with spaces'").unwrap();
        writeln!(file, "    tags: [test]").unwrap();

        let loader = YamlConfigLoader::new();
        let result = loader.load_workspace(file.path().to_str().unwrap());

        assert!(result.is_ok(), "Should handle paths with spaces");

        let config = result.unwrap();
        assert_eq!(config.repositories.len(), 1);

        let repo_path = &config.repositories[0].path;
        assert!(
            repo_path.as_path().to_string_lossy().contains("path with spaces"),
            "Path should preserve spaces"
        );
    }

    #[test]
    fn handles_tilde_expansion_in_path() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "name: test-workspace").unwrap();
        writeln!(file, "repositories:").unwrap();
        writeln!(file, "  - path: '~/src/project'").unwrap();
        writeln!(file, "    tags: [test]").unwrap();

        let loader = YamlConfigLoader::new();
        let result = loader.load_workspace(file.path().to_str().unwrap());

        assert!(result.is_ok(), "Should handle tilde expansion");

        let config = result.unwrap();
        let repo_path = &config.repositories[0].path;

        // Tilde should be expanded
        assert!(
            !repo_path.as_path().to_string_lossy().starts_with("~"),
            "Tilde should be expanded, got: {}",
            repo_path.as_path().display()
        );
    }

    #[test]
    fn handles_environment_variable_in_path() {
        // Set a test env var
        std::env::set_var("GROVE_TEST_PATH", "/tmp/test");

        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "name: test-workspace").unwrap();
        writeln!(file, "repositories:").unwrap();
        writeln!(file, "  - path: '$GROVE_TEST_PATH/repo'").unwrap();
        writeln!(file, "    tags: [test]").unwrap();

        let loader = YamlConfigLoader::new();
        let result = loader.load_workspace(file.path().to_str().unwrap());

        assert!(result.is_ok(), "Should handle environment variables");

        let config = result.unwrap();
        let repo_path = &config.repositories[0].path;

        // Env var should be expanded
        assert!(
            repo_path
                .as_path()
                .to_string_lossy()
                .contains("/tmp/test/repo"),
            "Env var should be expanded, got: {}",
            repo_path.as_path().display()
        );

        // Cleanup
        std::env::remove_var("GROVE_TEST_PATH");
    }

    #[test]
    fn rejects_undefined_environment_variable() {
        // Ensure the var is not set
        std::env::remove_var("GROVE_UNDEFINED_VAR");

        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "name: test-workspace").unwrap();
        writeln!(file, "repositories:").unwrap();
        writeln!(file, "  - path: '$GROVE_UNDEFINED_VAR/repo'").unwrap();
        writeln!(file, "    tags: [test]").unwrap();

        let loader = YamlConfigLoader::new();
        let result = loader.load_workspace(file.path().to_str().unwrap());

        // shellexpand errors on undefined variables in LookupError mode
        assert!(
            result.is_err(),
            "Should error on undefined env vars (shellexpand behavior)"
        );

        // Verify error mentions the undefined variable
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("GROVE_UNDEFINED_VAR") || err_msg.contains("environment"),
            "Error should mention undefined variable, got: {err_msg}"
        );
    }

    #[test]
    fn handles_empty_tags_list() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "name: test-workspace").unwrap();
        writeln!(file, "repositories:").unwrap();
        writeln!(file, "  - path: /tmp/repo").unwrap();
        writeln!(file, "    tags: []").unwrap();

        let loader = YamlConfigLoader::new();
        let result = loader.load_workspace(file.path().to_str().unwrap());

        assert!(result.is_ok(), "Should handle empty tags list");

        let config = result.unwrap();
        assert_eq!(config.repositories[0].tags.len(), 0);
    }

    #[test]
    fn handles_missing_tags_field() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "name: test-workspace").unwrap();
        writeln!(file, "repositories:").unwrap();
        writeln!(file, "  - path: /tmp/repo").unwrap();

        let loader = YamlConfigLoader::new();
        let result = loader.load_workspace(file.path().to_str().unwrap());

        // Should error because tags field is required in the struct
        // (unless it has #[serde(default)])
        // Let's test the actual behavior
        if result.is_err() {
            // Tags field is required
            let err_msg = result.unwrap_err().to_string();
            assert!(
                err_msg.contains("missing") || err_msg.contains("tags"),
                "Error should mention missing tags field"
            );
        } else {
            // Tags field has a default
            let config = result.unwrap();
            assert_eq!(
                config.repositories[0].tags.len(),
                0,
                "Default should be empty tags"
            );
        }
    }
}
