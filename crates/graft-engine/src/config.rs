//! Configuration parsing for graft.yaml files.

use crate::{
    Change, Command, DependencySpec, GitRef, GitUrl, GraftConfig, GraftError, Metadata, Result,
    ScionHooks, StateCache, StateQuery, StdinSource,
};
use serde_yaml::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Coerce a YAML value to a string suitable for env vars.
///
/// Accepts strings, numbers, and booleans. Returns `None` for nulls,
/// sequences, and mappings (callers should error on those).
fn yaml_value_to_env_string(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// Human-readable name for a YAML value type (for error messages).
fn yaml_type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Sequence(_) => "sequence",
        Value::Mapping(_) => "mapping",
        Value::Tagged(_) => "tagged",
    }
}

/// Parse a YAML sequence into a `Vec<String>`, erroring on non-string elements.
fn parse_string_list(seq: &[Value], path: &str, field: &str) -> Result<Vec<String>> {
    let mut result = Vec::with_capacity(seq.len());
    for (i, v) in seq.iter().enumerate() {
        match v.as_str() {
            Some(s) => result.push(s.to_string()),
            None => {
                return Err(GraftError::ConfigValidation {
                    path: path.to_string(),
                    field: field.to_string(),
                    reason: format!(
                        "element [{}] is {} but must be a string",
                        i,
                        yaml_type_name(v)
                    ),
                });
            }
        }
    }
    Ok(result)
}

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

/// Load dependency configs from `.graft/<dep>/graft.yaml` for each dependency
/// in the project config. Returns `(successes, warnings)` — dependencies whose
/// graft.yaml is missing or invalid are reported in `warnings` instead of
/// being silently skipped.
pub fn load_dep_configs(
    repo_path: impl AsRef<Path>,
    config: &GraftConfig,
) -> (Vec<(String, GraftConfig)>, Vec<String>) {
    let mut successes = Vec::new();
    let mut warnings = Vec::new();
    for dep_name in config.dependencies.keys() {
        let dep_yaml = repo_path
            .as_ref()
            .join(".graft")
            .join(dep_name)
            .join("graft.yaml");
        let dep_dir = repo_path.as_ref().join(".graft").join(dep_name);
        if !dep_dir.exists() {
            warnings.push(format!(
                "dependency '{dep_name}': .graft/{dep_name}/ not found (submodule not initialized?)"
            ));
            continue;
        }
        if !dep_yaml.exists() {
            // Not every dependency is graft-aware (e.g. copier templates).
            // Skip silently when the directory exists but has no graft.yaml.
            continue;
        }
        match parse_graft_yaml(&dep_yaml) {
            Ok(cfg) => successes.push((dep_name.clone(), cfg)),
            Err(e) => {
                warnings.push(format!(
                    "dependency '{dep_name}': failed to parse .graft/{dep_name}/graft.yaml: {e}"
                ));
            }
        }
    }
    (successes, warnings)
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

            if let Some(cat_value) = cmd_obj.get(Value::String("category".to_string())) {
                if let Some(cat) = cat_value.as_str() {
                    command.category = Some(cat.to_string());
                }
            }

            if let Some(ex_value) = cmd_obj.get(Value::String("example".to_string())) {
                if let Some(ex) = ex_value.as_str() {
                    command.example = Some(ex.to_string());
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
                        let key = k
                            .as_str()
                            .ok_or_else(|| GraftError::ConfigValidation {
                                path: path.to_string(),
                                field: format!("commands.{name}.env"),
                                reason: format!(
                                    "env key is {} but must be a string",
                                    yaml_type_name(k)
                                ),
                            })?
                            .to_string();
                        let val = yaml_value_to_env_string(v).ok_or_else(|| {
                            GraftError::ConfigValidation {
                                path: path.to_string(),
                                field: format!("commands.{name}.env.{key}"),
                                reason: format!(
                                    "env value is {} but must be a string, number, or bool",
                                    yaml_type_name(v)
                                ),
                            }
                        })?;
                        env.insert(key, val);
                    }
                    command.env = Some(env);
                }
            }

            // Parse stdin (optional)
            if let Some(stdin_value) = cmd_obj.get(Value::String("stdin".to_string())) {
                if let Some(literal) = stdin_value.as_str() {
                    command.stdin = Some(StdinSource::Literal(literal.to_string()));
                } else if let Some(mapping) = stdin_value.as_mapping() {
                    if let Some(tmpl_path) = mapping
                        .get(Value::String("template".to_string()))
                        .and_then(|v| v.as_str())
                    {
                        let engine = mapping
                            .get(Value::String("engine".to_string()))
                            .and_then(|v| v.as_str())
                            .map(String::from);
                        command.stdin = Some(StdinSource::Template {
                            path: tmpl_path.to_string(),
                            engine,
                        });
                    }
                }
            }

            // Parse context (optional)
            if let Some(ctx_value) = cmd_obj.get(Value::String("context".to_string())) {
                if let Some(seq) = ctx_value.as_sequence() {
                    command.context =
                        parse_string_list(seq, path, &format!("commands.{name}.context"))?;
                }
            }

            // Parse writes (optional)
            if let Some(writes_value) = cmd_obj.get(Value::String("writes".to_string())) {
                if let Some(seq) = writes_value.as_sequence() {
                    command.writes =
                        parse_string_list(seq, path, &format!("commands.{name}.writes"))?;
                }
            }

            // Parse reads (optional)
            if let Some(reads_value) = cmd_obj.get(Value::String("reads".to_string())) {
                if let Some(seq) = reads_value.as_sequence() {
                    command.reads =
                        parse_string_list(seq, path, &format!("commands.{name}.reads"))?;
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

    // Parse state queries (optional)
    if let Some(state_value) = obj.get(Value::String("state".to_string())) {
        let state_map = state_value
            .as_mapping()
            .ok_or_else(|| GraftError::ConfigValidation {
                path: path.to_string(),
                field: "state".to_string(),
                reason: "must be a mapping/dict of query_name: {...}".to_string(),
            })?;

        for (query_name, query_data) in state_map {
            let name = query_name
                .as_str()
                .ok_or_else(|| GraftError::ConfigValidation {
                    path: path.to_string(),
                    field: "state".to_string(),
                    reason: "state query name must be a string".to_string(),
                })?;

            let query_obj =
                query_data
                    .as_mapping()
                    .ok_or_else(|| GraftError::ConfigValidation {
                        path: path.to_string(),
                        field: format!("state.{name}"),
                        reason: "state query must be an object".to_string(),
                    })?;

            // Extract 'run' field (required)
            let run = query_obj
                .get(Value::String("run".to_string()))
                .ok_or_else(|| GraftError::ConfigValidation {
                    path: path.to_string(),
                    field: format!("state.{name}"),
                    reason: "state query must have 'run' field".to_string(),
                })?
                .as_str()
                .ok_or_else(|| GraftError::ConfigValidation {
                    path: path.to_string(),
                    field: format!("state.{name}.run"),
                    reason: "'run' must be a string".to_string(),
                })?;

            let mut query = StateQuery::new(name, run)?;

            // Parse cache (optional)
            if let Some(cache_value) = query_obj.get(Value::String("cache".to_string())) {
                let cache_obj =
                    cache_value
                        .as_mapping()
                        .ok_or_else(|| GraftError::ConfigValidation {
                            path: path.to_string(),
                            field: format!("state.{name}.cache"),
                            reason: "cache must be an object".to_string(),
                        })?;

                let inputs = match cache_obj
                    .get(Value::String("inputs".to_string()))
                    .and_then(serde_yaml::Value::as_sequence)
                {
                    Some(seq) => {
                        parse_string_list(seq, path, &format!("state.{name}.cache.inputs"))?
                    }
                    None => Vec::new(),
                };

                let ttl = cache_obj
                    .get(Value::String("ttl".to_string()))
                    .and_then(serde_yaml::Value::as_u64);

                query.cache = StateCache { inputs, ttl };
            }

            // Parse timeout (optional)
            if let Some(timeout_value) = query_obj.get(Value::String("timeout".to_string())) {
                let timeout =
                    timeout_value
                        .as_u64()
                        .ok_or_else(|| GraftError::ConfigValidation {
                            path: path.to_string(),
                            field: format!("state.{name}.timeout"),
                            reason: "timeout must be a positive integer".to_string(),
                        })?;
                query.timeout = Some(timeout);
            }

            config.state.insert(name.to_string(), query);
        }
    }

    // Parse sequences (optional) using graft_common parser, then validate steps exist
    {
        let parsed_seqs = graft_common::parse_sequences_from_str(content).map_err(|e| {
            GraftError::ConfigParse {
                path: path.to_string(),
                reason: format!("failed to parse sequences: {e}"),
            }
        })?;

        for (seq_name, seq_def) in parsed_seqs {
            // Validate that all referenced steps exist as commands
            for step in &seq_def.steps {
                if !config.commands.contains_key(step.name.as_str()) {
                    return Err(GraftError::ConfigValidation {
                        path: path.to_string(),
                        field: format!("sequences.{seq_name}.steps"),
                        reason: format!("step '{}' not found in commands section", step.name),
                    });
                }
            }
            // Validate on_step_fail references
            if let Some(ref osf) = seq_def.on_step_fail {
                if !seq_def.steps.iter().any(|s| s.name == osf.step) {
                    return Err(GraftError::ConfigValidation {
                        path: path.to_string(),
                        field: format!("sequences.{seq_name}.on_step_fail.step"),
                        reason: format!("step '{}' not found in sequence's steps list", osf.step),
                    });
                }
                if !config.commands.contains_key(osf.recovery.as_str()) {
                    return Err(GraftError::ConfigValidation {
                        path: path.to_string(),
                        field: format!("sequences.{seq_name}.on_step_fail.recovery"),
                        reason: format!(
                            "recovery command '{}' not found in commands section",
                            osf.recovery
                        ),
                    });
                }
            }
            config.sequences.insert(seq_name, seq_def);
        }
    }

    // Parse scions (optional)
    if let Some(scions_value) = obj.get(Value::String("scions".to_string())) {
        let scions_obj = scions_value
            .as_mapping()
            .ok_or_else(|| GraftError::ConfigValidation {
                path: path.to_string(),
                field: "scions".to_string(),
                reason: "must be a mapping/dict of hook_name: command_name(s)".to_string(),
            })?;

        let mut hooks = ScionHooks {
            on_create: None,
            pre_fuse: None,
            post_fuse: None,
            on_prune: None,
            start: None,
            source: None,
        };

        for (key, value) in scions_obj {
            let key_str = key.as_str().ok_or_else(|| GraftError::ConfigValidation {
                path: path.to_string(),
                field: "scions".to_string(),
                reason: "hook name must be a string".to_string(),
            })?;
            // start and source are single string fields, handle separately
            if key_str == "start" || key_str == "source" {
                match value.as_str() {
                    Some(s) => {
                        if key_str == "start" {
                            hooks.start = Some(s.to_string());
                        } else {
                            hooks.source = Some(s.to_string());
                        }
                    }
                    None => {
                        return Err(GraftError::ConfigValidation {
                            path: path.to_string(),
                            field: format!("scions.{key_str}"),
                            reason: format!("{key_str} must be a string"),
                        });
                    }
                }
                continue;
            }

            let cmds = parse_hook_commands(value, path, &format!("scions.{key_str}"))?;
            match key_str {
                "on_create" => hooks.on_create = cmds,
                "pre_fuse" => hooks.pre_fuse = cmds,
                "post_fuse" => hooks.post_fuse = cmds,
                "on_prune" => hooks.on_prune = cmds,
                other => {
                    return Err(GraftError::ConfigValidation {
                        path: path.to_string(),
                        field: format!("scions.{other}"),
                        reason: format!("unknown hook point '{other}'; expected on_create, pre_fuse, post_fuse, on_prune, start, or source"),
                    });
                }
            }
        }

        // Only store if at least one hook was defined
        if hooks.on_create.is_some()
            || hooks.pre_fuse.is_some()
            || hooks.post_fuse.is_some()
            || hooks.on_prune.is_some()
            || hooks.start.is_some()
            || hooks.source.is_some()
        {
            config.scion_hooks = Some(hooks);
        }
    }

    // Validate configuration
    config.validate()?;

    Ok(config)
}

/// Parse a hook value that can be a single string or a list of strings.
///
/// Returns `Some(vec)` if the value is a non-empty string or non-empty list,
/// or `None` if the value is null.
fn parse_hook_commands(value: &Value, path: &str, field: &str) -> Result<Option<Vec<String>>> {
    match value {
        Value::String(s) => Ok(Some(vec![s.clone()])),
        Value::Sequence(seq) => {
            let cmds = parse_string_list(seq, path, field)?;
            if cmds.is_empty() {
                Ok(None)
            } else {
                Ok(Some(cmds))
            }
        }
        Value::Null => Ok(None),
        _ => Err(GraftError::ConfigValidation {
            path: path.to_string(),
            field: field.to_string(),
            reason: "hook must be a command name (string) or list of command names".to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_config() {
        let yaml = r"
apiVersion: graft/v0
";
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
        let yaml = r"
apiVersion: graft/v0
changes:
  v1.0.0:
    migration: missing-command
";
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
        let yaml = r"
apiVersion: v1
";
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

    // ── stdin and context parsing tests ──────────────────────────────────────

    #[test]
    fn parses_command_with_stdin_literal() {
        let yaml = r#"
apiVersion: graft/v0
commands:
  gen:
    run: "cat"
    stdin: "hello"
"#;
        let config = parse_graft_yaml_str(yaml, "test.yaml").unwrap();
        let cmd = config.get_command("gen").unwrap();
        assert_eq!(cmd.stdin, Some(StdinSource::Literal("hello".to_string())));
    }

    #[test]
    fn parses_command_with_stdin_template() {
        let yaml = r#"
apiVersion: graft/v0
commands:
  gen:
    run: "cat"
    stdin:
      template: "f.md"
"#;
        let config = parse_graft_yaml_str(yaml, "test.yaml").unwrap();
        let cmd = config.get_command("gen").unwrap();
        match &cmd.stdin {
            Some(StdinSource::Template { path, engine }) => {
                assert_eq!(path, "f.md");
                assert_eq!(*engine, None);
            }
            other => panic!("expected Template, got: {other:?}"),
        }
    }

    #[test]
    fn parses_command_with_stdin_template_and_engine() {
        let yaml = r#"
apiVersion: graft/v0
commands:
  gen:
    run: "cat"
    stdin:
      template: "f.md"
      engine: "tera"
"#;
        let config = parse_graft_yaml_str(yaml, "test.yaml").unwrap();
        let cmd = config.get_command("gen").unwrap();
        match &cmd.stdin {
            Some(StdinSource::Template { path, engine }) => {
                assert_eq!(path, "f.md");
                assert_eq!(*engine, Some("tera".to_string()));
            }
            other => panic!("expected Template with engine, got: {other:?}"),
        }
    }

    #[test]
    fn parses_command_with_context() {
        let yaml = r#"
apiVersion: graft/v0
commands:
  gen:
    run: "echo ok"
    context:
      - coverage
state:
  coverage:
    run: "echo 87.5"
"#;
        let config = parse_graft_yaml_str(yaml, "test.yaml").unwrap();
        let cmd = config.get_command("gen").unwrap();
        assert_eq!(cmd.context, vec!["coverage".to_string()]);
    }

    #[test]
    fn parses_command_with_writes_and_reads() {
        let yaml = r#"
apiVersion: graft/v0
commands:
  implement:
    run: "bash scripts/implement.sh"
    writes:
      - session
  resume:
    run: "bash scripts/resume.sh"
    reads:
      - session
"#;
        let config = parse_graft_yaml_str(yaml, "test.yaml").unwrap();
        let implement = config.get_command("implement").unwrap();
        assert_eq!(implement.writes, vec!["session".to_string()]);
        assert!(implement.reads.is_empty());
        let resume = config.get_command("resume").unwrap();
        assert_eq!(resume.reads, vec!["session".to_string()]);
        assert!(resume.writes.is_empty());
    }

    #[test]
    fn command_writes_reads_absent_gives_empty_vecs() {
        let yaml = r#"
apiVersion: graft/v0
commands:
  plain:
    run: "echo ok"
"#;
        let config = parse_graft_yaml_str(yaml, "test.yaml").unwrap();
        let cmd = config.get_command("plain").unwrap();
        assert!(cmd.writes.is_empty());
        assert!(cmd.reads.is_empty());
    }

    #[test]
    fn parses_state_query_with_inputs_and_ttl() {
        let yaml = r#"
apiVersion: graft/v0
state:
  verify:
    run: "cargo test"
    cache:
      inputs:
        - "**/*.rs"
        - "Cargo.toml"
      ttl: 120
    timeout: 60
"#;
        let config = parse_graft_yaml_str(yaml, "test.yaml").unwrap();
        let query = config.state.get("verify").unwrap();
        assert_eq!(query.cache.inputs, &["**/*.rs", "Cargo.toml"]);
        assert_eq!(query.cache.ttl, Some(120));
        assert_eq!(query.timeout, Some(60));
    }

    #[test]
    fn parses_command_without_stdin_or_context() {
        let yaml = r#"
apiVersion: graft/v0
commands:
  test:
    run: "cargo test"
"#;
        let config = parse_graft_yaml_str(yaml, "test.yaml").unwrap();
        let cmd = config.get_command("test").unwrap();
        assert_eq!(cmd.stdin, None);
        assert!(cmd.context.is_empty());
    }

    #[test]
    fn rejects_context_referencing_missing_state() {
        let yaml = r#"
apiVersion: graft/v0
commands:
  gen:
    run: "echo ok"
    context:
      - missing
"#;
        let result = parse_graft_yaml_str(yaml, "test.yaml");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, GraftError::ConfigValidation { .. }));
        assert!(
            err.to_string().contains("missing"),
            "error should mention 'missing': {err}"
        );
    }

    #[test]
    fn parses_command_with_stdin_and_context() {
        let yaml = r#"
apiVersion: graft/v0
commands:
  gen:
    run: "report-tool"
    stdin: "literal text"
    context:
      - coverage
state:
  coverage:
    run: "echo 87.5"
"#;
        let config = parse_graft_yaml_str(yaml, "test.yaml").unwrap();
        let cmd = config.get_command("gen").unwrap();
        assert_eq!(
            cmd.stdin,
            Some(StdinSource::Literal("literal text".to_string()))
        );
        assert_eq!(cmd.context, vec!["coverage".to_string()]);
    }

    #[test]
    fn parses_sequences() {
        let yaml = r#"
apiVersion: graft/v0
commands:
  implement:
    run: "bash scripts/implement.sh {slice}"
  verify:
    run: "bash scripts/verify.sh"
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
        let config = parse_graft_yaml_str(yaml, "test.yaml").unwrap();
        assert_eq!(config.sequences.len(), 1);
        let seq = config.sequences.get("implement-verified").unwrap();
        assert_eq!(
            seq.steps,
            vec![
                graft_common::StepDef::simple("implement"),
                graft_common::StepDef::simple("verify")
            ]
        );
        assert_eq!(seq.description.as_deref(), Some("Implement and verify"));
        assert_eq!(seq.args.len(), 1);
    }

    #[test]
    fn sequence_with_missing_step_command_fails() {
        let yaml = r#"
apiVersion: graft/v0
commands:
  implement:
    run: "bash scripts/implement.sh"
sequences:
  test-seq:
    steps:
      - implement
      - nonexistent
"#;
        let result = parse_graft_yaml_str(yaml, "test.yaml");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("nonexistent"),
            "Error should mention the missing step: {err}"
        );
    }

    #[test]
    fn sequence_with_on_step_fail_parses_correctly() {
        let yaml = r#"
apiVersion: graft/v0
commands:
  implement:
    run: "bash scripts/implement.sh {slice}"
  verify:
    run: "bash scripts/verify.sh"
  resume:
    run: "bash scripts/resume.sh {slice}"
    reads:
      - session
sequences:
  implement-verified:
    steps:
      - implement
      - verify
    on_step_fail:
      step: verify
      recovery: resume
      max: 3
"#;
        let config = parse_graft_yaml_str(yaml, "test.yaml").unwrap();
        let seq = config.sequences.get("implement-verified").unwrap();
        let osf = seq.on_step_fail.as_ref().unwrap();
        assert_eq!(osf.step, "verify");
        assert_eq!(osf.recovery, "resume");
        assert_eq!(osf.max, 3);
    }

    #[test]
    fn sequence_on_step_fail_with_invalid_step_fails() {
        let yaml = r#"
apiVersion: graft/v0
commands:
  implement:
    run: "bash scripts/implement.sh"
  recovery:
    run: "echo recovery"
sequences:
  test-seq:
    steps:
      - implement
    on_step_fail:
      step: nonexistent-step
      recovery: recovery
"#;
        let result = parse_graft_yaml_str(yaml, "test.yaml");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("nonexistent-step"),
            "Error should mention the invalid step: {err}"
        );
    }

    #[test]
    fn sequence_on_step_fail_with_missing_recovery_command_fails() {
        let yaml = r#"
apiVersion: graft/v0
commands:
  implement:
    run: "bash scripts/implement.sh"
sequences:
  test-seq:
    steps:
      - implement
    on_step_fail:
      step: implement
      recovery: nonexistent-recovery
"#;
        let result = parse_graft_yaml_str(yaml, "test.yaml");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("nonexistent-recovery"),
            "Error should mention the missing recovery command: {err}"
        );
    }

    #[test]
    fn parses_command_with_stdin_template_engine_none() {
        let yaml = r#"
apiVersion: graft/v0
commands:
  gen:
    run: "cat"
    stdin:
      template: "raw.txt"
      engine: "none"
"#;
        let config = parse_graft_yaml_str(yaml, "test.yaml").unwrap();
        let cmd = config.get_command("gen").unwrap();
        match &cmd.stdin {
            Some(StdinSource::Template { path, engine }) => {
                assert_eq!(path, "raw.txt");
                assert_eq!(*engine, Some("none".to_string()));
            }
            other => panic!("expected Template with engine none, got: {other:?}"),
        }
    }

    // ── scions parsing tests ──────────────────────────────────────────────────

    #[test]
    fn scions_section_absent_gives_none() {
        let yaml = r"
apiVersion: graft/v0
";
        let config = parse_graft_yaml_str(yaml, "test.yaml").unwrap();
        assert!(config.scion_hooks.is_none());
    }

    #[test]
    fn scions_single_command_normalizes_to_vec() {
        let yaml = r#"
apiVersion: graft/v0
commands:
  setup:
    run: "echo setup"
scions:
  on_create: setup
"#;
        let config = parse_graft_yaml_str(yaml, "test.yaml").unwrap();
        let hooks = config.scion_hooks.as_ref().unwrap();
        assert_eq!(hooks.on_create, Some(vec!["setup".to_string()]));
        assert!(hooks.pre_fuse.is_none());
        assert!(hooks.post_fuse.is_none());
        assert!(hooks.on_prune.is_none());
    }

    #[test]
    fn scions_list_of_commands() {
        let yaml = r#"
apiVersion: graft/v0
commands:
  install:
    run: "npm install"
  seed:
    run: "npm run seed"
scions:
  on_create:
    - install
    - seed
"#;
        let config = parse_graft_yaml_str(yaml, "test.yaml").unwrap();
        let hooks = config.scion_hooks.as_ref().unwrap();
        assert_eq!(
            hooks.on_create,
            Some(vec!["install".to_string(), "seed".to_string()])
        );
    }

    #[test]
    fn scions_all_hook_points() {
        let yaml = r#"
apiVersion: graft/v0
commands:
  setup:
    run: "echo setup"
  test:
    run: "echo test"
  notify:
    run: "echo notify"
  cleanup:
    run: "echo cleanup"
scions:
  on_create: setup
  pre_fuse: test
  post_fuse: notify
  on_prune: cleanup
"#;
        let config = parse_graft_yaml_str(yaml, "test.yaml").unwrap();
        let hooks = config.scion_hooks.as_ref().unwrap();
        assert_eq!(hooks.on_create, Some(vec!["setup".to_string()]));
        assert_eq!(hooks.pre_fuse, Some(vec!["test".to_string()]));
        assert_eq!(hooks.post_fuse, Some(vec!["notify".to_string()]));
        assert_eq!(hooks.on_prune, Some(vec!["cleanup".to_string()]));
    }

    #[test]
    fn scions_invalid_hook_name_rejected() {
        let yaml = r#"
apiVersion: graft/v0
scions:
  on_deploy: something
"#;
        let result = parse_graft_yaml_str(yaml, "test.yaml");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("unknown hook point"));
    }

    #[test]
    fn scions_hook_referencing_nonexistent_command_fails() {
        let yaml = r#"
apiVersion: graft/v0
scions:
  on_create: nonexistent-command
"#;
        let result = parse_graft_yaml_str(yaml, "test.yaml");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("nonexistent-command"),
            "Error should mention the bad command name: {err}"
        );
    }

    #[test]
    fn scions_hook_referencing_valid_command_passes() {
        let yaml = r#"
apiVersion: graft/v0
commands:
  setup:
    run: "echo setup"
scions:
  on_create: setup
"#;
        let config = parse_graft_yaml_str(yaml, "test.yaml").unwrap();
        assert!(config.scion_hooks.is_some());
    }

    #[test]
    fn scions_start_parses_correctly() {
        let yaml = r#"
apiVersion: graft/v0
commands:
  agent:
    run: "claude --model opus"
scions:
  start: agent
"#;
        let config = parse_graft_yaml_str(yaml, "test.yaml").unwrap();
        let hooks = config.scion_hooks.unwrap();
        assert_eq!(hooks.start.as_deref(), Some("agent"));
        assert!(hooks.on_create.is_none());
    }

    #[test]
    fn scions_start_nonexistent_command_fails() {
        let yaml = r#"
apiVersion: graft/v0
scions:
  start: nonexistent-worker
"#;
        let result = parse_graft_yaml_str(yaml, "test.yaml");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("nonexistent-worker"),
            "Error should mention the bad command name: {err}"
        );
    }

    #[test]
    fn scions_start_rejects_list_value() {
        let yaml = r#"
apiVersion: graft/v0
commands:
  a:
    run: "echo a"
  b:
    run: "echo b"
scions:
  start:
    - a
    - b
"#;
        let result = parse_graft_yaml_str(yaml, "test.yaml");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("must be a string"),
            "Error should say start must be a string: {err}"
        );
    }

    #[test]
    fn scions_source_parses_correctly() {
        let yaml = r#"
apiVersion: graft/v0
state:
  slices:
    run: "echo slices"
scions:
  source: slices
"#;
        let config = parse_graft_yaml_str(yaml, "test.yaml").unwrap();
        let hooks = config.scion_hooks.unwrap();
        assert_eq!(hooks.source.as_deref(), Some("slices"));
    }

    #[test]
    fn scions_source_unknown_query_accepted() {
        // source is not validated against local state section because it may
        // reference a query from a dependency config
        let yaml = r#"
apiVersion: graft/v0
scions:
  source: nonexistent
"#;
        let config = parse_graft_yaml_str(yaml, "test.yaml").unwrap();
        let hooks = config.scion_hooks.unwrap();
        assert_eq!(hooks.source.as_deref(), Some("nonexistent"));
    }

    #[test]
    fn scions_source_rejects_list_value() {
        let yaml = r#"
apiVersion: graft/v0
state:
  slices:
    run: "echo slices"
scions:
  source:
    - slices
"#;
        let result = parse_graft_yaml_str(yaml, "test.yaml");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("must be a string"),
            "Error should say source must be a string: {err}"
        );
    }

    // ── YAML value coercion / data-loss prevention tests ────────────────────

    #[test]
    fn env_integer_values_coerced_to_string() {
        let yaml = r#"
apiVersion: graft/v0
commands:
  serve:
    run: "echo ok"
    env:
      PORT: 8080
      DEBUG: true
      NAME: "app"
"#;
        let config = parse_graft_yaml_str(yaml, "test.yaml").unwrap();
        let cmd = config.get_command("serve").unwrap();
        let env = cmd.env.as_ref().unwrap();
        assert_eq!(env.get("PORT").unwrap(), "8080");
        assert_eq!(env.get("DEBUG").unwrap(), "true");
        assert_eq!(env.get("NAME").unwrap(), "app");
    }

    #[test]
    fn env_sequence_value_errors() {
        let yaml = r#"
apiVersion: graft/v0
commands:
  serve:
    run: "echo ok"
    env:
      PATHS:
        - /usr/bin
        - /usr/local/bin
"#;
        let result = parse_graft_yaml_str(yaml, "test.yaml");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("sequence"),
            "Error should mention the type: {err}"
        );
    }

    #[test]
    fn context_non_string_errors() {
        let yaml = r#"
apiVersion: graft/v0
commands:
  gen:
    run: "echo ok"
    context:
      - 42
state:
  coverage:
    run: "echo 87.5"
"#;
        let result = parse_graft_yaml_str(yaml, "test.yaml");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("number"),
            "Error should mention the type: {err}"
        );
    }

    #[test]
    fn inputs_non_string_errors() {
        let yaml = r#"
apiVersion: graft/v0
state:
  verify:
    run: "cargo test"
    cache:
      inputs:
        - 42
"#;
        let result = parse_graft_yaml_str(yaml, "test.yaml");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("number"),
            "Error should mention the type: {err}"
        );
    }

    #[test]
    fn hook_commands_non_string_errors() {
        let yaml = r#"
apiVersion: graft/v0
commands:
  setup:
    run: "echo setup"
scions:
  on_create:
    - setup
    - 123
"#;
        let result = parse_graft_yaml_str(yaml, "test.yaml");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("number"),
            "Error should mention the type: {err}"
        );
    }
}
