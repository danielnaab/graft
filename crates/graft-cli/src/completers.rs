//! Dynamic shell completers for graft CLI arguments.
//!
//! Each function reads config/lock files at tab-press time to provide
//! live completion candidates.

use clap_complete::engine::CompletionCandidate;
use std::path::{Path, PathBuf};

/// Find graft.yaml by searching from `start` up through parent directories.
fn find_graft_yaml_from(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();

    loop {
        let candidate = current.join("graft.yaml");
        if candidate.is_file() {
            return Some(candidate);
        }

        if !current.pop() {
            break;
        }
    }

    None
}

/// Collect dependency name candidates from a given directory.
fn dep_names_in_dir(dir: &Path, prefix: &str) -> Vec<CompletionCandidate> {
    let mut names = std::collections::BTreeSet::new();

    // Collect from graft.yaml
    if let Some(config_path) = find_graft_yaml_from(dir) {
        if let Ok(config) = graft_engine::parse_graft_yaml(&config_path) {
            for name in config.dependencies.keys() {
                names.insert(name.clone());
            }
        }
    }

    // Collect from graft.lock
    let lock_path = dir.join("graft.lock");
    if let Ok(lock_file) = graft_engine::parse_lock_file(&lock_path) {
        for name in lock_file.dependencies.keys() {
            names.insert(name.clone());
        }
    }

    names
        .into_iter()
        .filter(|name| name.starts_with(prefix))
        .map(CompletionCandidate::new)
        .collect()
}

/// Complete dependency names from graft.yaml + graft.lock (union, deduped).
///
/// Used by: status, changes, fetch, sync, apply, upgrade, remove.
pub fn complete_dep_names(current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    let prefix = current.to_string_lossy();
    let Ok(cwd) = std::env::current_dir() else {
        return Vec::new();
    };
    dep_names_in_dir(&cwd, &prefix)
}

/// Collect run command candidates from a given directory.
fn run_commands_in_dir(dir: &Path, prefix: &str) -> Vec<CompletionCandidate> {
    let mut candidates = Vec::new();

    if let Some(config_path) = find_graft_yaml_from(dir) {
        if let Ok(config) = graft_engine::parse_graft_yaml(&config_path) {
            // Local commands
            for (name, cmd) in &config.commands {
                if name.starts_with(prefix) {
                    let mut candidate = CompletionCandidate::new(name);
                    if let Some(desc) = &cmd.description {
                        candidate = candidate.help(Some(desc.into()));
                    }
                    candidates.push(candidate);
                }
            }

            // Dependency commands in dep:command format
            let config_dir = config_path.parent().unwrap_or(Path::new("."));
            for dep_name in config.dependencies.keys() {
                let dep_config_path = config_dir.join(".graft").join(dep_name).join("graft.yaml");
                if let Ok(dep_config) = graft_engine::parse_graft_yaml(&dep_config_path) {
                    for (cmd_name, cmd) in &dep_config.commands {
                        let qualified = format!("{dep_name}:{cmd_name}");
                        if qualified.starts_with(prefix) {
                            let mut candidate = CompletionCandidate::new(qualified);
                            if let Some(desc) = &cmd.description {
                                candidate = candidate.help(Some(desc.into()));
                            }
                            candidates.push(candidate);
                        }
                    }
                }
            }
        }
    }

    candidates
}

/// Complete command names from graft.yaml (local + dep:command format).
///
/// Used by: run.
pub fn complete_run_commands(current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    let prefix = current.to_string_lossy();
    let Ok(cwd) = std::env::current_dir() else {
        return Vec::new();
    };
    run_commands_in_dir(&cwd, &prefix)
}

/// Collect state query name candidates from a given directory.
fn state_names_in_dir(dir: &Path, prefix: &str) -> Vec<CompletionCandidate> {
    if let Some(config_path) = find_graft_yaml_from(dir) {
        if let Ok(config) = graft_engine::parse_graft_yaml(&config_path) {
            return config
                .state
                .keys()
                .filter(|name| name.starts_with(prefix))
                .map(CompletionCandidate::new)
                .collect();
        }
    }

    Vec::new()
}

/// Complete state query names from graft.yaml.
///
/// Used by: state query, state invalidate.
pub fn complete_state_names(current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    let prefix = current.to_string_lossy();
    let Ok(cwd) = std::env::current_dir() else {
        return Vec::new();
    };
    state_names_in_dir(&cwd, &prefix)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    /// Helper to extract completion candidate values as strings.
    fn candidate_names(candidates: &[CompletionCandidate]) -> Vec<String> {
        candidates
            .iter()
            .map(|c| c.get_value().to_string_lossy().to_string())
            .collect()
    }

    #[test]
    fn test_complete_dep_names_empty_dir() {
        let dir = tempdir().unwrap();
        let results = dep_names_in_dir(dir.path(), "");
        assert!(results.is_empty());
    }

    #[test]
    fn test_complete_dep_names_from_config() {
        let dir = tempdir().unwrap();

        std::fs::write(
            dir.path().join("graft.yaml"),
            r#"
apiVersion: graft/v0
deps:
  alpha: "https://example.com/alpha.git#main"
  beta: "https://example.com/beta.git#main"
"#,
        )
        .unwrap();

        let results = dep_names_in_dir(dir.path(), "");
        let names = candidate_names(&results);
        assert!(names.contains(&"alpha".to_string()));
        assert!(names.contains(&"beta".to_string()));
    }

    #[test]
    fn test_complete_dep_names_prefix_filter() {
        let dir = tempdir().unwrap();

        std::fs::write(
            dir.path().join("graft.yaml"),
            r#"
apiVersion: graft/v0
deps:
  alpha: "https://example.com/alpha.git#main"
  beta: "https://example.com/beta.git#main"
"#,
        )
        .unwrap();

        let results = dep_names_in_dir(dir.path(), "al");
        let names = candidate_names(&results);
        assert_eq!(names, vec!["alpha"]);
    }

    #[test]
    fn test_complete_run_commands_empty_dir() {
        let dir = tempdir().unwrap();
        let results = run_commands_in_dir(dir.path(), "");
        assert!(results.is_empty());
    }

    #[test]
    fn test_complete_run_commands_from_config() {
        let dir = tempdir().unwrap();

        std::fs::write(
            dir.path().join("graft.yaml"),
            r#"
apiVersion: graft/v0
commands:
  test:
    run: "cargo test"
    description: "Run tests"
  build:
    run: "cargo build"
"#,
        )
        .unwrap();

        let results = run_commands_in_dir(dir.path(), "");
        let names = candidate_names(&results);
        assert!(names.contains(&"test".to_string()));
        assert!(names.contains(&"build".to_string()));
    }

    #[test]
    fn test_complete_state_names_empty_dir() {
        let dir = tempdir().unwrap();
        let results = state_names_in_dir(dir.path(), "");
        assert!(results.is_empty());
    }

    #[test]
    fn test_complete_state_names_from_config() {
        let dir = tempdir().unwrap();

        std::fs::write(
            dir.path().join("graft.yaml"),
            r#"
apiVersion: graft/v0
state:
  deps-status:
    run: "graft status --format json"
    cache:
      deterministic: true
  repo-info:
    run: "git log --oneline -5"
    cache:
      deterministic: false
"#,
        )
        .unwrap();

        let results = state_names_in_dir(dir.path(), "");
        let names = candidate_names(&results);
        assert!(names.contains(&"deps-status".to_string()));
        assert!(names.contains(&"repo-info".to_string()));
    }

    #[test]
    fn test_complete_state_names_prefix_filter() {
        let dir = tempdir().unwrap();

        std::fs::write(
            dir.path().join("graft.yaml"),
            r#"
apiVersion: graft/v0
state:
  deps-status:
    run: "graft status --format json"
    cache:
      deterministic: true
  repo-info:
    run: "git log --oneline -5"
    cache:
      deterministic: false
"#,
        )
        .unwrap();

        let results = state_names_in_dir(dir.path(), "deps");
        let names = candidate_names(&results);
        assert_eq!(names, vec!["deps-status"]);
    }

    #[test]
    fn test_complete_run_commands_with_help_text() {
        let dir = tempdir().unwrap();

        std::fs::write(
            dir.path().join("graft.yaml"),
            r#"
apiVersion: graft/v0
commands:
  test:
    run: "cargo test"
    description: "Run the test suite"
"#,
        )
        .unwrap();

        let results = run_commands_in_dir(dir.path(), "");
        assert_eq!(results.len(), 1);

        let help = results[0].get_help();
        assert!(help.is_some());
        assert_eq!(help.unwrap().to_string(), "Run the test suite".to_string());
    }
}
