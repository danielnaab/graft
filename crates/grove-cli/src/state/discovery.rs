//! Discover state queries from graft.yaml files.
use super::query::StateQuery;
use std::path::Path;

/// Discover state queries defined in a graft.yaml file.
pub fn discover_state_queries(graft_yaml_path: &Path) -> Result<Vec<StateQuery>, String> {
    // Use shared parser from graft-common
    let queries_map = graft_common::parse_state_queries(graft_yaml_path)?;

    // Convert HashMap to Vec of StateQuery
    let queries = queries_map
        .into_iter()
        .map(|(name, def)| StateQuery {
            name,
            run: def.run,
            description: def.description,
            deterministic: def.deterministic,
            timeout: def.timeout,
        })
        .collect();

    Ok(queries)
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
