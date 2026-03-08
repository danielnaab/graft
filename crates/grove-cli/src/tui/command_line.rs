//! Command parsing and command palette definitions.
//!
//! This module contains the standalone command parsing logic and palette
//! definitions. Key handling has moved to `prompt.rs`.

// ===== Command palette registry =====

/// A single entry in the command palette.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PaletteEntry {
    /// The command name to type (e.g. `"help"` -> fills `:help`).
    pub(super) command: &'static str,
    /// Short human-readable description shown in the palette.
    pub(super) description: &'static str,
    /// Whether this command requires additional arguments after the name.
    pub(super) takes_args: bool,
}

/// All known commands, in alphabetical display order.
pub(super) const PALETTE_COMMANDS: &[PaletteEntry] = &[
    PaletteEntry {
        command: "attach",
        description: "Attach to a scion's runtime session",
        takes_args: true,
    },
    PaletteEntry {
        command: "catalog",
        description: "List available commands and sequences",
        takes_args: false,
    },
    PaletteEntry {
        command: "focus",
        description: "Set or list focused entity per query",
        takes_args: false,
    },
    PaletteEntry {
        command: "help",
        description: "Show keybindings and command reference",
        takes_args: false,
    },
    PaletteEntry {
        command: "invalidate",
        description: "Clear cached state for current repository",
        takes_args: false,
    },
    PaletteEntry {
        command: "quit",
        description: "Quit Grove",
        takes_args: false,
    },
    PaletteEntry {
        command: "refresh",
        description: "Refresh all repository statuses",
        takes_args: false,
    },
    PaletteEntry {
        command: "repo",
        description: "Jump to a repository by name or index",
        takes_args: true,
    },
    PaletteEntry {
        command: "repos",
        description: "Show all repositories",
        takes_args: false,
    },
    PaletteEntry {
        command: "review",
        description: "Review a scion's changes",
        takes_args: true,
    },
    PaletteEntry {
        command: "run",
        description: "Run a graft command in the current repository",
        takes_args: true,
    },
    PaletteEntry {
        command: "scion",
        description: "Manage scion workstreams",
        takes_args: true,
    },
    PaletteEntry {
        command: "state",
        description: "Show cached state queries",
        takes_args: false,
    },
    PaletteEntry {
        command: "status",
        description: "Show file changes and recent commits",
        takes_args: false,
    },
    PaletteEntry {
        command: "unfocus",
        description: "Clear focused entity (one query or all)",
        takes_args: false,
    },
];

/// Return the subset of `PALETTE_COMMANDS` whose `command` field starts with `filter`
/// as a case-insensitive prefix. Preserves the original display order.
pub(super) fn filtered_palette(filter: &str) -> Vec<&'static PaletteEntry> {
    let filter = filter.to_ascii_lowercase();
    PALETTE_COMMANDS
        .iter()
        .filter(|e| e.command.starts_with(filter.as_str()))
        .collect()
}

// ===== Command parsing =====

/// A parsed command from the `:` command line.
#[derive(Debug, PartialEq, Eq, Clone)]
pub(super) enum CliCommand {
    /// `:help` — show help reference.
    Help,
    /// `:quit` or `:q` — exit.
    Quit,
    /// `:refresh` — trigger a repo refresh.
    Refresh,
    /// `:repo <name-or-index>` — switch active repository.
    Repo(String),
    /// `:repos` — show all repositories.
    Repos,
    /// `:run <cmd> [args]` — execute a graft command by name, with optional args.
    Run(String, Vec<String>),
    /// `:status` or `:st` — show file changes and recent commits.
    Status,
    /// `:catalog [category]` — list available commands and sequences.
    Catalog(Option<String>),
    /// `:state [name]` — show cached state queries (or detail for a named query).
    State(Option<String>),
    /// `:invalidate [name]` — clear cached state (all or a single query).
    Invalidate(Option<String>),
    /// `:focus [query [value]]` — list, pick, or set a focused entity.
    ///
    /// - `Focus(None, None)` — list all focusable queries and current values.
    /// - `Focus(Some(query), None)` — open a picker over that query's entities.
    /// - `Focus(Some(query), Some(value))` — set focus directly (no picker).
    Focus(Option<String>, Option<String>),
    /// `:unfocus [query]` — clear focus for one query or all queries.
    ///
    /// - `Unfocus(None)` — clear all focuses.
    /// - `Unfocus(Some(query))` — clear focus for a single query.
    Unfocus(Option<String>),
    /// `:scion list` — list all scion workstreams.
    ScionList,
    /// `:scion create <name>` — create a new scion.
    ScionCreate(String),
    /// `:scion start <name>` — start a scion's runtime session.
    ScionStart(String),
    /// `:scion stop <name>` — stop a scion's runtime session.
    ScionStop(String),
    /// `:scion prune <name>` — remove a scion.
    ScionPrune(String),
    /// `:scion fuse <name>` — fuse a scion into main.
    ScionFuse(String),
    /// `:scion run <name>` — create if needed and start (combined workflow).
    ScionRun(String),
    /// `:attach <name>` — attach to a scion's runtime session.
    Attach(String),
    /// `:review <name> [full]` — review a scion's changes.
    Review(String, bool),
    /// Execute a single state query by name.
    StateRun(String),
    /// Pre-populate the prompt with text (used by catalog for commands needing args).
    PopulatePrompt(String),
    /// An unknown command (the raw input is preserved for error display).
    Unknown(String),
}

/// Parse a command line buffer (without the leading `:`) into a `CliCommand`.
#[allow(clippy::too_many_lines)]
pub(super) fn parse_command(input: &str) -> CliCommand {
    let input = input.trim();

    if input.is_empty() {
        return CliCommand::Unknown(String::new());
    }

    let mut parts = input.splitn(2, char::is_whitespace);
    let cmd = parts.next().unwrap_or("").to_ascii_lowercase();
    let rest = parts.next().unwrap_or("").trim();

    match cmd.as_str() {
        "help" | "h" => CliCommand::Help,
        "quit" | "q" => CliCommand::Quit,
        "refresh" | "r" => CliCommand::Refresh,
        "repos" => CliCommand::Repos,
        "repo" => {
            if rest.is_empty() {
                CliCommand::Unknown(input.to_string())
            } else {
                CliCommand::Repo(rest.to_string())
            }
        }
        "run" => {
            if rest.is_empty() {
                CliCommand::Unknown(input.to_string())
            } else {
                let mut words = rest.splitn(2, char::is_whitespace);
                let command_name = words.next().unwrap_or("").to_string();
                let args_str = words.next().unwrap_or("").trim();
                let args = if args_str.is_empty() {
                    Vec::new()
                } else {
                    shell_words::split(args_str).unwrap_or_else(|_| {
                        args_str.split_whitespace().map(str::to_string).collect()
                    })
                };
                CliCommand::Run(command_name, args)
            }
        }
        "status" | "st" => CliCommand::Status,
        "catalog" | "cat" => {
            if rest.is_empty() {
                CliCommand::Catalog(None)
            } else {
                CliCommand::Catalog(Some(rest.to_string()))
            }
        }
        "state" => {
            if rest.is_empty() {
                CliCommand::State(None)
            } else {
                CliCommand::State(Some(rest.to_string()))
            }
        }
        "invalidate" | "inv" => {
            if rest.is_empty() || rest == "--all" {
                CliCommand::Invalidate(None)
            } else {
                CliCommand::Invalidate(Some(rest.to_string()))
            }
        }
        "focus" | "f" => {
            if rest.is_empty() {
                CliCommand::Focus(None, None)
            } else {
                let mut parts = rest.splitn(2, char::is_whitespace);
                let query = parts.next().unwrap_or("").to_string();
                let value = parts.next().map(str::trim).filter(|s| !s.is_empty());
                CliCommand::Focus(Some(query), value.map(str::to_string))
            }
        }
        "unfocus" | "uf" => {
            if rest.is_empty() {
                CliCommand::Unfocus(None)
            } else {
                CliCommand::Unfocus(Some(rest.to_string()))
            }
        }
        "scion" | "sc" => {
            if rest.is_empty() {
                return CliCommand::ScionList;
            }
            let parts: Vec<&str> = rest.splitn(2, char::is_whitespace).collect();
            match parts.first().map(|s| s.to_ascii_lowercase()).as_deref() {
                Some("list" | "ls") => CliCommand::ScionList,
                Some("create") => {
                    let name = parts.get(1).unwrap_or(&"").trim();
                    if name.is_empty() {
                        CliCommand::Unknown(input.to_string())
                    } else {
                        CliCommand::ScionCreate(name.to_string())
                    }
                }
                Some("start") => {
                    let name = parts.get(1).unwrap_or(&"").trim();
                    if name.is_empty() {
                        CliCommand::Unknown(input.to_string())
                    } else {
                        CliCommand::ScionStart(name.to_string())
                    }
                }
                Some("stop") => {
                    let name = parts.get(1).unwrap_or(&"").trim();
                    if name.is_empty() {
                        CliCommand::Unknown(input.to_string())
                    } else {
                        CliCommand::ScionStop(name.to_string())
                    }
                }
                Some("prune") => {
                    let name = parts.get(1).unwrap_or(&"").trim();
                    if name.is_empty() {
                        CliCommand::Unknown(input.to_string())
                    } else {
                        CliCommand::ScionPrune(name.to_string())
                    }
                }
                Some("fuse") => {
                    let name = parts.get(1).unwrap_or(&"").trim();
                    if name.is_empty() {
                        CliCommand::Unknown(input.to_string())
                    } else {
                        CliCommand::ScionFuse(name.to_string())
                    }
                }
                Some("run") => {
                    let name = parts.get(1).unwrap_or(&"").trim();
                    if name.is_empty() {
                        CliCommand::Unknown(input.to_string())
                    } else {
                        CliCommand::ScionRun(name.to_string())
                    }
                }
                _ => CliCommand::Unknown(input.to_string()),
            }
        }
        "attach" => {
            if rest.is_empty() {
                CliCommand::Unknown(input.to_string())
            } else {
                CliCommand::Attach(rest.to_string())
            }
        }
        "review" => {
            if rest.is_empty() {
                CliCommand::Unknown(input.to_string())
            } else {
                let mut words = rest.splitn(2, char::is_whitespace);
                let name = words.next().unwrap_or("").to_string();
                let full = words
                    .next()
                    .map(str::trim)
                    .is_some_and(|s| s.eq_ignore_ascii_case("full"));
                CliCommand::Review(name, full)
            }
        }
        _ => CliCommand::Unknown(input.to_string()),
    }
}

// ===== Unit tests =====

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_help_command() {
        assert_eq!(parse_command("help"), CliCommand::Help);
        assert_eq!(parse_command("Help"), CliCommand::Help);
        assert_eq!(parse_command("HELP"), CliCommand::Help);
        assert_eq!(parse_command("h"), CliCommand::Help);
    }

    #[test]
    fn parse_quit_command() {
        assert_eq!(parse_command("quit"), CliCommand::Quit);
        assert_eq!(parse_command("q"), CliCommand::Quit);
        assert_eq!(parse_command("Quit"), CliCommand::Quit);
        assert_eq!(parse_command("Q"), CliCommand::Quit);
    }

    #[test]
    fn parse_refresh_command() {
        assert_eq!(parse_command("refresh"), CliCommand::Refresh);
        assert_eq!(parse_command("Refresh"), CliCommand::Refresh);
        assert_eq!(parse_command("r"), CliCommand::Refresh);
    }

    #[test]
    fn parse_state_command() {
        assert_eq!(parse_command("state"), CliCommand::State(None));
        assert_eq!(parse_command("State"), CliCommand::State(None));
    }

    #[test]
    fn parse_state_with_name() {
        assert_eq!(
            parse_command("state coverage"),
            CliCommand::State(Some("coverage".to_string()))
        );
    }

    #[test]
    fn parse_status_command() {
        assert_eq!(parse_command("status"), CliCommand::Status);
        assert_eq!(parse_command("st"), CliCommand::Status);
        assert_eq!(parse_command("Status"), CliCommand::Status);
    }

    #[test]
    fn parse_catalog_no_args() {
        assert_eq!(parse_command("catalog"), CliCommand::Catalog(None));
        assert_eq!(parse_command("cat"), CliCommand::Catalog(None));
    }

    #[test]
    fn parse_catalog_with_category() {
        assert_eq!(
            parse_command("catalog core"),
            CliCommand::Catalog(Some("core".to_string()))
        );
    }

    #[test]
    fn parse_invalidate_no_args() {
        assert_eq!(parse_command("invalidate"), CliCommand::Invalidate(None));
        assert_eq!(parse_command("inv"), CliCommand::Invalidate(None));
    }

    #[test]
    fn parse_invalidate_all() {
        assert_eq!(
            parse_command("invalidate --all"),
            CliCommand::Invalidate(None)
        );
    }

    #[test]
    fn parse_invalidate_with_name() {
        assert_eq!(
            parse_command("invalidate coverage"),
            CliCommand::Invalidate(Some("coverage".to_string()))
        );
    }

    #[test]
    fn parse_repos_command() {
        assert_eq!(parse_command("repos"), CliCommand::Repos);
    }

    #[test]
    fn parse_repo_command_with_name() {
        assert_eq!(
            parse_command("repo graft"),
            CliCommand::Repo("graft".to_string())
        );
        assert_eq!(
            parse_command("repo my-project"),
            CliCommand::Repo("my-project".to_string())
        );
    }

    #[test]
    fn parse_repo_command_with_index() {
        assert_eq!(parse_command("repo 1"), CliCommand::Repo("1".to_string()));
        assert_eq!(parse_command("repo 42"), CliCommand::Repo("42".to_string()));
    }

    #[test]
    fn parse_repo_command_without_name_is_unknown() {
        assert_eq!(
            parse_command("repo"),
            CliCommand::Unknown("repo".to_string())
        );
    }

    #[test]
    fn parse_run_command_with_name_only() {
        assert_eq!(
            parse_command("run test"),
            CliCommand::Run("test".to_string(), vec![])
        );
        assert_eq!(
            parse_command("run build"),
            CliCommand::Run("build".to_string(), vec![])
        );
    }

    #[test]
    fn parse_run_command_with_args() {
        assert_eq!(
            parse_command("run test --verbose"),
            CliCommand::Run("test".to_string(), vec!["--verbose".to_string()])
        );
        assert_eq!(
            parse_command("run deploy --env staging --dry-run"),
            CliCommand::Run(
                "deploy".to_string(),
                vec![
                    "--env".to_string(),
                    "staging".to_string(),
                    "--dry-run".to_string()
                ]
            )
        );
    }

    #[test]
    fn parse_run_command_without_name_is_unknown() {
        assert_eq!(parse_command("run"), CliCommand::Unknown("run".to_string()));
    }

    #[test]
    fn parse_unknown_command() {
        assert_eq!(
            parse_command("frobnicate"),
            CliCommand::Unknown("frobnicate".to_string())
        );
    }

    #[test]
    fn parse_empty_input_is_unknown_empty() {
        assert_eq!(parse_command(""), CliCommand::Unknown(String::new()));
        assert_eq!(parse_command("   "), CliCommand::Unknown(String::new()));
    }

    #[test]
    fn parse_leading_trailing_whitespace_stripped() {
        assert_eq!(parse_command("  help  "), CliCommand::Help);
        assert_eq!(
            parse_command("  repo graft  "),
            CliCommand::Repo("graft".to_string())
        );
    }

    #[test]
    fn parse_run_with_quoted_args() {
        assert_eq!(
            parse_command(r#"run test "arg with spaces""#),
            CliCommand::Run("test".to_string(), vec!["arg with spaces".to_string()])
        );
    }

    #[test]
    fn filtered_palette_all() {
        let all = filtered_palette("");
        assert!(all.len() >= 10);
    }

    #[test]
    fn filtered_palette_matches() {
        let matches = filtered_palette("re");
        assert!(matches.iter().any(|e| e.command == "refresh"));
        assert!(matches.iter().any(|e| e.command == "repo"));
        assert!(matches.iter().any(|e| e.command == "repos"));
    }

    #[test]
    fn filtered_palette_no_match() {
        let matches = filtered_palette("zzz");
        assert!(matches.is_empty());
    }

    // --- scion command parsing ---

    #[test]
    fn parse_scion_list() {
        assert_eq!(parse_command("scion list"), CliCommand::ScionList);
        assert_eq!(parse_command("scion ls"), CliCommand::ScionList);
        assert_eq!(parse_command("sc list"), CliCommand::ScionList);
        assert_eq!(parse_command("scion"), CliCommand::ScionList);
    }

    #[test]
    fn parse_scion_create() {
        assert_eq!(
            parse_command("scion create my-feature"),
            CliCommand::ScionCreate("my-feature".to_string())
        );
    }

    #[test]
    fn parse_scion_create_no_name() {
        assert_eq!(
            parse_command("scion create"),
            CliCommand::Unknown("scion create".to_string())
        );
    }

    #[test]
    fn parse_scion_start_stop() {
        assert_eq!(
            parse_command("scion start worker"),
            CliCommand::ScionStart("worker".to_string())
        );
        assert_eq!(
            parse_command("scion stop worker"),
            CliCommand::ScionStop("worker".to_string())
        );
    }

    #[test]
    fn parse_scion_prune_fuse() {
        assert_eq!(
            parse_command("scion prune old"),
            CliCommand::ScionPrune("old".to_string())
        );
        assert_eq!(
            parse_command("scion fuse done"),
            CliCommand::ScionFuse("done".to_string())
        );
    }

    #[test]
    fn parse_scion_unknown_sub() {
        assert_eq!(
            parse_command("scion bogus arg"),
            CliCommand::Unknown("scion bogus arg".to_string())
        );
    }

    #[test]
    fn parse_attach() {
        assert_eq!(
            parse_command("attach worker"),
            CliCommand::Attach("worker".to_string())
        );
    }

    #[test]
    fn parse_attach_no_name() {
        assert_eq!(
            parse_command("attach"),
            CliCommand::Unknown("attach".to_string())
        );
    }

    #[test]
    fn parse_review() {
        assert_eq!(
            parse_command("review my-feature"),
            CliCommand::Review("my-feature".to_string(), false)
        );
        assert_eq!(
            parse_command("review my-feature full"),
            CliCommand::Review("my-feature".to_string(), true)
        );
    }

    #[test]
    fn parse_review_no_name() {
        assert_eq!(
            parse_command("review"),
            CliCommand::Unknown("review".to_string())
        );
    }
}
