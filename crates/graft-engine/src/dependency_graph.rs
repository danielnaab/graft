//! Pre-computed mapping between state names and the commands that produce or consume them.
//!
//! `DependencyGraph` replaces the ad-hoc linear scan in `setup_run_state` with an O(1) lookup
//! and provides a foundation for parse-time validation and observability.

use crate::domain::GraftConfig;
use std::collections::HashMap;

/// Pre-computed producer/consumer mapping derived from a [`GraftConfig`].
///
/// Built by scanning every command's `writes:` and `reads:` declarations once.
/// Provides O(1) producer/consumer lookups compared to the O(n) scan that was
/// previously embedded inside `setup_run_state`.
///
/// # Validation
///
/// Two commands declaring `writes:` for the same state name is a configuration
/// error and causes [`DependencyGraph::from_config`] to return `Err`.
///
/// A command declaring `reads:` for a state name that no command `writes:` is
/// not an error — the state may be produced by a script outside graft — but
/// callers that want to warn about it can use [`DependencyGraph::producer`] to
/// check for a `None` result.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DependencyGraph {
    /// Maps each state name to the single command that declares it in `writes:`.
    producers: HashMap<String, String>,
    /// Maps each state name to all commands that list it in `reads:`.
    consumers: HashMap<String, Vec<String>>,
}

impl DependencyGraph {
    /// Build a `DependencyGraph` from a [`GraftConfig`].
    ///
    /// Returns `Err` if two commands both declare `writes: [<same-state>]`.
    /// Does **not** warn about reads with no known producer; callers that need
    /// that warning should call [`DependencyGraph::producer`] on each read name.
    pub fn from_config(config: &GraftConfig) -> Result<Self, String> {
        let mut producers: HashMap<String, String> = HashMap::new();
        let mut consumers: HashMap<String, Vec<String>> = HashMap::new();

        let mut sorted_commands: Vec<_> = config.commands.iter().collect();
        sorted_commands.sort_by_key(|(name, _)| name.as_str());

        for (cmd_name, command) in sorted_commands {
            for state_name in &command.writes {
                if let Some(existing) = producers.insert(state_name.clone(), cmd_name.clone()) {
                    return Err(format!(
                        "state '{state_name}' has duplicate producers: \
                         '{existing}' and '{cmd_name}'"
                    ));
                }
            }
            for state_name in &command.reads {
                consumers
                    .entry(state_name.clone())
                    .or_default()
                    .push(cmd_name.clone());
            }
        }

        Ok(Self {
            producers,
            consumers,
        })
    }

    /// Returns the name of the command that produces `state_name`, or `None`.
    pub fn producer(&self, state_name: &str) -> Option<&str> {
        self.producers.get(state_name).map(String::as_str)
    }

    /// Returns the names of all commands that consume `state_name`.
    ///
    /// Returns an empty slice when no command reads the given state.
    pub fn consumers_of(&self, state_name: &str) -> &[String] {
        self.consumers.get(state_name).map_or(&[], Vec::as_slice)
    }

    /// Returns the full producers map (`state_name → command_name`).
    pub fn all_producers(&self) -> &HashMap<String, String> {
        &self.producers
    }

    /// Returns the full consumers map (`state_name → [command_names]`).
    pub fn all_consumers(&self) -> &HashMap<String, Vec<String>> {
        &self.consumers
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Command, GraftConfig};

    fn make_config(commands: &[(&str, &[&str], &[&str])]) -> GraftConfig {
        // commands: (name, writes, reads)
        let mut config = GraftConfig::new("graft/v0").unwrap();
        for (name, writes, reads) in commands {
            let cmd = Command::new(*name, "echo ok")
                .unwrap()
                .with_writes(writes.iter().map(|s| (*s).to_string()).collect())
                .with_reads(reads.iter().map(|s| (*s).to_string()).collect());
            config.commands.insert((*name).to_string(), cmd);
        }
        config
    }

    #[test]
    fn from_config_empty_config_returns_empty_graph() {
        let config = GraftConfig::new("graft/v0").unwrap();
        let graph = DependencyGraph::from_config(&config).unwrap();
        assert!(graph.all_producers().is_empty());
        assert!(graph.all_consumers().is_empty());
    }

    #[test]
    fn from_config_builds_producers_map() {
        let config = make_config(&[
            ("implement", &["session"], &[]),
            ("verify", &["verify"], &[]),
        ]);
        let graph = DependencyGraph::from_config(&config).unwrap();
        assert_eq!(graph.producer("session"), Some("implement"));
        assert_eq!(graph.producer("verify"), Some("verify"));
        assert_eq!(graph.producer("unknown"), None);
    }

    #[test]
    fn from_config_builds_consumers_map() {
        let config = make_config(&[
            ("resume", &[], &["session", "verify"]),
            ("push", &[], &["session"]),
        ]);
        let graph = DependencyGraph::from_config(&config).unwrap();

        let mut session_consumers = graph.consumers_of("session").to_vec();
        session_consumers.sort();
        assert_eq!(session_consumers, vec!["push", "resume"]);

        let verify_consumers = graph.consumers_of("verify");
        assert_eq!(verify_consumers, ["resume"]);
    }

    #[test]
    fn from_config_error_on_duplicate_producers() {
        let config = make_config(&[
            ("implement", &["session"], &[]),
            ("also-writes-session", &["session"], &[]),
        ]);
        let err = DependencyGraph::from_config(&config).unwrap_err();
        assert!(
            err.contains("session"),
            "error should name the conflicting state, got: {err}"
        );
        assert!(
            err.contains("duplicate producers"),
            "error should mention duplicate producers, got: {err}"
        );
    }

    #[test]
    fn from_config_single_producer_no_error() {
        let config = make_config(&[("implement", &["session"], &[])]);
        assert!(DependencyGraph::from_config(&config).is_ok());
    }

    #[test]
    fn producer_returns_none_for_unknown_state() {
        let config = make_config(&[("cmd", &["known"], &[])]);
        let graph = DependencyGraph::from_config(&config).unwrap();
        assert_eq!(graph.producer("unknown"), None);
    }

    #[test]
    fn consumers_of_returns_empty_slice_for_unknown_state() {
        let config = make_config(&[("cmd", &["known"], &[])]);
        let graph = DependencyGraph::from_config(&config).unwrap();
        assert!(graph.consumers_of("unknown").is_empty());
    }

    #[test]
    fn command_that_both_reads_and_writes_appears_in_both_maps() {
        // A command that writes "verify" and reads "session" (e.g. a combined check)
        let config = make_config(&[
            ("implement", &["session"], &[]),
            ("verify", &["verify"], &["session"]),
        ]);
        let graph = DependencyGraph::from_config(&config).unwrap();

        assert_eq!(graph.producer("session"), Some("implement"));
        assert_eq!(graph.producer("verify"), Some("verify"));
        assert_eq!(graph.consumers_of("session"), ["verify"]);
        assert!(graph.consumers_of("verify").is_empty());
    }

    #[test]
    fn multiple_writes_from_one_command() {
        let config = make_config(&[("big-step", &["a", "b", "c"], &[])]);
        let graph = DependencyGraph::from_config(&config).unwrap();
        assert_eq!(graph.producer("a"), Some("big-step"));
        assert_eq!(graph.producer("b"), Some("big-step"));
        assert_eq!(graph.producer("c"), Some("big-step"));
    }

    #[test]
    fn multiple_reads_from_one_command() {
        let config = make_config(&[
            ("step-a", &["x"], &[]),
            ("step-b", &["y"], &[]),
            ("consumer", &[], &["x", "y"]),
        ]);
        let graph = DependencyGraph::from_config(&config).unwrap();
        assert_eq!(graph.consumers_of("x"), ["consumer"]);
        assert_eq!(graph.consumers_of("y"), ["consumer"]);
    }
}
