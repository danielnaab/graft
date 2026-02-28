//! Discover state queries from graft.yaml files.
#![allow(dead_code)]
use super::query::StateQuery;
use std::path::Path;

/// Discover state queries defined in a graft.yaml file.
///
/// All returned queries have `working_dir` set to the supplied `consumer_root`.
/// Callers are responsible for resolving dep-relative script paths before calling
/// this function (see [`discover_all_state_queries`]).
pub fn discover_state_queries(
    graft_yaml_path: &Path,
    consumer_root: &Path,
) -> Result<Vec<StateQuery>, String> {
    // Use shared parser from graft-common
    let queries_map = graft_common::parse_state_queries(graft_yaml_path)?;

    // Convert HashMap to Vec of StateQuery
    let queries = queries_map
        .into_iter()
        .map(|(name, def)| StateQuery {
            name,
            run: def.run,
            description: def.description,
            inputs: def.inputs,
            timeout: def.timeout,
            working_dir: consumer_root.to_path_buf(),
            entity: def.entity,
        })
        .collect();

    Ok(queries)
}

/// Discover all state queries for a repository: local (root `graft.yaml`) and all
/// dependency `graft.yaml` files under `.graft/*/graft.yaml`.
///
/// Returns `(queries, warnings)`. Warnings are human-readable strings describing
/// files that could not be parsed; callers should surface them to the user.
///
/// All `StateQuery` values have `working_dir = repo_base` (the consumer root).
/// For dep queries the `run` field has relative script paths rewritten to absolute
/// paths inside the dep directory, mirroring how `graft-engine` resolves context
/// state queries (see `resolve_state_queries` in graft-engine/src/command.rs).
pub fn discover_all_state_queries(repo_base: &Path) -> (Vec<StateQuery>, Vec<String>) {
    let mut queries = Vec::new();
    let mut warnings = Vec::new();

    // Root graft.yaml: working dir is the repo root, no path rewriting needed
    let graft_yaml = repo_base.join("graft.yaml");
    match discover_state_queries(&graft_yaml, repo_base) {
        Ok(local) => queries.extend(local),
        Err(e) => warnings.push(format!("Failed to discover state queries: {e}")),
    }

    // Dep graft.yamls: working dir is still the consumer root, but relative script
    // paths in `run:` are rewritten to absolute paths inside the dep directory so
    // that they can be found when the subprocess CWD is the consumer root.
    let graft_dir = repo_base.join(".graft");
    if let Ok(entries) = std::fs::read_dir(&graft_dir) {
        for entry in entries.flatten() {
            let dep_name = entry.file_name().to_string_lossy().to_string();
            if !entry.file_type().is_ok_and(|ft| ft.is_dir()) {
                continue;
            }
            if dep_name == "run-state" || dep_name == "runs" {
                continue;
            }
            let dep_dir = graft_dir.join(&dep_name);
            let dep_yaml = dep_dir.join("graft.yaml");
            match discover_state_queries(&dep_yaml, repo_base) {
                Ok(mut dep_queries) => {
                    for q in &mut dep_queries {
                        // Rewrite e.g. "bash scripts/foo.sh" to
                        // "bash /abs/.graft/<dep>/scripts/foo.sh" if the script
                        // exists in the dep directory.
                        q.run = graft_engine::resolve_script_in_command(&q.run, &dep_dir);
                    }
                    queries.extend(dep_queries);
                }
                Err(e) => {
                    warnings.push(format!("Failed to parse {dep_name}/graft.yaml: {e}"));
                }
            }
        }
    }

    (queries, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::{tempdir, NamedTempFile};

    // ===== discover_state_queries tests =====

    #[test]
    fn test_discover_state_queries_from_yaml() {
        let yaml_content = r#"
apiVersion: graft/v0
state:
  coverage:
    run: "pytest --cov"
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

        let consumer_root = Path::new("/tmp");
        let queries = discover_state_queries(temp_file.path(), consumer_root).unwrap();

        assert_eq!(queries.len(), 2);

        let coverage = queries.iter().find(|q| q.name == "coverage").unwrap();
        assert_eq!(coverage.run, "pytest --cov");
        let inputs = coverage.inputs.as_ref().unwrap();
        assert_eq!(inputs, &["**/*.py", "pyproject.toml"]);
        assert_eq!(coverage.timeout, Some(60));
        assert_eq!(coverage.working_dir, consumer_root);

        let tasks = queries.iter().find(|q| q.name == "tasks").unwrap();
        assert_eq!(tasks.run, "task-tracker status");
        assert!(tasks.inputs.is_none()); // no inputs → never cached
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

        let queries = discover_state_queries(temp_file.path(), Path::new("/tmp")).unwrap();
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

        let queries = discover_state_queries(temp_file.path(), Path::new("/tmp")).unwrap();
        assert_eq!(queries.len(), 1);
        assert!(queries[0].inputs.is_none()); // No cache → always run fresh
        assert_eq!(queries[0].timeout, None);
    }

    // ===== discover_all_state_queries tests =====

    fn write_graft_yaml(dir: &Path, content: &str) {
        fs::write(dir.join("graft.yaml"), content).unwrap();
    }

    fn make_dep(repo: &Path, dep_name: &str, yaml: &str) -> std::path::PathBuf {
        let dep_dir = repo.join(".graft").join(dep_name);
        fs::create_dir_all(&dep_dir).unwrap();
        write_graft_yaml(&dep_dir, yaml);
        dep_dir
    }

    #[test]
    fn test_discover_all_finds_dep_queries() {
        let repo = tempdir().unwrap();
        write_graft_yaml(repo.path(), "commands:\n  test:\n    run: echo\n");
        make_dep(
            repo.path(),
            "myfactory",
            "state:\n  slices:\n    run: \"bash scripts/list-slices.sh\"\n    timeout: 10\n",
        );

        let (queries, warnings) = discover_all_state_queries(repo.path());

        assert!(warnings.is_empty());
        assert_eq!(queries.len(), 1);
        assert_eq!(queries[0].name, "slices");
        assert_eq!(queries[0].timeout, Some(10));
    }

    #[test]
    fn test_discover_all_dep_working_dir_is_consumer_root() {
        let repo = tempdir().unwrap();
        write_graft_yaml(repo.path(), "commands: {}\n");
        make_dep(repo.path(), "dep", "state:\n  q:\n    run: echo test\n");

        let (queries, _) = discover_all_state_queries(repo.path());

        assert_eq!(queries.len(), 1);
        // working_dir must be the consumer root, not the dep dir
        assert_eq!(queries[0].working_dir, repo.path());
    }

    #[test]
    fn test_discover_all_resolves_dep_script_path_to_absolute() {
        let repo = tempdir().unwrap();
        write_graft_yaml(repo.path(), "commands: {}\n");
        let dep_dir = make_dep(
            repo.path(),
            "factory",
            "state:\n  slices:\n    run: \"bash scripts/list-slices.sh\"\n",
        );

        // Create the script so resolve_script_in_command can find it
        let scripts_dir = dep_dir.join("scripts");
        fs::create_dir_all(&scripts_dir).unwrap();
        fs::write(
            scripts_dir.join("list-slices.sh"),
            "#!/bin/bash\necho '{\"slices\":[]}'",
        )
        .unwrap();

        let (queries, _) = discover_all_state_queries(repo.path());

        assert_eq!(queries.len(), 1);
        let expected_abs = dep_dir.join("scripts").join("list-slices.sh");
        // run should now contain the absolute path to the script
        assert!(
            queries[0].run.contains(expected_abs.to_str().unwrap()),
            "expected absolute path in run, got: {}",
            queries[0].run
        );
    }

    #[test]
    fn test_discover_all_unresolvable_script_unchanged() {
        // If the script does NOT exist in the dep dir, run is left unchanged
        let repo = tempdir().unwrap();
        write_graft_yaml(repo.path(), "commands: {}\n");
        make_dep(
            repo.path(),
            "factory",
            "state:\n  q:\n    run: \"bash scripts/missing.sh\"\n",
        );
        // Note: scripts/missing.sh is NOT created

        let (queries, _) = discover_all_state_queries(repo.path());

        assert_eq!(queries.len(), 1);
        // Script not found → run unchanged
        assert_eq!(queries[0].run, "bash scripts/missing.sh");
    }

    #[test]
    fn test_discover_all_skips_run_state_and_runs() {
        let repo = tempdir().unwrap();
        write_graft_yaml(repo.path(), "commands: {}\n");
        for name in &["run-state", "runs"] {
            make_dep(repo.path(), name, "state:\n  q:\n    run: echo\n");
        }

        let (queries, _) = discover_all_state_queries(repo.path());
        assert!(
            queries.is_empty(),
            "run-state and runs dirs must be skipped"
        );
    }

    #[test]
    fn test_discover_all_surfaces_dep_parse_error() {
        let repo = tempdir().unwrap();
        write_graft_yaml(repo.path(), "commands: {}\n");
        make_dep(repo.path(), "broken", "invalid: [ yaml syntax\n");

        let (queries, warnings) = discover_all_state_queries(repo.path());

        assert!(queries.is_empty());
        assert_eq!(warnings.len(), 1);
        assert!(
            warnings[0].contains("broken"),
            "warning should name the dep, got: {}",
            warnings[0]
        );
    }

    #[test]
    fn test_discover_all_local_and_dep_combined() {
        let repo = tempdir().unwrap();
        write_graft_yaml(
            repo.path(),
            "state:\n  local-query:\n    run: \"echo local\"\n",
        );
        make_dep(
            repo.path(),
            "dep",
            "state:\n  dep-query:\n    run: \"echo dep\"\n",
        );

        let (queries, warnings) = discover_all_state_queries(repo.path());

        assert!(warnings.is_empty());
        assert_eq!(queries.len(), 2);
        let names: Vec<&str> = queries.iter().map(|q| q.name.as_str()).collect();
        assert!(names.contains(&"local-query"));
        assert!(names.contains(&"dep-query"));
        // Both have consumer root as working_dir
        for q in &queries {
            assert_eq!(q.working_dir, repo.path());
        }
    }
}
