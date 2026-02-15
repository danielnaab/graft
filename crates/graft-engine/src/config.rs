//! Configuration parsing for graft.yaml files.

use graft_core::{
    Change, Command, DependencySpec, GitRef, GitUrl, GraftConfig, GraftError, Metadata, Result,
};
use serde_yaml::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Parse a graft.yaml file from a path.
pub fn parse_graft_yaml(path: impl AsRef<Path>) -> Result<GraftConfig> {
    let path = path.as_ref();

    // Check file exists
    if !path.exists() {
        return Err(GraftError::ConfigFileNotFound {
            path: path.display().to_string(),
        });
    }

    // Read file
    let content = fs::read_to_string(path).map_err(|e| GraftError::ConfigParse {
        path: path.display().to_string(),
        reason: format!("failed to read file: {e}"),
    })?;

    parse_graft_yaml_str(&content, &path.display().to_string())
}

/// Parse graft.yaml from a string.
#[allow(clippy::too_many_lines)]
pub fn parse_graft_yaml_str(content: &str, path: &str) -> Result<GraftConfig> {
    // Parse YAML
    let data: Value = serde_yaml::from_str(content).map_err(|e| GraftError::ConfigParse {
        path: path.to_string(),
        reason: format!("invalid YAML syntax: {e}"),
    })?;

    // Validate structure
    let obj = data
        .as_mapping()
        .ok_or_else(|| GraftError::ConfigValidation {
            path: path.to_string(),
            field: "root".to_string(),
            reason: "configuration must be a YAML mapping/dict".to_string(),
        })?;

    // Extract apiVersion (required)
    let api_version = obj
        .get(Value::String("apiVersion".to_string()))
        .ok_or_else(|| GraftError::ConfigValidation {
            path: path.to_string(),
            field: "apiVersion".to_string(),
            reason: "missing required field".to_string(),
        })?
        .as_str()
        .ok_or_else(|| GraftError::ConfigValidation {
            path: path.to_string(),
            field: "apiVersion".to_string(),
            reason: "must be a string".to_string(),
        })?;

    // Create config
    let mut config = GraftConfig::new(api_version)?;

    // Parse metadata (optional)
    if let Some(metadata_value) = obj.get(Value::String("metadata".to_string())) {
        let metadata: Metadata = serde_yaml::from_value(metadata_value.clone()).map_err(|e| {
            GraftError::ConfigValidation {
                path: path.to_string(),
                field: "metadata".to_string(),
                reason: format!("invalid metadata: {e}"),
            }
        })?;
        config.metadata = Some(metadata);
    }

    // Parse commands (optional)
    if let Some(commands_value) = obj.get(Value::String("commands".to_string())) {
        let commands_map =
            commands_value
                .as_mapping()
                .ok_or_else(|| GraftError::ConfigValidation {
                    path: path.to_string(),
                    field: "commands".to_string(),
                    reason: "must be a mapping/dict of command_name: {...}".to_string(),
                })?;

        for (cmd_name, cmd_data) in commands_map {
            let name = cmd_name
                .as_str()
                .ok_or_else(|| GraftError::ConfigValidation {
                    path: path.to_string(),
                    field: "commands".to_string(),
                    reason: "command name must be a string".to_string(),
                })?;

            let cmd_obj = cmd_data
                .as_mapping()
                .ok_or_else(|| GraftError::ConfigValidation {
                    path: path.to_string(),
                    field: format!("commands.{name}"),
                    reason: "command must be a mapping/dict with 'run' field".to_string(),
                })?;

            let run = cmd_obj
                .get(Value::String("run".to_string()))
                .ok_or_else(|| GraftError::ConfigValidation {
                    path: path.to_string(),
                    field: format!("commands.{name}"),
                    reason: "command must have 'run' field".to_string(),
                })?
                .as_str()
                .ok_or_else(|| GraftError::ConfigValidation {
                    path: path.to_string(),
                    field: format!("commands.{name}.run"),
                    reason: "'run' must be a string".to_string(),
                })?;

            let mut command = Command::new(name, run)?;

            if let Some(desc_value) = cmd_obj.get(Value::String("description".to_string())) {
                if let Some(desc) = desc_value.as_str() {
                    command.description = Some(desc.to_string());
                }
            }

            if let Some(wd_value) = cmd_obj.get(Value::String("working_dir".to_string())) {
                if let Some(wd) = wd_value.as_str() {
                    command.working_dir = Some(wd.to_string());
                }
            }

            if let Some(env_value) = cmd_obj.get(Value::String("env".to_string())) {
                if let Some(env_map) = env_value.as_mapping() {
                    let mut env = HashMap::new();
                    for (k, v) in env_map {
                        if let (Some(key), Some(val)) = (k.as_str(), v.as_str()) {
                            env.insert(key.to_string(), val.to_string());
                        }
                    }
                    command.env = Some(env);
                }
            }

            config.commands.insert(name.to_string(), command);
        }
    }

    // Parse changes (optional)
    if let Some(changes_value) = obj.get(Value::String("changes".to_string())) {
        let changes_map =
            changes_value
                .as_mapping()
                .ok_or_else(|| GraftError::ConfigValidation {
                    path: path.to_string(),
                    field: "changes".to_string(),
                    reason: "must be a mapping/dict of ref: {...}".to_string(),
                })?;

        for (ref_name_value, change_data) in changes_map {
            let ref_name = ref_name_value
                .as_str()
                .ok_or_else(|| GraftError::ConfigValidation {
                    path: path.to_string(),
                    field: "changes".to_string(),
                    reason: "change ref must be a string".to_string(),
                })?;

            let mut change = Change::new(ref_name)?;

            // Allow null/empty changes
            if let Some(change_obj) = change_data.as_mapping() {
                if let Some(type_value) = change_obj.get(Value::String("type".to_string())) {
                    if let Some(type_str) = type_value.as_str() {
                        change.change_type = Some(type_str.to_string());
                    }
                }

                if let Some(desc_value) = change_obj.get(Value::String("description".to_string())) {
                    if let Some(desc) = desc_value.as_str() {
                        change.description = Some(desc.to_string());
                    }
                }

                if let Some(migration_value) =
                    change_obj.get(Value::String("migration".to_string()))
                {
                    if let Some(migration) = migration_value.as_str() {
                        change.migration = Some(migration.to_string());
                    }
                }

                if let Some(verify_value) = change_obj.get(Value::String("verify".to_string())) {
                    if let Some(verify) = verify_value.as_str() {
                        change.verify = Some(verify.to_string());
                    }
                }

                // Collect any extra fields into metadata
                for (k, v) in change_obj {
                    if let Some(key) = k.as_str() {
                        if !matches!(key, "type" | "description" | "migration" | "verify") {
                            change.metadata.insert(key.to_string(), v.clone());
                        }
                    }
                }
            }

            config.changes.insert(ref_name.to_string(), change);
        }
    }

    // Parse dependencies (support both "deps" and "dependencies" formats)
    // First try "deps" (short format: "deps: { name: url#ref }")
    if let Some(deps_value) = obj.get(Value::String("deps".to_string())) {
        let deps_map = deps_value
            .as_mapping()
            .ok_or_else(|| GraftError::ConfigValidation {
                path: path.to_string(),
                field: "deps".to_string(),
                reason: "must be a mapping/dict of dependency_name: url#ref".to_string(),
            })?;

        for (name_value, url_ref_value) in deps_map {
            let name = name_value
                .as_str()
                .ok_or_else(|| GraftError::ConfigValidation {
                    path: path.to_string(),
                    field: "deps".to_string(),
                    reason: "dependency name must be a string".to_string(),
                })?;

            let url_ref = url_ref_value
                .as_str()
                .ok_or_else(|| GraftError::ConfigValidation {
                    path: path.to_string(),
                    field: format!("deps.{name}"),
                    reason: "dependency must be a string in format 'url#ref'".to_string(),
                })?;

            // Parse URL#ref format
            let parts: Vec<&str> = url_ref.rsplitn(2, '#').collect();
            if parts.len() != 2 {
                return Err(GraftError::ConfigValidation {
                    path: path.to_string(),
                    field: format!("deps.{name}"),
                    reason: format!("must use format 'url#ref', got: {url_ref}"),
                });
            }

            let (git_ref, git_url) = (parts[0], parts[1]);

            let spec = DependencySpec::new(name, GitUrl::new(git_url)?, GitRef::new(git_ref)?)?;

            config.dependencies.insert(name.to_string(), spec);
        }
    }

    // Also support new "dependencies" format from spec
    if let Some(dependencies_value) = obj.get(Value::String("dependencies".to_string())) {
        let deps_map =
            dependencies_value
                .as_mapping()
                .ok_or_else(|| GraftError::ConfigValidation {
                    path: path.to_string(),
                    field: "dependencies".to_string(),
                    reason: "must be a mapping/dict".to_string(),
                })?;

        for (name_value, dep_data) in deps_map {
            let name = name_value
                .as_str()
                .ok_or_else(|| GraftError::ConfigValidation {
                    path: path.to_string(),
                    field: "dependencies".to_string(),
                    reason: "dependency name must be a string".to_string(),
                })?;

            let (git_url, git_ref) = if let Some(url_ref_str) = dep_data.as_str() {
                // Simple string format: "url#ref"
                let parts: Vec<&str> = url_ref_str.rsplitn(2, '#').collect();
                if parts.len() != 2 {
                    return Err(GraftError::ConfigValidation {
                        path: path.to_string(),
                        field: format!("dependencies.{name}"),
                        reason: format!("must use format 'url#ref', got: {url_ref_str}"),
                    });
                }
                (parts[1], parts[0])
            } else if let Some(dep_obj) = dep_data.as_mapping() {
                // Object format with source and ref
                let source = dep_obj
                    .get(Value::String("source".to_string()))
                    .ok_or_else(|| GraftError::ConfigValidation {
                        path: path.to_string(),
                        field: format!("dependencies.{name}"),
                        reason: "dependency must have 'source' field".to_string(),
                    })?
                    .as_str()
                    .ok_or_else(|| GraftError::ConfigValidation {
                        path: path.to_string(),
                        field: format!("dependencies.{name}.source"),
                        reason: "'source' must be a string".to_string(),
                    })?;

                let ref_str = dep_obj
                    .get(Value::String("ref".to_string()))
                    .and_then(|v| v.as_str())
                    .unwrap_or("main");

                (source, ref_str)
            } else {
                return Err(GraftError::ConfigValidation {
                    path: path.to_string(),
                    field: format!("dependencies.{name}"),
                    reason: "dependency must be string or object".to_string(),
                });
            };

            let spec = DependencySpec::new(name, GitUrl::new(git_url)?, GitRef::new(git_ref)?)?;

            config.dependencies.insert(name.to_string(), spec);
        }
    }

    // Validate configuration
    config.validate()?;

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_config() {
        let yaml = r#"
apiVersion: graft/v0
"#;
        let config = parse_graft_yaml_str(yaml, "test.yaml").unwrap();
        assert_eq!(config.api_version, "graft/v0");
        assert!(config.dependencies.is_empty());
        assert!(config.commands.is_empty());
        assert!(config.changes.is_empty());
    }

    #[test]
    fn parses_deps_format() {
        let yaml = r#"
apiVersion: graft/v0
deps:
  meta-kb: "https://github.com/user/meta-kb.git#main"
  rust-starter: "git@github.com:user/rust-starter.git#v1.0.0"
"#;
        let config = parse_graft_yaml_str(yaml, "test.yaml").unwrap();
        assert_eq!(config.dependencies.len(), 2);

        let meta_kb = config.get_dependency("meta-kb").unwrap();
        assert_eq!(
            meta_kb.git_url.as_str(),
            "https://github.com/user/meta-kb.git"
        );
        assert_eq!(meta_kb.git_ref.as_str(), "main");

        let rust_starter = config.get_dependency("rust-starter").unwrap();
        assert_eq!(
            rust_starter.git_url.as_str(),
            "ssh://git@github.com/user/rust-starter.git"
        );
        assert_eq!(rust_starter.git_ref.as_str(), "v1.0.0");
    }

    #[test]
    fn parses_commands() {
        let yaml = r#"
apiVersion: graft/v0
commands:
  test:
    run: "cargo test"
    description: "Run tests"
  build:
    run: "cargo build --release"
    working_dir: "."
    env:
      RUST_LOG: "info"
"#;
        let config = parse_graft_yaml_str(yaml, "test.yaml").unwrap();
        assert_eq!(config.commands.len(), 2);

        let test_cmd = config.get_command("test").unwrap();
        assert_eq!(test_cmd.run, "cargo test");
        assert_eq!(test_cmd.description.as_deref(), Some("Run tests"));

        let build_cmd = config.get_command("build").unwrap();
        assert_eq!(build_cmd.run, "cargo build --release");
        assert_eq!(build_cmd.working_dir.as_deref(), Some("."));
        assert!(build_cmd.has_env_vars());
    }

    #[test]
    fn parses_changes() {
        let yaml = r#"
apiVersion: graft/v0
changes:
  v2.0.0:
    type: breaking
    description: "Major refactor"
    migration: migrate-v2
    verify: verify-v2
  v1.5.0:
    type: feature
    description: "Added caching"
commands:
  migrate-v2:
    run: "echo migrating"
  verify-v2:
    run: "echo verifying"
"#;
        let config = parse_graft_yaml_str(yaml, "test.yaml").unwrap();
        assert_eq!(config.changes.len(), 2);

        let v2 = config.get_change("v2.0.0").unwrap();
        assert_eq!(v2.change_type.as_deref(), Some("breaking"));
        assert_eq!(v2.description.as_deref(), Some("Major refactor"));
        assert_eq!(v2.migration.as_deref(), Some("migrate-v2"));
        assert_eq!(v2.verify.as_deref(), Some("verify-v2"));
        assert!(v2.is_breaking());
        assert!(v2.needs_migration());

        let v1_5 = config.get_change("v1.5.0").unwrap();
        assert_eq!(v1_5.change_type.as_deref(), Some("feature"));
        assert!(!v1_5.is_breaking());
    }

    #[test]
    fn validates_migration_commands_exist() {
        let yaml = r#"
apiVersion: graft/v0
changes:
  v1.0.0:
    migration: missing-command
"#;
        let result = parse_graft_yaml_str(yaml, "test.yaml");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, GraftError::ConfigValidation { .. }));
    }

    #[test]
    fn rejects_empty_api_version() {
        let yaml = r#"
deps:
  test: "url#ref"
"#;
        let result = parse_graft_yaml_str(yaml, "test.yaml");
        assert!(result.is_err());
    }

    #[test]
    fn rejects_invalid_api_version() {
        let yaml = r#"
apiVersion: v1
"#;
        let result = parse_graft_yaml_str(yaml, "test.yaml");
        assert!(result.is_err());
    }

    #[test]
    fn rejects_command_with_colon_in_name() {
        let yaml = r#"
apiVersion: graft/v0
commands:
  test:unit:
    run: "npm test"
"#;
        let result = parse_graft_yaml_str(yaml, "test.yaml");
        assert!(result.is_err());
    }
}
