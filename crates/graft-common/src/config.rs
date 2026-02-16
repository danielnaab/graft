//! Shared graft.yaml parsing utilities.
//!
//! This module provides common parsing helpers for graft.yaml files,
//! focusing on commands and state queries. Both grove and graft tools
//! can use these helpers while maintaining their own domain types.

use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// A command definition from graft.yaml.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandDef {
    pub run: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
}

/// A state query definition from graft.yaml.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateQueryDef {
    pub run: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub deterministic: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
}

/// Parse commands section from graft.yaml.
///
/// Returns a `HashMap` of command name to command definition.
/// Returns an empty `HashMap` if the file doesn't exist or has no commands section.
pub fn parse_commands(
    graft_yaml_path: impl AsRef<Path>,
) -> Result<HashMap<String, CommandDef>, String> {
    let path = graft_yaml_path.as_ref();

    // Return empty if file doesn't exist
    if !path.exists() {
        return Ok(HashMap::new());
    }

    let content =
        fs::read_to_string(path).map_err(|e| format!("Failed to read graft.yaml: {e}"))?;

    let yaml: Value =
        serde_yaml::from_str(&content).map_err(|e| format!("Failed to parse graft.yaml: {e}"))?;

    let mut commands = HashMap::new();

    // Look for "commands:" section
    if let Some(commands_section) = yaml.get("commands") {
        if let Some(commands_map) = commands_section.as_mapping() {
            for (name, config) in commands_map {
                if let Some(name_str) = name.as_str() {
                    match parse_command(name_str, config) {
                        Ok(cmd) => {
                            commands.insert(name_str.to_string(), cmd);
                        }
                        Err(e) => {
                            eprintln!("Warning: Failed to parse command '{name_str}': {e}");
                        }
                    }
                }
            }
        }
    }

    Ok(commands)
}

/// Parse a single command from YAML config.
fn parse_command(name: &str, config: &Value) -> Result<CommandDef, String> {
    // Get run field (required)
    let run = config
        .get("run")
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("Command '{name}' missing 'run' field"))?
        .to_string();

    // Get optional fields
    let description = config
        .get("description")
        .and_then(|d| d.as_str())
        .map(std::string::ToString::to_string);

    let working_dir = config
        .get("working_dir")
        .and_then(|w| w.as_str())
        .map(std::string::ToString::to_string);

    let env = config
        .get("env")
        .and_then(|e| e.as_mapping())
        .map(|env_map| {
            let mut env = HashMap::new();
            for (k, v) in env_map {
                if let (Some(key), Some(val)) = (k.as_str(), v.as_str()) {
                    env.insert(key.to_string(), val.to_string());
                }
            }
            env
        });

    Ok(CommandDef {
        run,
        description,
        working_dir,
        env,
    })
}

/// Parse state queries section from graft.yaml.
///
/// Returns a `HashMap` of query name to query definition.
/// Returns an empty `HashMap` if the file doesn't exist or has no state section.
pub fn parse_state_queries(
    graft_yaml_path: impl AsRef<Path>,
) -> Result<HashMap<String, StateQueryDef>, String> {
    let path = graft_yaml_path.as_ref();

    // Return empty if file doesn't exist
    if !path.exists() {
        return Ok(HashMap::new());
    }

    let content =
        fs::read_to_string(path).map_err(|e| format!("Failed to read graft.yaml: {e}"))?;

    let yaml: Value =
        serde_yaml::from_str(&content).map_err(|e| format!("Failed to parse graft.yaml: {e}"))?;

    let mut queries = HashMap::new();

    // Look for "state:" section
    if let Some(state_section) = yaml.get("state") {
        if let Some(state_map) = state_section.as_mapping() {
            for (name, config) in state_map {
                if let Some(name_str) = name.as_str() {
                    match parse_state_query(name_str, config) {
                        Ok(query) => {
                            queries.insert(name_str.to_string(), query);
                        }
                        Err(e) => {
                            eprintln!("Warning: Failed to parse state query '{name_str}': {e}");
                        }
                    }
                }
            }
        }
    }

    Ok(queries)
}

/// Parse a single state query from YAML config.
fn parse_state_query(name: &str, config: &Value) -> Result<StateQueryDef, String> {
    // Get run command (required)
    let run = config
        .get("run")
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("State query '{name}' missing 'run' field"))?
        .to_string();

    // Get cache config
    let deterministic = config
        .get("cache")
        .and_then(|c| c.get("deterministic"))
        .and_then(serde_yaml::Value::as_bool)
        .unwrap_or(true); // Default to deterministic

    // Get timeout
    let timeout = config.get("timeout").and_then(serde_yaml::Value::as_u64);

    // Get description (optional)
    let description = config
        .get("description")
        .and_then(|d| d.as_str())
        .map(std::string::ToString::to_string);

    Ok(StateQueryDef {
        run,
        description,
        deterministic,
        timeout,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn parse_commands_handles_missing_file() {
        let result = parse_commands("/nonexistent/graft.yaml").unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn parse_commands_from_yaml() {
        let yaml_content = r#"
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

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let commands = parse_commands(temp_file.path()).unwrap();

        assert_eq!(commands.len(), 2);

        let test_cmd = commands.get("test").unwrap();
        assert_eq!(test_cmd.run, "cargo test");
        assert_eq!(test_cmd.description.as_deref(), Some("Run tests"));
        assert!(test_cmd.working_dir.is_none());
        assert!(test_cmd.env.is_none());

        let build_cmd = commands.get("build").unwrap();
        assert_eq!(build_cmd.run, "cargo build --release");
        assert_eq!(build_cmd.working_dir.as_deref(), Some("."));
        assert!(build_cmd.env.is_some());
        let env = build_cmd.env.as_ref().unwrap();
        assert_eq!(env.get("RUST_LOG"), Some(&"info".to_string()));
    }

    #[test]
    fn parse_commands_handles_no_commands_section() {
        let yaml_content = r#"
apiVersion: graft/v0
state:
  coverage:
    run: "pytest --cov"
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let commands = parse_commands(temp_file.path()).unwrap();
        assert_eq!(commands.len(), 0);
    }

    #[test]
    fn parse_state_queries_handles_missing_file() {
        let result = parse_state_queries("/nonexistent/graft.yaml").unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn parse_state_queries_from_yaml() {
        let yaml_content = r#"
apiVersion: graft/v0
state:
  coverage:
    run: "pytest --cov"
    description: "Run coverage"
    cache:
      deterministic: true
    timeout: 60

  tasks:
    run: "task-tracker status"
    cache:
      deterministic: false
    timeout: 30
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let queries = parse_state_queries(temp_file.path()).unwrap();

        assert_eq!(queries.len(), 2);

        let coverage = queries.get("coverage").unwrap();
        assert_eq!(coverage.run, "pytest --cov");
        assert_eq!(coverage.description.as_deref(), Some("Run coverage"));
        assert_eq!(coverage.deterministic, true);
        assert_eq!(coverage.timeout, Some(60));

        let tasks = queries.get("tasks").unwrap();
        assert_eq!(tasks.run, "task-tracker status");
        assert!(tasks.description.is_none());
        assert_eq!(tasks.deterministic, false);
        assert_eq!(tasks.timeout, Some(30));
    }

    #[test]
    fn parse_state_queries_handles_no_state_section() {
        let yaml_content = r#"
apiVersion: graft/v0
commands:
  test:
    run: "pytest"
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let queries = parse_state_queries(temp_file.path()).unwrap();
        assert_eq!(queries.len(), 0);
    }

    #[test]
    fn parse_state_queries_defaults_deterministic_to_true() {
        let yaml_content = r#"
state:
  simple:
    run: "echo test"
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let queries = parse_state_queries(temp_file.path()).unwrap();
        assert_eq!(queries.len(), 1);

        let simple = queries.get("simple").unwrap();
        assert_eq!(simple.deterministic, true); // Default
        assert_eq!(simple.timeout, None);
    }

    #[test]
    fn parse_state_queries_handles_missing_cache_field() {
        let yaml_content = r#"
state:
  simple:
    run: "echo test"
    timeout: 10
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let queries = parse_state_queries(temp_file.path()).unwrap();
        let simple = queries.get("simple").unwrap();
        assert_eq!(simple.deterministic, true); // Default when cache field missing
        assert_eq!(simple.timeout, Some(10));
    }
}
