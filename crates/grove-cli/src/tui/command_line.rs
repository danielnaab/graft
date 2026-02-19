//! Vim-style `:` command line input handling, command parsing, and command palette.

use super::{App, KeyCode, KeyModifiers, RepoDetailProvider, RepoRegistry, StatusMessage, View};

/// Maximum number of command history entries to keep.
const MAX_HISTORY: usize = 50;

// ===== Command palette registry =====

/// A single entry in the command palette.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PaletteEntry {
    /// The command name to type (e.g. `"help"` → fills `:help`).
    pub(super) command: &'static str,
    /// Short human-readable description shown in the palette.
    pub(super) description: &'static str,
    /// Whether this command requires additional arguments after the name.
    ///
    /// `false` → pressing `Enter` on this palette entry executes it immediately
    ///           (no second Enter needed).
    /// `true`  → pressing `Enter` fills the buffer so the user can type args,
    ///           then presses `Enter` again to execute.
    pub(super) takes_args: bool,
}

/// All known commands, in display order.
///
/// This is the canonical list for the palette. Each entry's `command` field
/// corresponds to the `parse_command` keyword that will execute it.
pub(super) const PALETTE_COMMANDS: &[PaletteEntry] = &[
    PaletteEntry {
        command: "help",
        description: "Show keybindings and command reference",
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
        command: "run",
        description: "Run a graft command in the current repository",
        takes_args: true,
    },
    PaletteEntry {
        command: "state",
        description: "Refresh state queries for the current repository",
        takes_args: false,
    },
];

/// Return the subset of `PALETTE_COMMANDS` whose `command` field contains `filter`
/// as a case-insensitive substring. Preserves the original display order.
pub(super) fn filtered_palette(filter: &str) -> Vec<&'static PaletteEntry> {
    let filter = filter.to_ascii_lowercase();
    PALETTE_COMMANDS
        .iter()
        .filter(|e| e.command.contains(filter.as_str()))
        .collect()
}

/// Return the longest common prefix of a set of strings.
///
/// Returns an empty string if the slice is empty.
fn longest_common_prefix(strings: &[&str]) -> String {
    let Some(first) = strings.first() else {
        return String::new();
    };

    let mut prefix_len = first.len();
    for s in &strings[1..] {
        prefix_len = first
            .chars()
            .zip(s.chars())
            .take(prefix_len)
            .take_while(|(a, b)| a == b)
            .count();
        if prefix_len == 0 {
            break;
        }
    }

    first.chars().take(prefix_len).collect()
}

// ===== Command parsing =====

/// A parsed command from the `:` command line.
#[derive(Debug, PartialEq, Eq, Clone)]
pub(super) enum CliCommand {
    /// `:help` — push the Help view.
    Help,
    /// `:quit` or `:q` — set `should_quit`.
    Quit,
    /// `:refresh` — trigger a repo refresh.
    Refresh,
    /// `:repo <name-or-index>` — jump directly to a repo detail view.
    Repo(String),
    /// `:run <cmd> [args]` — execute a graft command by name, with optional args.
    Run(String, Vec<String>),
    /// `:state` — refresh state queries for the current repo.
    State,
    /// An unknown command (the raw input is preserved for error display).
    Unknown(String),
}

/// Parse a command line buffer (without the leading `:`) into a `CliCommand`.
///
/// Parsing rules:
/// - Leading/trailing whitespace is stripped.
/// - The first whitespace-delimited token is the command name (case-insensitive).
/// - Remaining tokens are arguments.
/// - Empty input returns `Unknown("")`.
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
        "refresh" => CliCommand::Refresh,
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
                // Split remaining text shell-style, falling back to whitespace split
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
        "state" => CliCommand::State,
        _ => CliCommand::Unknown(input.to_string()),
    }
}

// ===== Key handling =====

impl<R: RepoRegistry, D: RepoDetailProvider> App<R, D> {
    /// Handle a key press when the command line is active.
    ///
    /// The command line intercepts all keys before view dispatch. `Escape`
    /// cancels; `Enter` either fills from the selected palette entry (when the
    /// palette has a selection and no explicit command is typed) or submits the
    /// buffer as a command; `j`/`k` / Up/Down navigate the palette.
    #[allow(clippy::too_many_lines)]
    pub(super) fn handle_key_command_line(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        let Some(state) = &mut self.command_line else {
            return;
        };

        // Handle Ctrl shortcuts first
        if modifiers.contains(KeyModifiers::CONTROL) {
            match code {
                KeyCode::Char('u') => {
                    state.text.clear();
                    state.palette_selected = 0;
                    return;
                }
                KeyCode::Char('w') => {
                    state.text.delete_word_backward();
                    state.palette_selected = 0;
                    return;
                }
                _ => {}
            }
        }

        match code {
            KeyCode::Esc => {
                // Cancel command line — dismiss without executing.
                self.command_line = None;
            }
            KeyCode::Enter => {
                // When the buffer is empty and a palette entry is highlighted:
                //   - No-arg commands (help, quit, refresh, state): execute immediately.
                //   - Arg commands (repo, run): fill the buffer so the user can type args.
                // When the buffer has content: submit as-is.
                let entries = filtered_palette(&state.text.buffer);
                let selected = state.palette_selected;

                if !entries.is_empty() && selected < entries.len() && state.text.buffer.is_empty() {
                    let entry = entries[selected];
                    if entry.takes_args {
                        // Fill buffer and leave open — user will add arguments, then Enter again.
                        state.text.set(entry.command);
                        return;
                    }
                    // No-arg command: fill buffer, dismiss, and execute in one keystroke.
                    let command = entry.command.to_string();
                    self.command_line = None;
                    // Save to history (skip consecutive duplicates)
                    if self
                        .command_history
                        .last()
                        .is_none_or(|last| *last != command)
                    {
                        self.command_history.push(command.clone());
                        if self.command_history.len() > MAX_HISTORY {
                            self.command_history.remove(0);
                        }
                    }
                    let cmd = parse_command(&command);
                    self.execute_cli_command(cmd);
                    return;
                }

                // Normal submit (buffer has content, or palette is empty / filtered out).
                let buffer = state.text.buffer.clone();
                self.command_line = None;

                if !buffer.is_empty() {
                    // Save to history (skip consecutive duplicates)
                    if self
                        .command_history
                        .last()
                        .is_none_or(|last| *last != buffer)
                    {
                        self.command_history.push(buffer.clone());
                        if self.command_history.len() > MAX_HISTORY {
                            self.command_history.remove(0);
                        }
                    }
                    let cmd = parse_command(&buffer);
                    self.execute_cli_command(cmd);
                }
                // Empty Enter with no palette match dismisses the command line silently.
            }
            // Palette navigation: j/k only navigate when buffer is empty (browsing mode).
            // When buffer has content, j/k fall through to Char(c) for text insertion.
            KeyCode::Char('j') if state.text.buffer.is_empty() => {
                let entries = filtered_palette(&state.text.buffer);
                if !entries.is_empty() {
                    let next = state.palette_selected + 1;
                    state.palette_selected = if next >= entries.len() { 0 } else { next };
                }
            }
            KeyCode::Char('k') if state.text.buffer.is_empty() => {
                let entries = filtered_palette(&state.text.buffer);
                if !entries.is_empty() {
                    state.palette_selected = if state.palette_selected == 0 {
                        entries.len() - 1
                    } else {
                        state.palette_selected - 1
                    };
                }
            }
            // Arrow keys: if currently browsing history, continue history navigation.
            // Otherwise, navigate palette when entries match, or navigate history.
            KeyCode::Down => {
                if state.history_index.is_some() {
                    self.command_line_history_down();
                } else {
                    let entries = filtered_palette(&state.text.buffer);
                    if entries.is_empty() {
                        self.command_line_history_down();
                    } else {
                        let next = state.palette_selected + 1;
                        state.palette_selected = if next >= entries.len() { 0 } else { next };
                    }
                }
            }
            KeyCode::Up => {
                if state.history_index.is_some() {
                    self.command_line_history_up();
                } else {
                    let entries = filtered_palette(&state.text.buffer);
                    if entries.is_empty() {
                        self.command_line_history_up();
                    } else {
                        state.palette_selected = if state.palette_selected == 0 {
                            entries.len() - 1
                        } else {
                            state.palette_selected - 1
                        };
                    }
                }
            }
            KeyCode::Tab => {
                let entries = filtered_palette(&state.text.buffer);
                match entries.len() {
                    1 => {
                        // Single match: complete fully, add trailing space if takes_args
                        let entry = entries[0];
                        let completed = if entry.takes_args {
                            format!("{} ", entry.command)
                        } else {
                            entry.command.to_string()
                        };
                        state.text.set(&completed);
                        state.palette_selected = 0;
                    }
                    2.. => {
                        // Multiple matches: fill longest common prefix
                        let commands: Vec<&str> = entries.iter().map(|e| e.command).collect();
                        let prefix = longest_common_prefix(&commands);
                        if prefix.len() > state.text.buffer.len() {
                            state.text.set(&prefix);
                            state.palette_selected = 0;
                        }
                    }
                    _ => {
                        // No palette matches: try argument hint completion
                        self.accept_argument_hint();
                    }
                }
            }
            KeyCode::Left => {
                state.text.move_left();
            }
            KeyCode::Right => {
                state.text.move_right();
            }
            KeyCode::Home => {
                state.text.move_home();
            }
            KeyCode::End => {
                state.text.move_end();
            }
            KeyCode::Delete => {
                state.text.delete_forward();
                state.palette_selected = 0;
            }
            KeyCode::Char(c) => {
                state.text.insert_char(c);
                // Reset palette selection and history browsing when buffer changes.
                state.palette_selected = 0;
                state.history_index = None;
            }
            KeyCode::Backspace => {
                state.text.backspace();
                // Reset palette selection and history browsing when buffer changes.
                state.palette_selected = 0;
                state.history_index = None;
            }
            _ => {}
        }
    }

    /// Navigate command history upward (toward older entries).
    fn command_line_history_up(&mut self) {
        if self.command_history.is_empty() {
            return;
        }
        let Some(state) = &mut self.command_line else {
            return;
        };

        match state.history_index {
            None => {
                // First time pressing Up: save current input as draft, show most recent
                state.history_draft = state.text.buffer.clone();
                let idx = self.command_history.len() - 1;
                state.history_index = Some(idx);
                state.text.set(&self.command_history[idx]);
            }
            Some(idx) if idx > 0 => {
                // Move to older entry
                let new_idx = idx - 1;
                state.history_index = Some(new_idx);
                state.text.set(&self.command_history[new_idx]);
            }
            _ => {
                // Already at oldest entry — no-op
            }
        }
    }

    /// Navigate command history downward (toward newer entries / back to draft).
    fn command_line_history_down(&mut self) {
        let Some(state) = &mut self.command_line else {
            return;
        };

        let Some(idx) = state.history_index else {
            return; // Not browsing history
        };

        if idx + 1 < self.command_history.len() {
            // Move to newer entry
            let new_idx = idx + 1;
            state.history_index = Some(new_idx);
            state.text.set(&self.command_history[new_idx]);
        } else {
            // Past newest: restore draft
            state.history_index = None;
            let draft = state.history_draft.clone();
            state.text.set(&draft);
        }
    }

    /// Compute an argument hint for the current command line buffer.
    ///
    /// Returns the suffix to display as ghost text after the cursor, or `None`
    /// if no hint is available. The hint is the portion of the best match that
    /// extends beyond what the user has already typed.
    ///
    /// Hints are available for:
    /// - `:repo <partial>` — matching repo names from the registry
    /// - `:run <partial>` — matching command names from `available_commands`
    pub(super) fn compute_argument_hint(&self) -> Option<String> {
        let state = self.command_line.as_ref()?;
        let buffer = &state.text.buffer;

        // Only hint when cursor is at the end (don't hint mid-edit)
        if state.text.cursor_pos != buffer.chars().count() {
            return None;
        }

        let mut parts = buffer.splitn(2, char::is_whitespace);
        let cmd = parts.next()?.to_ascii_lowercase();
        let rest = parts.next().unwrap_or("").trim_start();

        match cmd.as_str() {
            "run" => {
                if rest.is_empty() || rest.contains(char::is_whitespace) {
                    return None; // No partial arg or already past the command name
                }
                let partial = rest.to_ascii_lowercase();
                self.available_commands
                    .iter()
                    .map(|(name, _)| name.as_str())
                    .find(|name| name.to_ascii_lowercase().starts_with(&partial))
                    .and_then(|name| {
                        let suffix = &name[rest.len()..];
                        if suffix.is_empty() {
                            None
                        } else {
                            Some(suffix.to_string())
                        }
                    })
            }
            "repo" => {
                if rest.is_empty() || rest.contains(char::is_whitespace) {
                    return None;
                }
                let partial = rest.to_ascii_lowercase();
                let repos = self.registry.list_repos();
                repos.iter().find_map(|repo| {
                    let path_str = repo.as_path().display().to_string();
                    // Use the basename for matching
                    let basename = path_str.rsplit('/').next().unwrap_or(&path_str);
                    if basename.to_ascii_lowercase().starts_with(&partial) {
                        let suffix = &basename[rest.len()..];
                        if suffix.is_empty() {
                            None
                        } else {
                            Some(suffix.to_string())
                        }
                    } else {
                        None
                    }
                })
            }
            _ => None,
        }
    }

    /// Accept the current argument hint by appending it to the buffer.
    ///
    /// Called when Tab is pressed and there are no palette matches but an
    /// argument hint is available.
    pub(super) fn accept_argument_hint(&mut self) {
        let hint = self.compute_argument_hint();
        if let Some(suffix) = hint {
            let Some(state) = &mut self.command_line else {
                return;
            };
            for c in suffix.chars() {
                state.text.insert_char(c);
            }
        }
    }

    /// Execute a parsed `CliCommand`.
    pub(super) fn execute_cli_command(&mut self, cmd: CliCommand) {
        match cmd {
            CliCommand::Help => {
                self.push_view(View::Help);
            }
            CliCommand::Quit => {
                self.should_quit = true;
            }
            CliCommand::Refresh => {
                self.needs_refresh = true;
                self.status_message = Some(StatusMessage::info("Refreshing..."));
            }
            CliCommand::Repo(name_or_index) => {
                self.jump_to_repo(&name_or_index);
            }
            CliCommand::Run(command_name, args) => {
                self.run_command_by_name(&command_name, args);
            }
            CliCommand::State => {
                // Refresh all state queries for the currently focused repo
                self.refresh_state_queries();
            }
            CliCommand::Unknown(raw) => {
                if raw.is_empty() {
                    // Silent — empty input
                } else {
                    self.status_message =
                        Some(StatusMessage::error(format!("Unknown command: :{raw}")));
                }
            }
        }
    }

    /// Jump directly to a repo view by name (substring match) or 1-based index.
    ///
    /// Uses `reset_to_view()` to replace the stack rather than pushing — avoids
    /// accumulating depth from direct jumps.
    fn jump_to_repo(&mut self, name_or_index: &str) {
        let repos = self.registry.list_repos();

        // Try 1-based numeric index first
        if let Ok(n) = name_or_index.parse::<usize>() {
            let idx = n.saturating_sub(1);
            if idx < repos.len() {
                self.list_state.select(Some(idx));
                self.reset_to_view(View::RepoDetail(idx));
                return;
            }
            self.status_message = Some(StatusMessage::error(format!("No repository at index {n}")));
            return;
        }

        // Try case-insensitive substring match on path
        let query = name_or_index.to_ascii_lowercase();
        for (idx, repo) in repos.iter().enumerate() {
            let path_str = repo.as_path().display().to_string().to_ascii_lowercase();
            if path_str.contains(&query) {
                self.list_state.select(Some(idx));
                self.reset_to_view(View::RepoDetail(idx));
                return;
            }
        }

        self.status_message = Some(StatusMessage::error(format!(
            "No repository matching: {name_or_index}"
        )));
    }

    /// Execute a graft command by name for the currently selected repository.
    ///
    /// If in `RepoDetail` view, uses that repo. Otherwise uses the currently
    /// selected repo in the dashboard list. Pushes `CommandOutput` view and
    /// starts the command.
    fn run_command_by_name(&mut self, command_name: &str, args: Vec<String>) {
        // Determine the repo path
        let repo_path = match self.current_view().clone() {
            View::RepoDetail(idx) => {
                let repos = self.registry.list_repos();
                repos.get(idx).map(|r| r.as_path().display().to_string())
            }
            _ => {
                // Fall back to the selected dashboard item
                self.list_state.selected().and_then(|idx| {
                    let repos = self.registry.list_repos();
                    repos.get(idx).map(|r| r.as_path().display().to_string())
                })
            }
        };

        let Some(repo_path) = repo_path else {
            self.status_message =
                Some(StatusMessage::warning("No repository selected".to_string()));
            return;
        };

        // Set up the repo path for execute_command_with_args
        self.selected_repo_for_commands = Some(repo_path);

        // Push CommandOutput, then start the command
        self.push_view(View::CommandOutput);
        self.execute_command_with_args(command_name.to_string(), args);
    }
}

// ===== Unit tests for parse_command =====

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
    }

    #[test]
    fn parse_state_command() {
        assert_eq!(parse_command("state"), CliCommand::State);
        assert_eq!(parse_command("State"), CliCommand::State);
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
        // `:repo` with no argument is invalid
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
        // `:run` with no argument is invalid
        assert_eq!(parse_command("run"), CliCommand::Unknown("run".to_string()));
    }

    #[test]
    fn parse_unknown_command() {
        assert_eq!(
            parse_command("frobnicate"),
            CliCommand::Unknown("frobnicate".to_string())
        );
        assert_eq!(
            parse_command("launch session"),
            CliCommand::Unknown("launch session".to_string())
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
        // shell_words handles quoted args
        assert_eq!(
            parse_command(r#"run test "arg with spaces""#),
            CliCommand::Run("test".to_string(), vec!["arg with spaces".to_string()])
        );
    }

    // ===== longest_common_prefix tests =====

    #[test]
    fn lcp_empty_slice() {
        assert_eq!(longest_common_prefix(&[]), "");
    }

    #[test]
    fn lcp_single_string() {
        assert_eq!(longest_common_prefix(&["hello"]), "hello");
    }

    #[test]
    fn lcp_common_prefix() {
        assert_eq!(longest_common_prefix(&["refresh", "repo", "run"]), "r");
    }

    #[test]
    fn lcp_full_match() {
        assert_eq!(longest_common_prefix(&["help", "help"]), "help");
    }

    #[test]
    fn lcp_no_common() {
        assert_eq!(longest_common_prefix(&["abc", "xyz"]), "");
    }

    #[test]
    fn lcp_partial() {
        assert_eq!(longest_common_prefix(&["refresh", "repo"]), "re");
    }
}
