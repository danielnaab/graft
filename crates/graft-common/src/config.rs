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

/// Argument type for a command argument definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArgType {
    String,
    Choice,
    Flag,
}

/// A single argument definition in a command's `args` schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArgDef {
    pub name: String,
    #[serde(rename = "type")]
    pub arg_type: ArgType,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub options: Option<Vec<String>>,
    #[serde(default)]
    pub positional: bool,
}

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<ArgDef>>,
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

/// Read a YAML file, returning an empty string if the file doesn't exist.
fn read_yaml_file(path: impl AsRef<Path>) -> Result<String, String> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(String::new());
    }
    fs::read_to_string(path).map_err(|e| format!("Failed to read graft.yaml: {e}"))
}

/// Parse commands section from graft.yaml.
///
/// Returns a `HashMap` of command name to command definition.
/// Returns an empty `HashMap` if the file doesn't exist or has no commands section.
pub fn parse_commands(
    graft_yaml_path: impl AsRef<Path>,
) -> Result<HashMap<String, CommandDef>, String> {
    let content = read_yaml_file(graft_yaml_path)?;
    parse_commands_from_str(&content)
}

/// Parse commands from a graft.yaml content string.
///
/// Like [`parse_commands`] but operates on an already-read string,
/// avoiding a redundant file read when multiple sections are needed.
pub fn parse_commands_from_str(content: &str) -> Result<HashMap<String, CommandDef>, String> {
    if content.is_empty() {
        return Ok(HashMap::new());
    }

    let yaml: Value =
        serde_yaml::from_str(content).map_err(|e| format!("Failed to parse graft.yaml: {e}"))?;

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

    let args = config
        .get("args")
        .and_then(|a| a.as_sequence())
        .map(|args_seq| {
            let mut parsed_args = Vec::new();
            let mut seen_names = std::collections::HashSet::new();
            for (i, arg_val) in args_seq.iter().enumerate() {
                match serde_yaml::from_value::<ArgDef>(arg_val.clone()) {
                    Ok(arg_def) => {
                        // Validate: choice args must have non-empty options
                        if arg_def.arg_type == ArgType::Choice {
                            let has_options = arg_def
                                .options
                                .as_ref()
                                .is_some_and(|opts| !opts.is_empty());
                            if !has_options {
                                eprintln!(
                                    "Warning: Choice arg '{}' in command '{name}' has no options, skipping",
                                    arg_def.name
                                );
                                continue;
                            }
                        }
                        // Validate: flag args cannot be positional
                        if arg_def.arg_type == ArgType::Flag && arg_def.positional {
                            eprintln!(
                                "Warning: Flag arg '{}' in command '{name}' cannot be positional, skipping",
                                arg_def.name
                            );
                            continue;
                        }
                        // Validate: no duplicate names
                        if !seen_names.insert(arg_def.name.clone()) {
                            eprintln!(
                                "Warning: Duplicate arg name '{}' in command '{name}', skipping",
                                arg_def.name
                            );
                            continue;
                        }
                        // Validate: choice default must exist in options
                        if arg_def.arg_type == ArgType::Choice {
                            if let (Some(default), Some(options)) =
                                (&arg_def.default, &arg_def.options)
                            {
                                if !options.contains(default) {
                                    eprintln!(
                                        "Warning: Default '{}' for choice arg '{}' in command '{name}' \
                                         not in options, ignoring default",
                                        default, arg_def.name
                                    );
                                }
                            }
                        }
                        parsed_args.push(arg_def);
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to parse arg {i} in command '{name}': {e}");
                    }
                }
            }
            parsed_args
        });

    Ok(CommandDef {
        run,
        description,
        working_dir,
        env,
        args,
    })
}

/// Parse dependency names from graft.yaml.
///
/// Returns the keys from the `dependencies` (or `deps`) section.
/// Returns an empty `Vec` if the file doesn't exist or has no dependency section.
pub fn parse_dependency_names(graft_yaml_path: impl AsRef<Path>) -> Result<Vec<String>, String> {
    let content = read_yaml_file(graft_yaml_path)?;
    parse_dependency_names_from_str(&content)
}

/// Parse dependency names from a graft.yaml content string.
///
/// Like [`parse_dependency_names`] but operates on an already-read string,
/// avoiding a redundant file read when multiple sections are needed.
pub fn parse_dependency_names_from_str(content: &str) -> Result<Vec<String>, String> {
    if content.is_empty() {
        return Ok(Vec::new());
    }

    let yaml: Value =
        serde_yaml::from_str(content).map_err(|e| format!("Failed to parse graft.yaml: {e}"))?;

    let mut names = Vec::new();

    let deps_section = yaml.get("dependencies").or_else(|| yaml.get("deps"));

    if let Some(deps) = deps_section {
        if let Some(mapping) = deps.as_mapping() {
            for key in mapping.keys() {
                if let Some(name) = key.as_str() {
                    names.push(name.to_string());
                }
            }
        }
    }

    names.sort();
    Ok(names)
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
    fn parse_commands_with_no_args_backward_compat() {
        let yaml_content = r#"
commands:
  test:
    run: "cargo test"
    description: "Run tests"
"#;
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let commands = parse_commands(temp_file.path()).unwrap();
        let test_cmd = commands.get("test").unwrap();
        assert!(test_cmd.args.is_none());
    }

    #[test]
    fn parse_commands_with_args_schema() {
        let yaml_content = r#"
commands:
  deploy:
    run: "./deploy.sh"
    description: "Deploy the application"
    args:
      - name: environment
        type: choice
        options: [staging, production]
        required: true
        description: "Target environment"
      - name: tag
        type: string
        default: latest
        description: "Version tag to deploy"
      - name: verbose
        type: flag
        description: "Enable verbose output"
      - name: target
        type: string
        positional: true
        required: true
"#;
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let commands = parse_commands(temp_file.path()).unwrap();
        let deploy = commands.get("deploy").unwrap();

        let args = deploy.args.as_ref().unwrap();
        assert_eq!(args.len(), 4);

        // Choice arg
        assert_eq!(args[0].name, "environment");
        assert_eq!(args[0].arg_type, ArgType::Choice);
        assert!(args[0].required);
        assert_eq!(
            args[0].options.as_ref().unwrap(),
            &vec!["staging".to_string(), "production".to_string()]
        );
        assert_eq!(args[0].description.as_deref(), Some("Target environment"));

        // String arg with default
        assert_eq!(args[1].name, "tag");
        assert_eq!(args[1].arg_type, ArgType::String);
        assert!(!args[1].required);
        assert_eq!(args[1].default.as_deref(), Some("latest"));

        // Flag arg
        assert_eq!(args[2].name, "verbose");
        assert_eq!(args[2].arg_type, ArgType::Flag);
        assert!(!args[2].required);
        assert!(!args[2].positional);

        // Positional arg
        assert_eq!(args[3].name, "target");
        assert!(args[3].positional);
        assert!(args[3].required);
    }

    #[test]
    fn parse_commands_with_malformed_args_skips_bad_entries() {
        let yaml_content = r#"
commands:
  build:
    run: "make build"
    args:
      - name: target
        type: string
      - invalid_key_only: true
      - name: verbose
        type: flag
"#;
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let commands = parse_commands(temp_file.path()).unwrap();
        let build_cmd = commands.get("build").unwrap();

        // The malformed entry should be skipped
        let args = build_cmd.args.as_ref().unwrap();
        assert_eq!(args.len(), 2);
        assert_eq!(args[0].name, "target");
        assert_eq!(args[1].name, "verbose");
    }

    #[test]
    fn parse_commands_rejects_choice_without_options() {
        let yaml_content = r#"
commands:
  deploy:
    run: "./deploy.sh"
    args:
      - name: environment
        type: choice
      - name: tag
        type: string
"#;
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let commands = parse_commands(temp_file.path()).unwrap();
        let deploy = commands.get("deploy").unwrap();
        let args = deploy.args.as_ref().unwrap();

        // Choice without options should be skipped
        assert_eq!(args.len(), 1);
        assert_eq!(args[0].name, "tag");
    }

    #[test]
    fn parse_commands_rejects_positional_flag() {
        let yaml_content = r#"
commands:
  build:
    run: "make"
    args:
      - name: verbose
        type: flag
        positional: true
      - name: target
        type: string
"#;
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let commands = parse_commands(temp_file.path()).unwrap();
        let build = commands.get("build").unwrap();
        let args = build.args.as_ref().unwrap();

        // Positional flag should be skipped
        assert_eq!(args.len(), 1);
        assert_eq!(args[0].name, "target");
    }

    #[test]
    fn parse_commands_rejects_duplicate_arg_names() {
        let yaml_content = r#"
commands:
  test:
    run: "cargo test"
    args:
      - name: target
        type: string
      - name: target
        type: string
"#;
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let commands = parse_commands(temp_file.path()).unwrap();
        let test_cmd = commands.get("test").unwrap();
        let args = test_cmd.args.as_ref().unwrap();

        // Duplicate should be skipped
        assert_eq!(args.len(), 1);
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

    #[test]
    fn parse_notebook_graft_yaml_capture_args() {
        // Test against the real notebook graft.yaml to verify end-to-end parsing
        let notebook_path = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
            .join("src/notebook/graft.yaml");
        if !notebook_path.exists() {
            // Skip if notebook repo not available
            return;
        }
        let commands = parse_commands(&notebook_path).unwrap();
        let capture = commands
            .get("capture")
            .expect("capture command should exist");
        assert_eq!(capture.run, "uv run notecap capture");

        let args = capture
            .args
            .as_ref()
            .expect("capture should have args schema");
        assert_eq!(args.len(), 3, "Expected 3 args (section, content, raw)");

        // section: choice, positional, required
        assert_eq!(args[0].name, "section");
        assert_eq!(args[0].arg_type, ArgType::Choice);
        assert!(args[0].positional);
        assert!(args[0].required);
        assert_eq!(
            args[0].options.as_ref().unwrap(),
            &vec!["Personal".to_string(), "Work".to_string()]
        );

        // content: string, positional, required
        assert_eq!(args[1].name, "content");
        assert_eq!(args[1].arg_type, ArgType::String);
        assert!(args[1].positional);
        assert!(args[1].required);

        // raw: flag, not positional, not required
        assert_eq!(args[2].name, "raw");
        assert_eq!(args[2].arg_type, ArgType::Flag);
        assert!(!args[2].positional);
        assert!(!args[2].required);
    }

    #[test]
    fn parse_dependency_names_handles_missing_file() {
        let result = parse_dependency_names("/nonexistent/graft.yaml").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn parse_dependency_names_from_dependencies_section() {
        let yaml_content = r#"
apiVersion: graft/v0
dependencies:
  notebook: "https://github.com/user/notebook#main"
  tools: "https://github.com/user/tools#v1"
"#;
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let names = parse_dependency_names(temp_file.path()).unwrap();
        assert_eq!(names, vec!["notebook", "tools"]);
    }

    #[test]
    fn parse_dependency_names_from_deps_section() {
        let yaml_content = r#"
apiVersion: graft/v0
deps:
  alpha: "https://github.com/user/alpha#main"
  beta: "https://github.com/user/beta#main"
"#;
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let names = parse_dependency_names(temp_file.path()).unwrap();
        assert_eq!(names, vec!["alpha", "beta"]);
    }

    #[test]
    fn parse_dependency_names_prefers_dependencies_over_deps() {
        let yaml_content = r#"
apiVersion: graft/v0
dependencies:
  from_deps: "https://github.com/user/from_deps#main"
deps:
  from_short: "https://github.com/user/from_short#main"
"#;
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let names = parse_dependency_names(temp_file.path()).unwrap();
        // Should use "dependencies" since it's checked first
        assert_eq!(names, vec!["from_deps"]);
    }

    #[test]
    fn parse_dependency_names_returns_empty_when_no_deps() {
        let yaml_content = r#"
apiVersion: graft/v0
commands:
  test:
    run: "cargo test"
"#;
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let names = parse_dependency_names(temp_file.path()).unwrap();
        assert!(names.is_empty());
    }

    #[test]
    fn parse_dependency_names_returns_sorted() {
        let yaml_content = r#"
dependencies:
  zebra: "https://example.com/zebra#main"
  alpha: "https://example.com/alpha#main"
  middle: "https://example.com/middle#main"
"#;
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let names = parse_dependency_names(temp_file.path()).unwrap();
        assert_eq!(names, vec!["alpha", "middle", "zebra"]);
    }
}
