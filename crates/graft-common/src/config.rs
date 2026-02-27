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
    /// State query name whose results dynamically populate `options` at form-open time.
    #[serde(default)]
    pub options_from: Option<String>,
    #[serde(default)]
    pub positional: bool,
}

/// Source for text piped to a command's stdin (shared definition).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StdinDef {
    /// Literal text, piped as-is (no template evaluation).
    Literal(String),
    /// Template file, evaluated with a template engine.
    Template {
        path: String,
        engine: Option<String>,
    },
}

/// A command definition from graft.yaml.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandDef {
    pub run: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Role classification: core | diagnostic | optional | advanced
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Concrete invocation example shown in help output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub example: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<ArgDef>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdin: Option<StdinDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<Vec<String>>,
    /// State names this command produces after running.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub writes: Vec<String>,
    /// State names this command requires to exist before running.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reads: Vec<String>,
}

/// A state query definition from graft.yaml.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateQueryDef {
    pub run: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Glob patterns for files this query reads. Drives cache key selection:
    /// - `None` or empty: never cached
    /// - non-empty, clean tree: commit hash
    /// - non-empty, dirty tree: SHA256 of input file contents
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<Vec<String>>,
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
#[allow(clippy::too_many_lines)]
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
                        // Validate: choice args must have non-empty options or options_from
                        if arg_def.arg_type == ArgType::Choice {
                            let has_options = arg_def
                                .options
                                .as_ref()
                                .is_some_and(|opts| !opts.is_empty());
                            let has_options_from = arg_def.options_from.is_some();
                            if !has_options && !has_options_from {
                                eprintln!(
                                    "Warning: Choice arg '{}' in command '{name}' has no options or options_from, skipping",
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

    let stdin = parse_stdin_def(config);
    let context = parse_context_def(config);
    let writes = parse_string_list_field(config, "writes");
    let reads = parse_string_list_field(config, "reads");

    let category = config
        .get("category")
        .and_then(|v| v.as_str())
        .map(String::from);
    let example = config
        .get("example")
        .and_then(|v| v.as_str())
        .map(String::from);

    Ok(CommandDef {
        run,
        description,
        category,
        example,
        working_dir,
        env,
        args,
        stdin,
        context,
        writes,
        reads,
    })
}

/// Parse an optional `stdin:` field from a command YAML value.
fn parse_stdin_def(config: &Value) -> Option<StdinDef> {
    config.get("stdin").and_then(|stdin_value| {
        if let Some(literal) = stdin_value.as_str() {
            Some(StdinDef::Literal(literal.to_string()))
        } else if let Some(mapping) = stdin_value.as_mapping() {
            mapping
                .get(Value::String("template".to_string()))
                .and_then(|v| v.as_str())
                .map(|tmpl_path| {
                    let engine = mapping
                        .get(Value::String("engine".to_string()))
                        .and_then(|v| v.as_str())
                        .map(String::from);
                    StdinDef::Template {
                        path: tmpl_path.to_string(),
                        engine,
                    }
                })
        } else {
            None
        }
    })
}

/// Parse an optional `context:` field from a command YAML value.
fn parse_context_def(config: &Value) -> Option<Vec<String>> {
    config
        .get("context")
        .and_then(|ctx_value| ctx_value.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
}

/// Parse a named YAML field that is a list of strings (e.g. `writes:` or `reads:`).
///
/// Returns an empty `Vec` when the field is absent or not a sequence.
fn parse_string_list_field(config: &Value, field: &str) -> Vec<String> {
    config
        .get(field)
        .and_then(|v| v.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// Retry configuration for a sequence step failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnStepFail {
    /// The step name that triggers retry behavior on failure.
    pub step: String,
    /// Command to run as recovery before retrying the failed step.
    pub recovery: String,
    /// Maximum number of retry attempts after the initial failure (default 3).
    /// `max: 3` means 1 initial run + 3 retries = 4 total step executions.
    #[serde(default = "default_max_retries")]
    pub max: u32,
}

fn default_max_retries() -> u32 {
    3
}

/// A sequence definition from graft.yaml.
///
/// A conditional guard that must evaluate to true for a step to execute.
///
/// Reads a field from a run-state JSON file and applies one operator.
/// Exactly one operator (`equals`, `not_equals`, `starts_with`, `not_starts_with`)
/// must be set; zero or multiple operators is a parse error.
///
/// If the state file or field is absent the condition evaluates to false
/// and the step is skipped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WhenCondition {
    /// Run-state file name (without `.json`), e.g. `"verify"`.
    pub state: String,
    /// JSON field name inside the state file, e.g. `"lint"`.
    pub field: String,
    /// Execute only when the field value equals this string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub equals: Option<String>,
    /// Execute only when the field value does not equal this string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not_equals: Option<String>,
    /// Execute only when the field value starts with this prefix.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starts_with: Option<String>,
    /// Execute only when the field value does not start with this prefix.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not_starts_with: Option<String>,
}

impl WhenCondition {
    /// Count how many operator fields are set. Valid = exactly 1.
    fn operator_count(&self) -> usize {
        usize::from(self.equals.is_some())
            + usize::from(self.not_equals.is_some())
            + usize::from(self.starts_with.is_some())
            + usize::from(self.not_starts_with.is_some())
    }

    /// Validate that exactly one operator is set.
    pub fn validate(&self) -> Result<(), String> {
        let count = self.operator_count();
        if count == 0 {
            return Err(format!(
                "when condition on state '{}' field '{}' has no operator (need exactly one of: \
                 equals, not_equals, starts_with, not_starts_with)",
                self.state, self.field
            ));
        }
        if count > 1 {
            return Err(format!(
                "when condition on state '{}' field '{}' has {} operators (need exactly one)",
                self.state, self.field, count
            ));
        }
        Ok(())
    }
}

/// A single step in a sequence — either a bare command name or a named step with options.
///
/// Both forms are equivalent when `timeout` and `when` are absent:
/// ```yaml
/// steps:
///   - implement          # bare string form
///   - name: verify       # object form (supports timeout and when condition)
///     timeout: 180
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepDef {
    /// Command name to execute.
    pub name: String,
    /// Optional per-step timeout in seconds. `None` means no timeout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    /// Optional condition: the step only executes when the condition evaluates to true.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<WhenCondition>,
}

impl StepDef {
    /// Create a bare step with no timeout or condition.
    pub fn simple(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            timeout: None,
            when: None,
        }
    }
}

/// A sequence declares an ordered list of command references (steps) and optional
/// args that are passed through to every step using "pass-all" semantics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SequenceDef {
    /// Ordered list of steps to execute.
    pub steps: Vec<StepDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Role classification: core | diagnostic | optional | advanced
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Concrete invocation example shown in help output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub example: Option<String>,
    /// Args declared on the sequence, passed to every step.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<ArgDef>,
    /// Optional retry configuration for a named step.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_step_fail: Option<OnStepFail>,
    /// When true, writes checkpoint.json after all steps succeed for human review.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkpoint: Option<bool>,
}

/// Parse sequences section from a graft.yaml content string.
///
/// Returns a `HashMap` of sequence name to sequence definition.
/// Returns an empty `HashMap` if the content is empty or has no sequences section.
pub fn parse_sequences_from_str(content: &str) -> Result<HashMap<String, SequenceDef>, String> {
    if content.is_empty() {
        return Ok(HashMap::new());
    }

    let yaml: Value =
        serde_yaml::from_str(content).map_err(|e| format!("Failed to parse graft.yaml: {e}"))?;

    let mut sequences = HashMap::new();

    if let Some(sequences_section) = yaml.get("sequences") {
        if let Some(sequences_map) = sequences_section.as_mapping() {
            for (name, config) in sequences_map {
                if let Some(name_str) = name.as_str() {
                    match parse_sequence(name_str, config) {
                        Ok(seq) => {
                            sequences.insert(name_str.to_string(), seq);
                        }
                        Err(e) => {
                            eprintln!("Warning: Failed to parse sequence '{name_str}': {e}");
                        }
                    }
                }
            }
        }
    }

    Ok(sequences)
}

/// Parse a single sequence definition from YAML config.
#[allow(clippy::too_many_lines)]
/// Parse a single step value — either a bare string or an object with `name` + optional `timeout`/`when`.
fn parse_step_def(val: &Value, seq_name: &str, idx: usize) -> Option<StepDef> {
    if let Some(s) = val.as_str() {
        return Some(StepDef::simple(s));
    }
    if let Some(map) = val.as_mapping() {
        let name = map.get("name").and_then(|v| v.as_str()).map(String::from)?;
        let timeout = map.get("timeout").and_then(serde_yaml::Value::as_u64);

        // Parse optional `when:` condition block
        let when = if let Some(when_val) = map.get("when") {
            match serde_yaml::from_value::<WhenCondition>(when_val.clone()) {
                Ok(cond) => match cond.validate() {
                    Ok(()) => Some(cond),
                    Err(e) => {
                        eprintln!(
                            "Warning: Step '{name}' in sequence '{seq_name}' has invalid when condition: {e}"
                        );
                        return None;
                    }
                },
                Err(e) => {
                    eprintln!(
                        "Warning: Step '{name}' in sequence '{seq_name}' has malformed when condition: {e}"
                    );
                    return None;
                }
            }
        } else {
            None
        };

        return Some(StepDef {
            name,
            timeout,
            when,
        });
    }
    eprintln!("Warning: Step {idx} in sequence '{seq_name}' is not a string or object, skipping");
    None
}

#[allow(clippy::too_many_lines)]
fn parse_sequence(name: &str, config: &Value) -> Result<SequenceDef, String> {
    let steps = config
        .get("steps")
        .and_then(|v| v.as_sequence())
        .map(|seq| {
            seq.iter()
                .enumerate()
                .filter_map(|(i, v)| parse_step_def(v, name, i))
                .collect::<Vec<_>>()
        })
        .ok_or_else(|| format!("Sequence '{name}' missing 'steps' field"))?;

    if steps.is_empty() {
        return Err(format!("Sequence '{name}' must have at least one step"));
    }

    let description = config
        .get("description")
        .and_then(|d| d.as_str())
        .map(std::string::ToString::to_string);

    // Reuse the same args parsing as parse_command
    let args = config
        .get("args")
        .and_then(|a| a.as_sequence())
        .map(|args_seq| {
            let mut parsed_args = Vec::new();
            let mut seen_names = std::collections::HashSet::new();
            for (i, arg_val) in args_seq.iter().enumerate() {
                match serde_yaml::from_value::<ArgDef>(arg_val.clone()) {
                    Ok(arg_def) => {
                        if arg_def.arg_type == ArgType::Choice {
                            let has_options = arg_def
                                .options
                                .as_ref()
                                .is_some_and(|opts| !opts.is_empty());
                            let has_options_from = arg_def.options_from.is_some();
                            if !has_options && !has_options_from {
                                eprintln!(
                                    "Warning: Choice arg '{}' in sequence '{name}' has no options or options_from, skipping",
                                    arg_def.name
                                );
                                continue;
                            }
                        }
                        if arg_def.arg_type == ArgType::Flag && arg_def.positional {
                            eprintln!(
                                "Warning: Flag arg '{}' in sequence '{name}' cannot be positional, skipping",
                                arg_def.name
                            );
                            continue;
                        }
                        if !seen_names.insert(arg_def.name.clone()) {
                            eprintln!(
                                "Warning: Duplicate arg name '{}' in sequence '{name}', skipping",
                                arg_def.name
                            );
                            continue;
                        }
                        parsed_args.push(arg_def);
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to parse arg {i} in sequence '{name}': {e}");
                    }
                }
            }
            parsed_args
        })
        .unwrap_or_default();

    // Parse on_step_fail (optional)
    let on_step_fail = if let Some(osf_value) = config.get("on_step_fail") {
        let osf_step = osf_value
            .get("step")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("Sequence '{name}' on_step_fail missing 'step' field"))?
            .to_string();

        let osf_recovery = osf_value
            .get("recovery")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("Sequence '{name}' on_step_fail missing 'recovery' field"))?
            .to_string();

        let osf_max = osf_value
            .get("max")
            .and_then(serde_yaml::Value::as_u64)
            .and_then(|v| u32::try_from(v).ok())
            .unwrap_or_else(default_max_retries);

        Some(OnStepFail {
            step: osf_step,
            recovery: osf_recovery,
            max: osf_max,
        })
    } else {
        None
    };

    // Parse checkpoint (optional)
    let checkpoint = config
        .get("checkpoint")
        .and_then(serde_yaml::Value::as_bool);

    let category = config
        .get("category")
        .and_then(|v| v.as_str())
        .map(String::from);
    let example = config
        .get("example")
        .and_then(|v| v.as_str())
        .map(String::from);

    Ok(SequenceDef {
        steps,
        description,
        category,
        example,
        args,
        on_step_fail,
        checkpoint,
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

    // Get cache.inputs (optional list of glob patterns)
    let inputs = config
        .get("cache")
        .and_then(|c| c.get("inputs"))
        .and_then(serde_yaml::Value::as_sequence)
        .map(|seq| {
            seq.iter()
                .filter_map(|v| v.as_str().map(std::string::ToString::to_string))
                .collect::<Vec<_>>()
        });

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
        inputs,
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
    fn parse_commands_accepts_choice_with_options_from() {
        let yaml_content = r#"
commands:
  iterate:
    run: "bash scripts/iterate.sh"
    args:
      - name: slice
        type: choice
        description: "Slice to iterate on"
        required: true
        positional: true
        options_from: slices
"#;
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let commands = parse_commands(temp_file.path()).unwrap();
        let iterate = commands.get("iterate").unwrap();
        let args = iterate.args.as_ref().unwrap();

        // Choice with options_from (no static options) should be accepted
        assert_eq!(args.len(), 1);
        assert_eq!(args[0].name, "slice");
        assert_eq!(args[0].arg_type, ArgType::Choice);
        assert!(args[0].options.is_none());
        assert_eq!(args[0].options_from.as_deref(), Some("slices"));
        assert!(args[0].positional);
        assert!(args[0].required);
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
      inputs:
        - "**/*.py"
        - "pyproject.toml"
    timeout: 60

  tasks:
    run: "task-tracker status"
    timeout: 30
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let queries = parse_state_queries(temp_file.path()).unwrap();

        assert_eq!(queries.len(), 2);

        let coverage = queries.get("coverage").unwrap();
        assert_eq!(coverage.run, "pytest --cov");
        assert_eq!(coverage.description.as_deref(), Some("Run coverage"));
        let inputs = coverage.inputs.as_ref().unwrap();
        assert_eq!(inputs, &["**/*.py", "pyproject.toml"]);
        assert_eq!(coverage.timeout, Some(60));

        let tasks = queries.get("tasks").unwrap();
        assert_eq!(tasks.run, "task-tracker status");
        assert!(tasks.description.is_none());
        assert!(tasks.inputs.is_none()); // no inputs → never cached
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
    fn parse_state_queries_no_inputs_is_none() {
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
        assert!(simple.inputs.is_none()); // No cache → always run fresh
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
        assert!(simple.inputs.is_none()); // No inputs → never cached
        assert_eq!(simple.timeout, Some(10));
    }

    #[test]
    fn parse_state_queries_inputs_list() {
        let yaml_content = r#"
state:
  verify:
    run: "cargo test"
    cache:
      inputs:
        - "**/*.rs"
        - "Cargo.toml"
        - "Cargo.lock"
    timeout: 180
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let queries = parse_state_queries(temp_file.path()).unwrap();
        let verify = queries.get("verify").unwrap();
        let inputs = verify.inputs.as_ref().unwrap();
        assert_eq!(inputs, &["**/*.rs", "Cargo.toml", "Cargo.lock"]);
        assert_eq!(verify.timeout, Some(180));
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

    #[test]
    fn parse_sequences_from_yaml() {
        let yaml_content = r#"
sequences:
  implement-verified:
    description: "Implement and verify"
    steps:
      - implement
      - verify
    args:
      - name: slice
        type: choice
        description: "Slice to implement"
        required: true
        positional: true
        options_from: slices
"#;
        let sequences = parse_sequences_from_str(yaml_content).unwrap();
        assert_eq!(sequences.len(), 1);
        let seq = sequences.get("implement-verified").unwrap();
        assert_eq!(
            seq.steps,
            vec![StepDef::simple("implement"), StepDef::simple("verify")]
        );
        assert_eq!(seq.description.as_deref(), Some("Implement and verify"));
        assert_eq!(seq.args.len(), 1);
        assert_eq!(seq.args[0].name, "slice");
    }

    #[test]
    fn parse_sequences_step_object_form() {
        let yaml_content = r#"
sequences:
  timed:
    steps:
      - name: implement
        timeout: 600
      - name: verify
        timeout: 180
"#;
        let sequences = parse_sequences_from_str(yaml_content).unwrap();
        let seq = sequences.get("timed").unwrap();
        assert_eq!(seq.steps[0].name, "implement");
        assert_eq!(seq.steps[0].timeout, Some(600));
        assert_eq!(seq.steps[1].name, "verify");
        assert_eq!(seq.steps[1].timeout, Some(180));
    }

    #[test]
    fn parse_sequences_empty_returns_empty() {
        let sequences = parse_sequences_from_str("").unwrap();
        assert!(sequences.is_empty());
    }

    #[test]
    fn parse_sequences_no_section_returns_empty() {
        let yaml_content = r#"
apiVersion: graft/v0
commands:
  test:
    run: "cargo test"
"#;
        let sequences = parse_sequences_from_str(yaml_content).unwrap();
        assert!(sequences.is_empty());
    }

    // ── stdin and context parsing tests ──────────────────────────────────────

    #[test]
    fn parse_commands_with_stdin_literal() {
        let yaml_content = r#"
commands:
  gen:
    run: "cat"
    stdin: "text"
"#;
        let commands = parse_commands_from_str(yaml_content).unwrap();
        let cmd = commands.get("gen").unwrap();
        assert_eq!(cmd.stdin, Some(StdinDef::Literal("text".to_string())));
    }

    #[test]
    fn parse_commands_with_stdin_template() {
        let yaml_content = r#"
commands:
  gen:
    run: "cat"
    stdin:
      template: "r.md"
"#;
        let commands = parse_commands_from_str(yaml_content).unwrap();
        let cmd = commands.get("gen").unwrap();
        match &cmd.stdin {
            Some(StdinDef::Template { path, engine }) => {
                assert_eq!(path, "r.md");
                assert_eq!(*engine, None);
            }
            other => panic!("expected Template, got: {other:?}"),
        }
    }

    #[test]
    fn parse_commands_with_stdin_template_and_engine() {
        let yaml_content = r#"
commands:
  gen:
    run: "cat"
    stdin:
      template: "r.md"
      engine: "tera"
"#;
        let commands = parse_commands_from_str(yaml_content).unwrap();
        let cmd = commands.get("gen").unwrap();
        match &cmd.stdin {
            Some(StdinDef::Template { path, engine }) => {
                assert_eq!(path, "r.md");
                assert_eq!(*engine, Some("tera".to_string()));
            }
            other => panic!("expected Template with engine, got: {other:?}"),
        }
    }

    #[test]
    fn parse_commands_with_context_list() {
        let yaml_content = r#"
commands:
  gen:
    run: "echo ok"
    context:
      - a
      - b
"#;
        let commands = parse_commands_from_str(yaml_content).unwrap();
        let cmd = commands.get("gen").unwrap();
        assert_eq!(cmd.context, Some(vec!["a".to_string(), "b".to_string()]));
    }

    #[test]
    fn parse_commands_without_stdin_or_context() {
        let yaml_content = r#"
commands:
  test:
    run: "cargo test"
"#;
        let commands = parse_commands_from_str(yaml_content).unwrap();
        let cmd = commands.get("test").unwrap();
        assert_eq!(cmd.stdin, None);
        assert_eq!(cmd.context, None);
    }

    #[test]
    fn parse_sequences_on_step_fail_max_defaults_to_3() {
        let yaml_content = r"
sequences:
  ci:
    steps:
      - build
      - test
    on_step_fail:
      step: test
      recovery: fix
";
        let sequences = parse_sequences_from_str(yaml_content).unwrap();
        let seq = sequences.get("ci").unwrap();
        let osf = seq.on_step_fail.as_ref().unwrap();
        assert_eq!(osf.max, 3, "max should default to 3 when omitted");
    }

    #[test]
    fn parse_sequences_on_step_fail_explicit_max_overrides_default() {
        let yaml_content = r"
sequences:
  ci:
    steps:
      - build
      - test
    on_step_fail:
      step: test
      recovery: fix
      max: 5
";
        let sequences = parse_sequences_from_str(yaml_content).unwrap();
        let seq = sequences.get("ci").unwrap();
        let osf = seq.on_step_fail.as_ref().unwrap();
        assert_eq!(osf.max, 5);
    }

    // ── WhenCondition validation tests ───────────────────────────────────────

    #[test]
    fn when_condition_validate_zero_operators_returns_error() {
        let cond = WhenCondition {
            state: "verify".to_string(),
            field: "lint".to_string(),
            equals: None,
            not_equals: None,
            starts_with: None,
            not_starts_with: None,
        };
        assert!(
            cond.validate().is_err(),
            "zero operators should be a validation error"
        );
    }

    #[test]
    fn when_condition_validate_multiple_operators_returns_error() {
        let cond = WhenCondition {
            state: "verify".to_string(),
            field: "lint".to_string(),
            equals: Some("OK".to_string()),
            not_equals: Some("FAILED".to_string()),
            starts_with: None,
            not_starts_with: None,
        };
        assert!(
            cond.validate().is_err(),
            "multiple operators should be a validation error"
        );
    }

    #[test]
    fn when_condition_validate_single_operator_ok() {
        for cond in [
            WhenCondition {
                state: "v".to_string(),
                field: "f".to_string(),
                equals: Some("x".to_string()),
                not_equals: None,
                starts_with: None,
                not_starts_with: None,
            },
            WhenCondition {
                state: "v".to_string(),
                field: "f".to_string(),
                equals: None,
                not_equals: Some("x".to_string()),
                starts_with: None,
                not_starts_with: None,
            },
            WhenCondition {
                state: "v".to_string(),
                field: "f".to_string(),
                equals: None,
                not_equals: None,
                starts_with: Some("x".to_string()),
                not_starts_with: None,
            },
            WhenCondition {
                state: "v".to_string(),
                field: "f".to_string(),
                equals: None,
                not_equals: None,
                starts_with: None,
                not_starts_with: Some("x".to_string()),
            },
        ] {
            assert!(
                cond.validate().is_ok(),
                "single operator should pass validation"
            );
        }
    }

    // ── parse_step_def with when: condition ──────────────────────────────────

    #[test]
    fn parse_step_with_when_equals_condition() {
        let yaml_content = r"
sequences:
  deploy:
    steps:
      - name: push
        when:
          state: verify
          field: lint
          equals: OK
";
        let sequences = parse_sequences_from_str(yaml_content).unwrap();
        let seq = sequences.get("deploy").unwrap();
        assert_eq!(seq.steps.len(), 1);
        let step = &seq.steps[0];
        assert_eq!(step.name, "push");
        let when = step
            .when
            .as_ref()
            .expect("step should have a when condition");
        assert_eq!(when.state, "verify");
        assert_eq!(when.field, "lint");
        assert_eq!(when.equals.as_deref(), Some("OK"));
        assert!(when.not_equals.is_none());
    }

    #[test]
    fn parse_step_with_when_not_starts_with_condition() {
        let yaml_content = r#"
sequences:
  ci:
    steps:
      - name: baseline-check
        when:
          state: session
          field: baseline_sha
          not_starts_with: ""
"#;
        let sequences = parse_sequences_from_str(yaml_content).unwrap();
        let seq = sequences.get("ci").unwrap();
        let step = &seq.steps[0];
        let when = step
            .when
            .as_ref()
            .expect("step should have a when condition");
        assert_eq!(when.state, "session");
        assert_eq!(when.field, "baseline_sha");
        assert_eq!(when.not_starts_with.as_deref(), Some(""));
    }

    #[test]
    fn parse_step_with_invalid_when_no_operator_drops_step() {
        // A when: block with no operator should cause the step to be dropped (warning printed)
        let yaml_content = r"
sequences:
  ci:
    steps:
      - name: bad-step
        when:
          state: verify
          field: lint
      - good-step
";
        let sequences = parse_sequences_from_str(yaml_content).unwrap();
        let seq = sequences.get("ci").unwrap();
        // bad-step should be dropped, good-step should remain
        assert_eq!(seq.steps.len(), 1, "bad-step should be dropped");
        assert_eq!(seq.steps[0].name, "good-step");
    }

    #[test]
    fn parse_bare_step_has_no_when_condition() {
        let yaml_content = r"
sequences:
  ci:
    steps:
      - build
";
        let sequences = parse_sequences_from_str(yaml_content).unwrap();
        let seq = sequences.get("ci").unwrap();
        assert!(
            seq.steps[0].when.is_none(),
            "bare step should have no when condition"
        );
    }
}
