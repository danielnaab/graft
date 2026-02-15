///! Discover state queries from graft.yaml files.
use super::query::StateQuery;
use serde_yaml::Value;
use std::fs;
use std::path::Path;

/// Discover state queries defined in a graft.yaml file.
pub fn discover_state_queries(graft_yaml_path: &Path) -> Result<Vec<StateQuery>, String> {
    let content = fs::read_to_string(graft_yaml_path)
        .map_err(|e| format!("Failed to read graft.yaml: {}", e))?;

    let yaml: Value =
        serde_yaml::from_str(&content).map_err(|e| format!("Failed to parse graft.yaml: {}", e))?;

    let mut queries = Vec::new();

    // Look for "state:" section in YAML
    if let Some(state_section) = yaml.get("state") {
        if let Some(state_map) = state_section.as_mapping() {
            for (name, config) in state_map {
                if let Some(name_str) = name.as_str() {
                    match parse_state_query(name_str, config) {
                        Ok(query) => queries.push(query),
                        Err(e) => {
                            eprintln!("Warning: Failed to parse state query '{}': {}", name_str, e);
                            continue;
                        }
                    }
                }
            }
        }
    }

    Ok(queries)
}

/// Parse a single state query from YAML config.
fn parse_state_query(name: &str, config: &Value) -> Result<StateQuery, String> {
    // Get run command (required)
    let run = config
        .get("run")
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("State query '{}' missing 'run' field", name))?
        .to_string();

    // Get cache config
    let deterministic = config
        .get("cache")
        .and_then(|c| c.get("deterministic"))
        .and_then(|d| d.as_bool())
        .unwrap_or(true); // Default to deterministic

    // Get timeout
    let timeout = config.get("timeout").and_then(|t| t.as_u64());

    // Get description (optional, from command description if available)
    let description = config
        .get("description")
        .and_then(|d| d.as_str())
        .map(|s| s.to_string());

    Ok(StateQuery {
        name: name.to_string(),
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
    fn test_discover_state_queries_from_yaml() {
        let yaml_content = r#"
apiVersion: graft/v0
state:
  coverage:
    run: "pytest --cov"
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

        let queries = discover_state_queries(temp_file.path()).unwrap();

        assert_eq!(queries.len(), 2);

        let coverage = queries.iter().find(|q| q.name == "coverage").unwrap();
        assert_eq!(coverage.run, "pytest --cov");
        assert_eq!(coverage.deterministic, true);
        assert_eq!(coverage.timeout, Some(60));

        let tasks = queries.iter().find(|q| q.name == "tasks").unwrap();
        assert_eq!(tasks.run, "task-tracker status");
        assert_eq!(tasks.deterministic, false);
        assert_eq!(tasks.timeout, Some(30));
    }

    #[test]
    fn test_discover_no_state_section() {
        let yaml_content = r#"
apiVersion: graft/v0
commands:
  test:
    run: "pytest"
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let queries = discover_state_queries(temp_file.path()).unwrap();
        assert_eq!(queries.len(), 0);
    }

    #[test]
    fn test_discover_handles_missing_cache_field() {
        let yaml_content = r#"
state:
  simple:
    run: "echo test"
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let queries = discover_state_queries(temp_file.path()).unwrap();
        assert_eq!(queries.len(), 1);
        assert_eq!(queries[0].deterministic, true); // Default
        assert_eq!(queries[0].timeout, None);
    }
}
