//! Transcript-paradigm TUI: scrolling content area with command prompt.
//!
//! Replaces the old spatial dashboard with a single scrolling transcript.
//! Every action is triggered from the prompt, results appear as blocks in the scroll buffer.

use crossterm::event::KeyCode;
use graft_common::runtime::SessionRuntime;
use graft_common::CommandDef;
use grove_core::{CommandState, RepoDetail, RepoDetailProvider, RepoRegistry};
use grove_engine::GraftYamlConfigLoader;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::Instant;

use super::command_exec::CommandEvent;
use super::command_line::CliCommand;
use super::formatting::{extract_basename, format_file_change_indicator};
use super::prompt::{CompletionState, PickerItem, PickerOutcome, PickerState, PromptState};
use super::scroll_buffer::{BlockId, ContentBlock, ScrollBuffer};
use super::status_bar::StatusMessage;

/// Default number of recent commits to show.
const DEFAULT_MAX_COMMITS: usize = 10;

/// Active repository context.
#[derive(Debug)]
pub(super) struct RepoContext {
    /// Index of the selected repo in the registry.
    pub(super) selected_index: Option<usize>,
    /// Cached detail for the selected repo.
    pub(super) cached_detail: Option<RepoDetail>,
    /// Index for which detail was cached.
    pub(super) cached_detail_index: Option<usize>,
    /// Available commands for the selected repo.
    pub(super) available_commands: Vec<(String, CommandDef)>,
    /// Path of the repo selected for command execution.
    pub(super) selected_repo_path: Option<String>,
    /// Cached state query definitions from graft.yaml.
    pub(super) cached_state_queries: Option<Vec<crate::state::StateQuery>>,
    /// In-memory cache for state query results resolved during the current session.
    /// Keyed by query name. Cleared after any `:run` command execution.
    pub(super) in_memory_state: std::collections::HashMap<String, serde_json::Value>,
    /// Cached result of `commands_with_resolved_options`.
    /// Computed lazily on first call; cleared by `invalidate_caches` (repo switch, `:refresh`),
    /// by the `:run` handler, and by `load_commands_for_repo` when the command list is replaced.
    pub(super) resolved_commands: Option<Vec<(String, CommandDef)>>,
    /// Cached scion name completions for the selected repo.
    /// Computed lazily on first access; cleared on repo switch, `:refresh`,
    /// and after scion-mutating commands (create, start, stop, prune, fuse).
    pub(super) cached_scion_completions: Option<Vec<super::prompt::ArgCompletion>>,
}

impl RepoContext {
    fn new() -> Self {
        Self {
            selected_index: None,
            cached_detail: None,
            cached_detail_index: None,
            available_commands: Vec::new(),
            selected_repo_path: None,
            cached_state_queries: None,
            in_memory_state: std::collections::HashMap::new(),
            resolved_commands: None,
            cached_scion_completions: None,
        }
    }

    /// Invalidate caches but keep the current repo selection.
    fn invalidate_caches(&mut self) {
        self.cached_detail = None;
        self.cached_detail_index = None;
        self.available_commands.clear();
        self.cached_state_queries = None;
        self.in_memory_state.clear();
        self.resolved_commands = None;
        self.cached_scion_completions = None;
    }

    /// Full reset: clear caches and deselect repo.
    #[allow(dead_code)]
    fn reset(&mut self) {
        self.invalidate_caches();
        self.selected_repo_path = None;
        self.selected_index = None;
    }
}

/// Command execution state.
#[derive(Debug)]
pub(super) struct ExecutionState {
    pub(super) command_event_rx: Option<Receiver<CommandEvent>>,
    pub(super) running_command_pid: Option<u32>,
    pub(super) command_state: CommandState,
    pub(super) command_name: Option<String>,
    pub(super) current_log_path: Option<std::path::PathBuf>,
    /// The `BlockId` of the active `ContentBlock::Running` in the scroll buffer.
    /// `None` when no command is executing.
    pub(super) active_output_block: Option<BlockId>,
}

impl ExecutionState {
    fn new() -> Self {
        Self {
            command_event_rx: None,
            running_command_pid: None,
            command_state: CommandState::NotStarted,
            command_name: None,
            current_log_path: None,
            active_output_block: None,
        }
    }
}

/// Main transcript TUI application state.
pub struct TranscriptApp<R, D> {
    // Core
    pub(super) registry: R,
    pub(super) detail_provider: D,
    pub(super) workspace_name: String,
    pub(super) should_quit: bool,

    // Active repo context
    pub(super) context: RepoContext,

    // Display
    pub(super) scroll: ScrollBuffer,

    // Input
    pub(super) prompt: PromptState,

    // Picker overlay (Some when a picker is open over the transcript)
    pub(super) picker: Option<PickerState>,

    // Execution
    pub(super) execution: ExecutionState,

    // Status
    pub(super) status: Option<StatusMessage>,

    // Focus: per-query selected entity value (query name → value)
    pub(super) focus: HashMap<String, String>,

    // Misc
    pub(super) needs_refresh: bool,
    #[allow(dead_code)]
    pub(super) graft_loader: GraftYamlConfigLoader,
}

impl<R: RepoRegistry, D: RepoDetailProvider> TranscriptApp<R, D> {
    pub(super) fn new(registry: R, detail_provider: D, workspace_name: String) -> Self {
        let mut app = Self {
            registry,
            detail_provider,
            workspace_name,
            should_quit: false,
            context: RepoContext::new(),
            scroll: ScrollBuffer::new(),
            prompt: PromptState::new(),
            picker: None,
            execution: ExecutionState::new(),
            status: None,
            focus: HashMap::new(),
            needs_refresh: false,
            graft_loader: GraftYamlConfigLoader::new(),
        };

        // Select first repo by default
        let repos = app.registry.list_repos();
        if !repos.is_empty() {
            app.context.selected_index = Some(0);
            app.context.selected_repo_path = Some(repos[0].as_path().display().to_string());
        }

        // Load commands for initial repo (enables ghost hints for :run)
        app.load_commands_for_repo();

        // Push initial welcome block
        app.push_welcome_block();

        app
    }

    // ===== Event handling =====

    /// Handle a key press.
    #[allow(clippy::too_many_lines)]
    pub(super) fn handle_key(&mut self, code: KeyCode, modifiers: crossterm::event::KeyModifiers) {
        // Picker overlay intercepts all keys when open
        let picker_outcome = self
            .picker
            .as_mut()
            .map(|picker| picker.handle_key(code, modifiers));
        if let Some(outcome) = picker_outcome {
            match outcome {
                PickerOutcome::Select(cmd) => {
                    self.picker = None;
                    self.execute_cli_command(cmd);
                }
                PickerOutcome::Dismiss => {
                    self.picker = None;
                }
                PickerOutcome::Nothing => {}
            }
            return;
        }

        // Command line intercepts all keys when active
        if self.prompt.is_active() {
            let focus_opts = self.focus_entity_opts_for_buffer();
            let resolved = self.commands_with_resolved_options();
            let scion_comps = self.scion_completions();
            let cs = self.prompt.compute_completions(
                &resolved,
                &self.repo_basenames(),
                &self.state_query_names(),
                &focus_opts,
                &scion_comps,
            );
            if let Some(cmd) = self.prompt.handle_key(code, modifiers, &cs) {
                self.execute_cli_command(cmd);
            }
            return;
        }

        // `:` opens command line from anywhere
        if code == KeyCode::Char(':') {
            self.prompt.open();
            return;
        }

        // Global key bindings
        match code {
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.scroll.scroll_down(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.scroll.scroll_up(1);
            }
            KeyCode::Char('G') => {
                self.scroll.scroll_to_bottom();
            }
            KeyCode::Char('g') => {
                self.scroll.scroll_offset = 0;
            }
            KeyCode::Enter => {
                // Clone actions to avoid borrow conflict when accessing rows later
                let actions_opt = self.scroll.focused_block_actions().map(<[_]>::to_vec);
                if let Some(actions) = actions_opt {
                    // Build picker items from focused table's rows and per-row actions
                    let items: Vec<PickerItem> = if let Some(idx) = self.scroll.focused_block {
                        if let Some(ContentBlock::Table { rows, .. }) = self.scroll.blocks.get(idx)
                        {
                            rows.iter()
                                .zip(actions.iter())
                                .map(|(row, action)| {
                                    let label = row
                                        .first()
                                        .map(|s| s.content.to_string())
                                        .unwrap_or_default();
                                    let description = row
                                        .get(1)
                                        .map(|s| s.content.to_string())
                                        .unwrap_or_default();
                                    PickerItem {
                                        label,
                                        description,
                                        action: action.clone(),
                                    }
                                })
                                .collect()
                        } else {
                            vec![]
                        }
                    } else {
                        vec![]
                    };
                    if !items.is_empty() {
                        self.picker = Some(PickerState::new(items));
                    }
                } else {
                    self.scroll.toggle_focused_collapse();
                }
            }
            KeyCode::Char('c') => {
                self.scroll.toggle_focused_collapse();
            }
            KeyCode::Tab => {
                self.scroll.focus_next();
            }
            KeyCode::BackTab => {
                self.scroll.focus_prev();
            }
            KeyCode::Char('r') => {
                self.needs_refresh = true;
                self.status = Some(StatusMessage::info("Refreshing..."));
            }
            KeyCode::Char('?') => {
                self.cmd_help();
            }
            _ => {}
        }
    }

    // ===== Command execution =====

    fn execute_cli_command(&mut self, cmd: CliCommand) {
        match cmd {
            CliCommand::Help => self.cmd_help(),
            CliCommand::Quit => {
                self.should_quit = true;
            }
            CliCommand::Refresh => {
                self.needs_refresh = true;
                self.status = Some(StatusMessage::info("Refreshing..."));
            }
            CliCommand::Repo(name_or_index) => self.cmd_repo(&name_or_index),
            CliCommand::Repos => self.cmd_repos(),
            CliCommand::Run(command_name, args) => {
                // Clear in-memory and resolved-commands caches: a command may change repo state.
                self.context.in_memory_state.clear();
                self.context.resolved_commands = None;
                self.cmd_run(&command_name, args);
            }
            CliCommand::Status => self.cmd_status(),
            CliCommand::Catalog(cat) => self.cmd_catalog(cat.as_deref()),
            CliCommand::State(name) => self.cmd_state(name.as_deref()),
            CliCommand::Invalidate(name) => self.cmd_invalidate(name.as_deref()),
            CliCommand::Focus(query, value) => {
                self.cmd_focus(query.as_deref(), value.as_deref());
            }
            CliCommand::Unfocus(query) => {
                self.cmd_unfocus(query.as_deref());
            }
            CliCommand::ScionList => self.cmd_scion_list(),
            CliCommand::ScionCreate(name) => self.cmd_scion_create(&name),
            CliCommand::ScionStart(name) => self.cmd_scion_start(&name),
            CliCommand::ScionStop(name) => self.cmd_scion_stop(&name),
            CliCommand::ScionPrune(name) => self.cmd_scion_prune(&name),
            CliCommand::ScionFuse(name) => self.cmd_scion_fuse(&name),
            CliCommand::Attach(name) => self.cmd_attach(&name),
            CliCommand::Review(name, full) => self.cmd_review(&name, full),
            CliCommand::Unknown(raw) => {
                if !raw.is_empty() {
                    self.status = Some(StatusMessage::error(format!("Unknown command: :{raw}")));
                }
            }
        }
    }

    // ===== Commands =====

    fn push_welcome_block(&mut self) {
        let lines = vec![
            Line::from(vec![Span::styled(
                "Welcome to Grove",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(Span::styled(
                "Type : to open the command palette, or try these commands:",
                Style::default().fg(Color::Gray),
            )),
            Line::from(vec![
                Span::styled("  :repos", Style::default().fg(Color::Cyan)),
                Span::styled(
                    "     Show all repositories",
                    Style::default().fg(Color::Gray),
                ),
            ]),
            Line::from(vec![
                Span::styled("  :repo ", Style::default().fg(Color::Cyan)),
                Span::styled("<name>", Style::default().fg(Color::White)),
                Span::styled("  Switch to a repository", Style::default().fg(Color::Gray)),
            ]),
            Line::from(vec![
                Span::styled("  :help", Style::default().fg(Color::Cyan)),
                Span::styled("      Command reference", Style::default().fg(Color::Gray)),
            ]),
            Line::from(vec![
                Span::styled("  :quit", Style::default().fg(Color::Cyan)),
                Span::styled("      Exit grove", Style::default().fg(Color::Gray)),
            ]),
        ];

        self.scroll.push(ContentBlock::Text {
            id: BlockId::new(),
            lines,
            collapsed: false,
        });
    }

    /// `:repos` — show a table of all repositories.
    fn cmd_repos(&mut self) {
        let repos = self.registry.list_repos();

        if repos.is_empty() {
            self.scroll.push(ContentBlock::Text {
                id: BlockId::new(),
                lines: vec![Line::from(Span::styled(
                    "No repositories configured",
                    Style::default().fg(Color::Yellow),
                ))],
                collapsed: false,
            });
            return;
        }

        // Repository is the first column so it becomes the picker label when filtering.
        let headers = vec![
            "Repository".to_string(),
            "Branch".to_string(),
            "Status".to_string(),
            "#".to_string(),
        ];

        let mut rows = Vec::new();
        let mut actions = Vec::new();
        for (i, repo) in repos.iter().enumerate() {
            let path_str = repo.as_path().display().to_string();
            let basename = extract_basename(&path_str).to_string();
            let status = self.registry.get_status(repo);

            let (branch, dirty) = match status {
                Some(s) => {
                    let b = s.branch.as_deref().unwrap_or("detached").to_string();
                    let d = if s.is_dirty { "\u{25cf}" } else { "\u{25cb}" };
                    let color = if s.is_dirty {
                        Color::Yellow
                    } else {
                        Color::Green
                    };
                    (
                        Span::styled(b, Style::default().fg(Color::Cyan)),
                        Span::styled(d, Style::default().fg(color)),
                    )
                }
                None => (
                    Span::styled("...", Style::default().fg(Color::DarkGray)),
                    Span::styled("-", Style::default().fg(Color::DarkGray)),
                ),
            };

            let selected = self.context.selected_index == Some(i);
            let idx_style = if selected {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let name_style = if selected {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            actions.push(CliCommand::Repo(basename.clone()));
            rows.push(vec![
                Span::styled(basename, name_style),
                branch,
                dirty,
                Span::styled(format!("{}", i + 1), idx_style),
            ]);
        }

        self.scroll.push(ContentBlock::Table {
            id: BlockId::new(),
            title: "Repositories".to_string(),
            headers,
            rows,
            collapsed: false,
            actions: Some(actions),
        });
    }

    /// `:repo <name|index>` — switch the active repository.
    fn cmd_repo(&mut self, name_or_index: &str) {
        // Handle special case: `:repo repos` shows the repo list
        if name_or_index == "repos" {
            self.cmd_repos();
            return;
        }

        let repos = self.registry.list_repos();

        // Try 1-based numeric index first
        if let Ok(n) = name_or_index.parse::<usize>() {
            let idx = n.saturating_sub(1);
            if idx < repos.len() {
                self.switch_repo(idx);
                return;
            }
            self.status = Some(StatusMessage::error(format!("No repository at index {n}")));
            return;
        }

        // Try case-insensitive substring match on path
        let query = name_or_index.to_ascii_lowercase();
        for (idx, repo) in repos.iter().enumerate() {
            let path_str = repo.as_path().display().to_string().to_ascii_lowercase();
            if path_str.contains(&query) {
                self.switch_repo(idx);
                return;
            }
        }

        self.status = Some(StatusMessage::error(format!(
            "No repository matching: {name_or_index}"
        )));
    }

    /// Switch to a repo by index and push a confirmation block.
    fn switch_repo(&mut self, idx: usize) {
        let repos = self.registry.list_repos();
        let path_str = repos[idx].as_path().display().to_string();
        let basename = extract_basename(&path_str).to_string();

        self.context.invalidate_caches();
        self.context.selected_index = Some(idx);
        self.context.selected_repo_path = Some(path_str.clone());

        // Load commands for the new repo
        self.load_commands_for_repo();

        // Eagerly populate completion and option caches so that the first
        // prompt open (`:`) doesn't block on subprocess I/O.
        let _ = self.scion_completions();
        let _ = self.commands_with_resolved_options();

        let status = self.registry.get_status(&repos[idx]);
        let branch_info = status
            .and_then(|s| s.branch.as_ref())
            .map_or(String::new(), |b| format!(" ({b})"));

        self.scroll.push(ContentBlock::Text {
            id: BlockId::new(),
            lines: vec![Line::from(vec![
                Span::styled("\u{2192} ", Style::default().fg(Color::Green)),
                Span::styled(
                    basename,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(branch_info, Style::default().fg(Color::Cyan)),
            ])],
            collapsed: false,
        });
    }

    /// `:help` — push a help reference block.
    fn cmd_help(&mut self) {
        let lines = vec![
            Line::from(Span::styled(
                "Command Reference",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            help_line(":repos", "Show all repositories"),
            help_line(":repo <name|idx>", "Switch active repository"),
            help_line(":run <cmd> [args]", "Execute a graft command"),
            help_line(":status", "Show file changes and recent commits"),
            help_line(":catalog [category]", "List available commands/sequences"),
            help_line(":state [name]", "Show cached state queries"),
            help_line(":invalidate [name]", "Clear cached state"),
            help_line(":refresh / :r", "Refresh repository statuses"),
            help_line(":help / :h", "Show this reference"),
            help_line(":quit / :q", "Exit grove"),
            Line::from(""),
            Line::from(Span::styled(
                "Navigation",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            help_line("j/k", "Scroll down/up"),
            help_line("g/G", "Scroll to top/bottom"),
            help_line("Tab/Shift+Tab", "Focus next/prev block"),
            help_line("Enter", "Activate focused block (open picker on tables)"),
            help_line("c", "Toggle collapse on focused block"),
            help_line(":", "Open command palette"),
            help_line("r", "Refresh"),
            help_line("q", "Quit"),
        ];

        self.scroll.push(ContentBlock::Text {
            id: BlockId::new(),
            lines,
            collapsed: false,
        });
    }

    /// `:run <cmd> [args]` — execute a command.
    #[allow(clippy::too_many_lines)]
    fn cmd_run(&mut self, command_name: &str, args: Vec<String>) {
        // Guard: reject concurrent executions — one command at a time.
        if self.execution.command_state == CommandState::Running {
            self.status = Some(StatusMessage::warning(
                "A command is already running — wait for it to finish",
            ));
            return;
        }

        let repo_path = if let Some(p) = &self.context.selected_repo_path {
            p.clone()
        } else {
            // Try to use first repo
            let repos = self.registry.list_repos();
            if repos.is_empty() {
                self.status = Some(StatusMessage::warning("No repository selected"));
                return;
            }
            let p = repos[0].as_path().display().to_string();
            self.context.selected_index = Some(0);
            self.context.selected_repo_path = Some(p.clone());
            p
        };

        // Ensure commands are loaded
        if self.context.available_commands.is_empty() {
            self.load_commands_for_repo();
        }

        // Auto-fill missing required args from focus when options_from matches a focused query.
        // Clone arg_defs to avoid borrow conflict with self.focus.
        let mut args = args;
        {
            let arg_defs_opt: Option<Vec<graft_common::ArgDef>> = self
                .context
                .available_commands
                .iter()
                .find(|(n, _)| n == command_name)
                .and_then(|(_, cmd_def)| cmd_def.args.clone());
            if let Some(arg_defs) = arg_defs_opt {
                let mut auto_filled: Vec<String> = Vec::new();
                for (i, arg_def) in arg_defs.iter().enumerate() {
                    if i < args.len() {
                        continue; // user-supplied arg
                    }
                    if arg_def.required && arg_def.default.is_none() {
                        if let Some(query_name) = &arg_def.options_from {
                            if let Some(focused_value) = self.focus.get(query_name).cloned() {
                                args.push(focused_value.clone());
                                auto_filled.push(format!("{query_name}: {focused_value}"));
                                continue;
                            }
                        }
                        break; // required arg can't be filled — stop to avoid mispositioning
                    }
                }
                if !auto_filled.is_empty() {
                    self.status = Some(StatusMessage::info(format!(
                        "Using focused {}",
                        auto_filled.join(", ")
                    )));
                }
            }
        }

        // Validate command exists and check required args
        if let Some((_, cmd_def)) = self
            .context
            .available_commands
            .iter()
            .find(|(n, _)| n == command_name)
        {
            if let Some(arg_defs) = &cmd_def.args {
                let missing: Vec<&graft_common::ArgDef> = arg_defs
                    .iter()
                    .filter(|a| a.required && a.default.is_none())
                    .skip(args.len())
                    .collect();
                if !missing.is_empty() {
                    let mut lines = vec![Line::from(Span::styled(
                        format!("Missing required arguments for '{command_name}'"),
                        Style::default().fg(Color::Red),
                    ))];
                    for arg in arg_defs {
                        let req = if arg.required && arg.default.is_none() {
                            "*"
                        } else {
                            " "
                        };
                        let desc = arg.description.as_deref().unwrap_or("");
                        lines.push(Line::from(vec![
                            Span::styled(
                                format!("  {req}{:<16}", arg.name),
                                Style::default().fg(Color::Cyan),
                            ),
                            Span::styled(desc.to_string(), Style::default().fg(Color::Gray)),
                        ]));
                    }
                    lines.push(Line::from(Span::styled(
                        format!(
                            "Usage: :run {command_name} {}",
                            arg_defs
                                .iter()
                                .map(|a| if a.required && a.default.is_none() {
                                    format!("<{}>", a.name)
                                } else {
                                    format!("[{}]", a.name)
                                })
                                .collect::<Vec<_>>()
                                .join(" ")
                        ),
                        Style::default().fg(Color::DarkGray),
                    )));
                    self.scroll.push(ContentBlock::Text {
                        id: BlockId::new(),
                        lines,
                        collapsed: false,
                    });
                    return;
                }
            }
        }

        let run_ctx = Some(super::command_exec::RunContext {
            workspace: self.workspace_name.clone(),
            repo: graft_common::repo_name_from_path(&repo_path).to_string(),
            command: command_name.to_string(),
        });

        let (tx, rx) = std::sync::mpsc::channel();
        self.execution.command_event_rx = Some(rx);
        self.execution.command_name = Some(command_name.to_string());
        self.execution.command_state = CommandState::Running;
        self.execution.running_command_pid = None;
        self.execution.current_log_path = None;

        // Clone args for the display block before moving them into the thread.
        let display_args = args.clone();
        let cmd_name = command_name.to_string();
        let repo = repo_path;
        std::thread::spawn(move || {
            super::command_exec::spawn_command(cmd_name, args, repo, run_ctx, tx);
        });

        // Push a live Running block — it animates until finalized on completion.
        let block_id = BlockId::new();
        self.execution.active_output_block = Some(block_id);
        self.scroll.push(ContentBlock::Running {
            id: block_id,
            command: command_name.to_string(),
            args: display_args,
            started_at: Instant::now(),
            output_lines: vec![],
            output_truncated: false,
            collapsed: false,
        });
    }

    /// `:status` — show changed files and recent commits.
    fn cmd_status(&mut self) {
        let Some(repo_path) = self.context.selected_repo_path.clone() else {
            self.status = Some(StatusMessage::warning("No repository selected"));
            return;
        };

        let detail = self.load_detail(&repo_path);
        let basename = extract_basename(&repo_path).to_string();

        let mut lines = vec![Line::from(Span::styled(
            format!("Status: {basename}"),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))];

        if let Some(err) = &detail.error {
            lines.push(Line::from(Span::styled(
                format!("Error: {err}"),
                Style::default().fg(Color::Red),
            )));
        }

        // Changed files section
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Changed Files",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));

        if detail.changed_files.is_empty() {
            lines.push(Line::from(Span::styled(
                "  (no changes)",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            for fc in &detail.changed_files {
                let (indicator, color) = format_file_change_indicator(&fc.status);
                lines.push(Line::from(vec![
                    Span::styled(format!("  {indicator} "), Style::default().fg(color)),
                    Span::styled(fc.path.clone(), Style::default().fg(Color::White)),
                ]));
            }
        }

        // Recent commits section
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Recent Commits",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));

        if detail.commits.is_empty() {
            lines.push(Line::from(Span::styled(
                "  (no commits)",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            for c in &detail.commits {
                let short_hash = if c.hash.len() > 7 {
                    &c.hash[..7]
                } else {
                    &c.hash
                };
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {short_hash} "),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::styled(c.subject.clone(), Style::default().fg(Color::White)),
                    Span::styled(
                        format!("  {} {}", c.author, c.relative_date),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
            }
        }

        self.scroll.push(ContentBlock::Text {
            id: BlockId::new(),
            lines,
            collapsed: false,
        });
    }

    /// Load repo detail, using/populating the cache.
    fn load_detail(&mut self, repo_path: &str) -> RepoDetail {
        let idx = self.context.selected_index;
        // Return cached if available and for the same index
        if let (Some(cached), Some(ci)) = (
            &self.context.cached_detail,
            self.context.cached_detail_index,
        ) {
            if idx == Some(ci) {
                return cached.clone();
            }
        }

        let repo = match grove_core::RepoPath::new(repo_path) {
            Ok(r) => r,
            Err(e) => return RepoDetail::with_error(format!("Invalid path: {e}")),
        };
        let detail = self
            .detail_provider
            .get_detail(&repo, DEFAULT_MAX_COMMITS)
            .unwrap_or_else(|e| RepoDetail::with_error(e.to_string()));

        self.context.cached_detail = Some(detail.clone());
        self.context.cached_detail_index = idx;
        detail
    }

    /// `:catalog [category]` — list available commands and sequences.
    fn cmd_catalog(&mut self, category_filter: Option<&str>) {
        let Some(repo_path) = self.context.selected_repo_path.clone() else {
            self.status = Some(StatusMessage::warning("No repository selected"));
            return;
        };

        // Ensure commands are loaded
        if self.context.available_commands.is_empty() {
            self.load_commands_for_repo();
        }

        // Build combined list: (name, description, category)
        // Sequences are prefixed with "\u{00bb} " to distinguish from commands.
        let mut entries: Vec<(String, String, String)> = Vec::new();

        for (name, cmd) in &self.context.available_commands {
            entries.push((
                name.clone(),
                cmd.description.clone().unwrap_or_default(),
                cmd.category
                    .clone()
                    .unwrap_or_else(|| "uncategorized".to_string()),
            ));
        }

        // Load sequences from graft.yaml (local + deps, mirroring load_commands_for_repo)
        let repo_base = PathBuf::from(&repo_path);
        let graft_yaml_path = repo_base.join("graft.yaml");
        self.load_sequences_into(&graft_yaml_path, None, &mut entries);

        let graft_dir = repo_base.join(".graft");
        if let Ok(dir_entries) = std::fs::read_dir(&graft_dir) {
            for entry in dir_entries.flatten() {
                let dep_name = entry.file_name().to_string_lossy().to_string();
                if !entry.file_type().is_ok_and(|ft| ft.is_dir()) {
                    continue;
                }
                if dep_name == "run-state" || dep_name == "runs" {
                    continue;
                }
                let dep_yaml = graft_dir.join(&dep_name).join("graft.yaml");
                self.load_sequences_into(&dep_yaml, Some(&dep_name), &mut entries);
            }
        }

        // Apply optional category filter
        if let Some(filter) = category_filter {
            let filter_lower = filter.to_ascii_lowercase();
            entries.retain(|e| e.2.to_ascii_lowercase() == filter_lower);
        }

        // Sort by category then name
        entries.sort_by(|a, b| a.2.cmp(&b.2).then(a.0.cmp(&b.0)));

        if entries.is_empty() {
            let msg = if let Some(filter) = category_filter {
                format!("No commands matching category: {filter}")
            } else {
                "No commands or sequences found".to_string()
            };
            self.status = Some(StatusMessage::info(msg));
            return;
        }

        let headers = vec![
            "Name".to_string(),
            "Description".to_string(),
            "Category".to_string(),
        ];

        let mut rows = Vec::new();
        let mut actions = Vec::new();
        for (name, desc, cat) in entries {
            // Strip the sequence prefix (» ) to get the runnable name
            let run_name = if let Some(stripped) = name.strip_prefix("\u{00bb} ") {
                stripped.to_string()
            } else {
                name.clone()
            };
            actions.push(CliCommand::Run(run_name, vec![]));
            rows.push(vec![
                Span::styled(name, Style::default().fg(Color::Cyan)),
                Span::styled(desc, Style::default().fg(Color::White)),
                Span::styled(cat, Style::default().fg(Color::DarkGray)),
            ]);
        }

        self.scroll.push(ContentBlock::Table {
            id: BlockId::new(),
            title: "Catalog".to_string(),
            headers,
            rows,
            collapsed: false,
            actions: Some(actions),
        });
    }

    /// `:state [name]` — show cached state queries.
    #[allow(clippy::too_many_lines)]
    fn cmd_state(&mut self, name: Option<&str>) {
        let Some(repo_path) = self.context.selected_repo_path.clone() else {
            self.status = Some(StatusMessage::warning("No repository selected"));
            return;
        };

        // Discover and cache state queries (root + all dep graft.yamls)
        if self.context.cached_state_queries.is_none() {
            let (queries, warnings) =
                crate::state::discover_all_state_queries(&PathBuf::from(&repo_path));
            if let Some(w) = warnings.first() {
                self.status = Some(StatusMessage::warning(w.clone()));
            }
            self.context.cached_state_queries = Some(queries);
        }

        let queries = self
            .context
            .cached_state_queries
            .clone()
            .unwrap_or_default();
        let repo_name = graft_common::repo_name_from_path(&repo_path);

        match name {
            None => {
                // Summary table of all queries
                if queries.is_empty() {
                    self.status = Some(StatusMessage::info("No state queries defined"));
                    return;
                }

                let headers = vec![
                    "Query".to_string(),
                    "Summary".to_string(),
                    "Age".to_string(),
                    "Cached".to_string(),
                ];

                let mut rows = Vec::new();
                let mut actions = Vec::new();
                for q in &queries {
                    let cached =
                        graft_common::read_latest_cached(&self.workspace_name, repo_name, &q.name);
                    let (summary, age) = match &cached {
                        Some(result) => (
                            crate::state::format_state_summary(result),
                            result.metadata.time_ago(),
                        ),
                        None => ("(not cached)".to_string(), "-".to_string()),
                    };
                    // "yes" when query declares inputs (cacheable), "no" otherwise
                    let cacheable = if q.inputs.as_ref().is_some_and(|v| !v.is_empty()) {
                        "yes"
                    } else {
                        "no"
                    };

                    actions.push(CliCommand::State(Some(q.name.clone())));
                    rows.push(vec![
                        Span::styled(q.name.clone(), Style::default().fg(Color::Cyan)),
                        Span::styled(summary, Style::default().fg(Color::White)),
                        Span::styled(age, Style::default().fg(Color::DarkGray)),
                        Span::styled(cacheable.to_string(), Style::default().fg(Color::DarkGray)),
                    ]);
                }

                self.scroll.push(ContentBlock::Table {
                    id: BlockId::new(),
                    title: "State Queries".to_string(),
                    headers,
                    rows,
                    collapsed: false,
                    actions: Some(actions),
                });
            }
            Some(query_name) => {
                // Validate query name against discovered queries
                let known = queries.iter().any(|q| q.name == query_name);
                if !known {
                    let available: Vec<&str> = queries.iter().map(|q| q.name.as_str()).collect();
                    let msg = if available.is_empty() {
                        format!("Unknown state query: {query_name} (no queries defined)")
                    } else {
                        format!(
                            "Unknown state query: {query_name}. Available: {}",
                            available.join(", ")
                        )
                    };
                    self.status = Some(StatusMessage::warning(msg));
                    return;
                }

                // Detail for a specific query
                let cached =
                    graft_common::read_latest_cached(&self.workspace_name, repo_name, query_name);

                match cached {
                    Some(result) => {
                        let pretty = serde_json::to_string_pretty(&result.data)
                            .unwrap_or_else(|_| format!("{:?}", result.data));
                        let mut lines = vec![
                            Line::from(Span::styled(
                                format!("State: {query_name}"),
                                Style::default()
                                    .fg(Color::Cyan)
                                    .add_modifier(Modifier::BOLD),
                            )),
                            Line::from(vec![
                                Span::styled("  Commit: ", Style::default().fg(Color::DarkGray)),
                                Span::styled(
                                    result.metadata.commit_hash.clone(),
                                    Style::default().fg(Color::Yellow),
                                ),
                            ]),
                            Line::from(vec![
                                Span::styled("  Age:    ", Style::default().fg(Color::DarkGray)),
                                Span::styled(
                                    result.metadata.time_ago(),
                                    Style::default().fg(Color::White),
                                ),
                            ]),
                            Line::from(""),
                        ];

                        for line in pretty.lines() {
                            lines.push(Line::from(Span::styled(
                                format!("  {line}"),
                                Style::default().fg(Color::White),
                            )));
                        }

                        self.scroll.push(ContentBlock::Text {
                            id: BlockId::new(),
                            lines,
                            collapsed: false,
                        });
                    }
                    None => {
                        self.status = Some(StatusMessage::info(format!(
                            "No cached state for query: {query_name}"
                        )));
                    }
                }
            }
        }
    }

    /// `:invalidate [name]` — clear cached state.
    fn cmd_invalidate(&mut self, name: Option<&str>) {
        let Some(repo_path) = self.context.selected_repo_path.clone() else {
            self.status = Some(StatusMessage::warning("No repository selected"));
            return;
        };

        let repo_name = graft_common::repo_name_from_path(&repo_path);

        match graft_common::invalidate_cached_state(&self.workspace_name, repo_name, name) {
            Ok(count) => {
                // Clear local caches
                self.context.cached_state_queries = None;
                self.context.cached_detail = None;
                self.context.cached_detail_index = None;

                let msg = match name {
                    Some(n) => format!("Invalidated cache for query '{n}' ({count} files removed)"),
                    None => format!("Invalidated all cached state ({count} files removed)"),
                };
                self.status = Some(StatusMessage::success(msg));
            }
            Err(e) => {
                self.status = Some(StatusMessage::error(format!(
                    "Failed to invalidate cache: {e}"
                )));
            }
        }
    }

    /// `:focus [query [value]]` — list, pick, or set a focused entity.
    ///
    /// Three modes:
    /// - `focus` (no args) — list focusable queries and their current focus.
    /// - `focus <query>` — open a picker over that query's entity values.
    /// - `focus <query> <value>` — set focus directly (no picker).
    #[allow(clippy::too_many_lines)]
    fn cmd_focus(&mut self, query: Option<&str>, value: Option<&str>) {
        match (query, value) {
            // Mode 3: direct set — :focus <query> <value>
            (Some(q), Some(v)) => {
                self.focus.insert(q.to_string(), v.to_string());
                self.status = Some(StatusMessage::success(format!("Focus set: {q} → {v}")));
            }

            // Mode 2: picker — :focus <query>
            (Some(q), None) => {
                let repo_name = self
                    .context
                    .selected_repo_path
                    .as_deref()
                    .map(graft_common::repo_name_from_path)
                    .unwrap_or_default()
                    .to_string();
                let opts = self.resolve_options_from(q, &repo_name);
                if opts.is_empty() {
                    self.status = Some(StatusMessage::warning(format!(
                        "No values found for query: {q}"
                    )));
                    return;
                }
                let q_owned = q.to_string();
                let items: Vec<PickerItem> = opts
                    .into_iter()
                    .map(|v| PickerItem {
                        label: v.clone(),
                        description: String::new(),
                        action: CliCommand::Focus(Some(q_owned.clone()), Some(v)),
                    })
                    .collect();
                self.picker = Some(PickerState::new(items));
            }

            // Mode 1: list — :focus (no args)
            (None, _) => {
                let repo_name = self
                    .context
                    .selected_repo_path
                    .as_deref()
                    .map(graft_common::repo_name_from_path)
                    .unwrap_or_default()
                    .to_string();

                // Ensure state queries are loaded
                if self.context.cached_state_queries.is_none() {
                    if let Some(ref rp) = self.context.selected_repo_path.clone() {
                        let (queries, _) =
                            crate::state::discover_all_state_queries(&PathBuf::from(rp));
                        self.context.cached_state_queries = Some(queries);
                    }
                }

                let queries = self
                    .context
                    .cached_state_queries
                    .clone()
                    .unwrap_or_default();

                if queries.is_empty() && self.focus.is_empty() {
                    self.status = Some(StatusMessage::info("No focusable queries defined"));
                    return;
                }

                let mut lines = vec![
                    Line::from(Span::styled(
                        "Focus",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                ];

                if queries.is_empty() {
                    lines.push(Line::from(Span::styled(
                        "  (no state queries defined)",
                        Style::default().fg(Color::DarkGray),
                    )));
                } else {
                    for q in &queries {
                        let current = self.focus.get(&q.name);
                        let (value_span, hint_span) = match current {
                            Some(v) => (
                                Span::styled(v.clone(), Style::default().fg(Color::Cyan)),
                                Span::styled(
                                    format!("  :unfocus {}", q.name),
                                    Style::default().fg(Color::DarkGray),
                                ),
                            ),
                            None => (
                                Span::styled("(none)", Style::default().fg(Color::DarkGray)),
                                Span::styled(
                                    format!("  :focus {}", q.name),
                                    Style::default().fg(Color::DarkGray),
                                ),
                            ),
                        };
                        lines.push(Line::from(vec![
                            Span::styled(
                                format!("  {:<20}", q.name),
                                Style::default().fg(Color::White),
                            ),
                            value_span,
                            hint_span,
                        ]));
                    }

                    // Show any focused queries not in the discovered list
                    for (q, v) in &self.focus {
                        if !queries.iter().any(|sq| &sq.name == q) {
                            lines.push(Line::from(vec![
                                Span::styled(
                                    format!("  {q:<20}"),
                                    Style::default().fg(Color::DarkGray),
                                ),
                                Span::styled(v.clone(), Style::default().fg(Color::Yellow)),
                                Span::styled(
                                    "  (query not found)",
                                    Style::default().fg(Color::DarkGray),
                                ),
                            ]));
                        }
                    }
                }

                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    format!(
                        "Repo context: {}",
                        if repo_name.is_empty() {
                            "(none)"
                        } else {
                            repo_name.as_str()
                        }
                    ),
                    Style::default().fg(Color::DarkGray),
                )));

                self.scroll.push(ContentBlock::Text {
                    id: BlockId::new(),
                    lines,
                    collapsed: false,
                });
            }
        }
    }

    /// `:unfocus [query]` — clear focus for one query or all queries.
    fn cmd_unfocus(&mut self, query: Option<&str>) {
        if let Some(q) = query {
            if self.focus.remove(q).is_some() {
                self.status = Some(StatusMessage::success(format!("Focus cleared: {q}")));
            } else {
                self.status = Some(StatusMessage::info(format!("No focus set for query: {q}")));
            }
        } else {
            let count = self.focus.len();
            self.focus.clear();
            if count > 0 {
                self.status = Some(StatusMessage::success(format!(
                    "All focuses cleared ({count})"
                )));
            } else {
                self.status = Some(StatusMessage::info("No focuses to clear"));
            }
        }
    }

    // ===== Scion commands =====

    /// `:scion list` — list all scion workstreams.
    fn cmd_scion_list(&mut self) {
        let Some(repo_path) = self.context.selected_repo_path.clone() else {
            self.status = Some(StatusMessage::warning("No repository selected"));
            return;
        };
        let runtime = graft_common::TmuxRuntime::new().ok();
        let runtime_ref = runtime.as_ref().map(|r| r as &dyn SessionRuntime);
        match graft_engine::scion_list(&repo_path, runtime_ref) {
            Ok(scions) if scions.is_empty() => {
                self.scroll.push(ContentBlock::Text {
                    id: BlockId::new(),
                    lines: vec![Line::from(Span::styled(
                        "No scions",
                        Style::default().fg(Color::Gray),
                    ))],
                    collapsed: false,
                });
            }
            Ok(scions) => {
                let mut lines = vec![Line::from(vec![
                    Span::styled(
                        "Scions",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!(" ({})", scions.len()),
                        Style::default().fg(Color::Gray),
                    ),
                ])];
                lines.push(Line::from(""));
                for s in &scions {
                    let ahead_str = s.ahead.map_or("?".to_string(), |a| a.to_string());
                    let behind_str = s.behind.map_or("?".to_string(), |b| b.to_string());
                    let dirty_str = if s.dirty { " [dirty]" } else { "" };
                    let session_str = match s.session_active {
                        Some(true) => " [session]",
                        Some(false) | None => "",
                    };
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!("  {:<20}", s.name),
                            Style::default().fg(Color::Cyan),
                        ),
                        Span::styled(
                            format!("+{ahead_str}/-{behind_str}"),
                            Style::default().fg(Color::Yellow),
                        ),
                        Span::styled(dirty_str, Style::default().fg(Color::Red)),
                        Span::styled(session_str, Style::default().fg(Color::Green)),
                    ]));
                }
                self.scroll.push(ContentBlock::Text {
                    id: BlockId::new(),
                    lines,
                    collapsed: false,
                });
            }
            Err(e) => {
                self.status = Some(StatusMessage::error(format!("scion list failed: {e}")));
            }
        }
    }

    /// `:scion create <name>` — create a new scion.
    fn cmd_scion_create(&mut self, name: &str) {
        let Some(repo_path) = self.context.selected_repo_path.clone() else {
            self.status = Some(StatusMessage::warning("No repository selected"));
            return;
        };
        let config =
            graft_engine::parse_graft_yaml(std::path::Path::new(&repo_path).join("graft.yaml"))
                .ok();
        let dep_configs = config
            .as_ref()
            .map(|c| graft_engine::load_dep_configs(&repo_path, c))
            .unwrap_or_default();
        match graft_engine::scion_create(&repo_path, name, config.as_ref(), &dep_configs) {
            Ok(wt_path) => {
                self.status = Some(StatusMessage::success(format!(
                    "Created scion '{name}' at {}",
                    wt_path.display()
                )));
            }
            Err(e) => {
                self.status = Some(StatusMessage::error(format!("scion create failed: {e}")));
            }
        }
        self.context.cached_scion_completions = None;
    }

    /// `:scion start <name>` — start a scion's runtime session.
    fn cmd_scion_start(&mut self, name: &str) {
        let Some(repo_path) = self.context.selected_repo_path.clone() else {
            self.status = Some(StatusMessage::warning("No repository selected"));
            return;
        };
        let config =
            graft_engine::parse_graft_yaml(std::path::Path::new(&repo_path).join("graft.yaml"))
                .ok();
        let Ok(runtime) = graft_common::TmuxRuntime::new() else {
            self.status = Some(StatusMessage::error("tmux not available"));
            return;
        };
        match graft_engine::scion_start(&repo_path, name, config.as_ref(), &runtime) {
            Ok(()) => {
                self.status = Some(StatusMessage::success(format!("Started scion '{name}'")));
            }
            Err(e) => {
                self.status = Some(StatusMessage::error(format!("scion start failed: {e}")));
            }
        }
        self.context.cached_scion_completions = None;
    }

    /// `:scion stop <name>` — stop a scion's runtime session.
    fn cmd_scion_stop(&mut self, name: &str) {
        let Some(repo_path) = self.context.selected_repo_path.clone() else {
            self.status = Some(StatusMessage::warning("No repository selected"));
            return;
        };
        let Ok(runtime) = graft_common::TmuxRuntime::new() else {
            self.status = Some(StatusMessage::error("tmux not available"));
            return;
        };
        match graft_engine::scion_stop(&repo_path, name, &runtime) {
            Ok(()) => {
                self.status = Some(StatusMessage::success(format!("Stopped scion '{name}'")));
            }
            Err(e) => {
                self.status = Some(StatusMessage::error(format!("scion stop failed: {e}")));
            }
        }
        self.context.cached_scion_completions = None;
    }

    /// `:scion prune <name>` — remove a scion.
    fn cmd_scion_prune(&mut self, name: &str) {
        let Some(repo_path) = self.context.selected_repo_path.clone() else {
            self.status = Some(StatusMessage::warning("No repository selected"));
            return;
        };
        let config =
            graft_engine::parse_graft_yaml(std::path::Path::new(&repo_path).join("graft.yaml"))
                .ok();
        let dep_configs = config
            .as_ref()
            .map(|c| graft_engine::load_dep_configs(&repo_path, c))
            .unwrap_or_default();
        let runtime = graft_common::TmuxRuntime::new().ok();
        let runtime_ref = runtime.as_ref().map(|r| r as &dyn SessionRuntime);
        match graft_engine::scion_prune(
            &repo_path,
            name,
            config.as_ref(),
            &dep_configs,
            runtime_ref,
            false,
        ) {
            Ok(()) => {
                self.status = Some(StatusMessage::success(format!("Pruned scion '{name}'")));
            }
            Err(e) => {
                self.status = Some(StatusMessage::error(format!("scion prune failed: {e}")));
            }
        }
        self.context.cached_scion_completions = None;
    }

    /// `:scion fuse <name>` — fuse a scion into main.
    fn cmd_scion_fuse(&mut self, name: &str) {
        let Some(repo_path) = self.context.selected_repo_path.clone() else {
            self.status = Some(StatusMessage::warning("No repository selected"));
            return;
        };
        let config =
            graft_engine::parse_graft_yaml(std::path::Path::new(&repo_path).join("graft.yaml"))
                .ok();
        let dep_configs = config
            .as_ref()
            .map(|c| graft_engine::load_dep_configs(&repo_path, c))
            .unwrap_or_default();
        let runtime = graft_common::TmuxRuntime::new().ok();
        let runtime_ref = runtime.as_ref().map(|r| r as &dyn SessionRuntime);
        match graft_engine::scion_fuse(
            &repo_path,
            name,
            config.as_ref(),
            &dep_configs,
            runtime_ref,
            false,
        ) {
            Ok(commit) => {
                self.scroll.push(ContentBlock::Text {
                    id: BlockId::new(),
                    lines: vec![
                        Line::from(Span::styled(
                            format!("Fused scion '{name}' into main"),
                            Style::default().fg(Color::Green),
                        )),
                        Line::from(Span::styled(
                            format!("  merge commit: {commit}"),
                            Style::default().fg(Color::Gray),
                        )),
                    ],
                    collapsed: false,
                });
            }
            Err(e) => {
                self.status = Some(StatusMessage::error(format!("scion fuse failed: {e}")));
            }
        }
        self.context.cached_scion_completions = None;
    }

    /// `:attach <name>` — attach to a scion's runtime session.
    fn cmd_attach(&mut self, name: &str) {
        let Some(repo_path) = self.context.selected_repo_path.clone() else {
            self.status = Some(StatusMessage::warning("No repository selected"));
            return;
        };
        let Ok(runtime) = graft_common::TmuxRuntime::new() else {
            self.status = Some(StatusMessage::error("tmux not available"));
            return;
        };
        let session_id = match graft_engine::scion_attach_check(&repo_path, name, &runtime) {
            Ok(id) => id,
            Err(e) => {
                self.status = Some(StatusMessage::error(format!("{e}")));
                return;
            }
        };
        // Suspend TUI for blocking attach
        crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen).ok();
        crossterm::terminal::disable_raw_mode().ok();
        let attach_result = runtime.attach(&session_id);
        // Resume TUI
        crossterm::terminal::enable_raw_mode().ok();
        crossterm::execute!(std::io::stdout(), crossterm::terminal::EnterAlternateScreen).ok();
        self.needs_refresh = true;
        if let Err(e) = attach_result {
            self.status = Some(StatusMessage::error(format!("attach failed: {e}")));
        }
    }

    /// `:review <name> [full]` — review a scion's changes.
    #[allow(clippy::too_many_lines)]
    fn cmd_review(&mut self, name: &str, full: bool) {
        let Some(repo_path) = self.context.selected_repo_path.clone() else {
            self.status = Some(StatusMessage::warning("No repository selected"));
            return;
        };
        let repo = std::path::Path::new(&repo_path);

        // Get worktrees and base branch
        let worktrees = match graft_common::git_worktree_list(repo) {
            Ok(wts) => wts,
            Err(e) => {
                self.status = Some(StatusMessage::error(format!(
                    "Failed to list worktrees: {e}"
                )));
                return;
            }
        };
        let base = match graft_engine::resolve_base_branch(&worktrees) {
            Ok(b) => b,
            Err(e) => {
                self.status = Some(StatusMessage::error(format!("{e}")));
                return;
            }
        };

        // Check scion exists
        let wt_path = graft_engine::worktree_path(repo, name);
        if !wt_path.exists() {
            self.status = Some(StatusMessage::error(format!(
                "scion '{name}' does not exist"
            )));
            return;
        }

        let branch = graft_engine::branch_name(name);

        // Check ahead count
        let (ahead, _behind) = match graft_common::git_ahead_behind(repo, &branch, &base) {
            Ok(counts) => counts,
            Err(e) => {
                self.status = Some(StatusMessage::error(format!(
                    "Failed to compute ahead/behind: {e}"
                )));
                return;
            }
        };

        if ahead == 0 {
            self.scroll.push(ContentBlock::Text {
                id: BlockId::new(),
                lines: vec![Line::from(Span::styled(
                    format!("Scion '{name}' has no changes to review (0 commits ahead)"),
                    Style::default().fg(Color::Gray),
                ))],
                collapsed: false,
            });
            return;
        }

        let mut lines = vec![Line::from(vec![
            Span::styled(
                format!("Review: {name}"),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" ({ahead} commit{})", if ahead == 1 { "" } else { "s" }),
                Style::default().fg(Color::Gray),
            ),
        ])];
        lines.push(Line::from(""));

        // Commit log
        if let Ok(log) = graft_common::git_log_output(repo, &base, &branch) {
            if !log.is_empty() {
                lines.push(Line::from(Span::styled(
                    "Commits:",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )));
                for line in log.lines() {
                    lines.push(Line::from(Span::styled(
                        format!("  {line}"),
                        Style::default().fg(Color::White),
                    )));
                }
                lines.push(Line::from(""));
            }
        }

        // Diff stat or full diff
        if full {
            if let Ok(diff) = graft_common::git_diff_output(repo, &base, &branch) {
                lines.push(Line::from(Span::styled(
                    "Diff:",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )));
                for line in diff.lines() {
                    let style = if line.starts_with('+') && !line.starts_with("+++") {
                        Style::default().fg(Color::Green)
                    } else if line.starts_with('-') && !line.starts_with("---") {
                        Style::default().fg(Color::Red)
                    } else if line.starts_with("@@") {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    lines.push(Line::from(Span::styled(format!("  {line}"), style)));
                }
            }
        } else if let Ok(stat) = graft_common::git_diff_stat(repo, &base, &branch) {
            if !stat.is_empty() {
                lines.push(Line::from(Span::styled(
                    "Changes:",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )));
                for line in stat.lines() {
                    lines.push(Line::from(Span::styled(
                        format!("  {line}"),
                        Style::default().fg(Color::White),
                    )));
                }
                lines.push(Line::from(""));
            }
        }

        // Follow-up suggestions
        lines.push(Line::from(Span::styled(
            "Tip: :review <name> full  for full diff  |  :scion fuse <name>  to merge",
            Style::default().fg(Color::DarkGray),
        )));

        self.scroll.push(ContentBlock::Text {
            id: BlockId::new(),
            lines,
            collapsed: false,
        });
    }

    // ===== Refresh =====

    pub(super) fn handle_refresh_if_needed(&mut self) {
        if !self.needs_refresh {
            return;
        }

        match self.registry.refresh_all() {
            Ok(stats) => {
                self.status = if stats.all_successful() {
                    Some(StatusMessage::success(format!(
                        "Refreshed {} repositories",
                        stats.successful
                    )))
                } else {
                    Some(StatusMessage::warning(format!(
                        "Refreshed {}/{} repositories ({} errors)",
                        stats.successful,
                        stats.total(),
                        stats.failed
                    )))
                };
            }
            Err(e) => {
                self.status = Some(StatusMessage::error(format!("Refresh failed: {e}")));
            }
        }

        self.context.invalidate_caches();
        self.needs_refresh = false;
    }

    /// Clear expired status messages.
    pub(super) fn clear_expired_status(&mut self) {
        if let Some(msg) = &self.status {
            if msg.is_expired() {
                self.status = None;
            }
        }
    }

    /// Handle incoming command output events.
    ///
    /// Output lines are streamed into the active `ContentBlock::Running` in the
    /// scroll buffer. When the process finishes the block is finalized (converted
    /// to a static `Text` block) with elapsed time stamped in the header.
    pub(super) fn handle_command_events(&mut self) {
        let mut should_close = false;
        let mut output_lines: Vec<Line<'static>> = Vec::new();
        let mut completion: Option<super::scroll_buffer::RunCompletion> = None;

        if let Some(rx) = &self.execution.command_event_rx {
            while let Ok(event) = rx.try_recv() {
                match event {
                    CommandEvent::Started(pid) => {
                        self.execution.running_command_pid = Some(pid);
                    }
                    CommandEvent::LogPath(path) => {
                        self.execution.current_log_path = Some(path);
                    }
                    CommandEvent::OutputLine(line) => {
                        output_lines.push(Line::from(line));
                    }
                    CommandEvent::Completed(exit_code) => {
                        self.execution.command_state = CommandState::Completed { exit_code };
                        completion = Some(super::scroll_buffer::RunCompletion::Exited(exit_code));
                        should_close = true;
                    }
                    CommandEvent::Failed(error) => {
                        self.execution.command_state = CommandState::Failed {
                            error: error.clone(),
                        };
                        completion = Some(super::scroll_buffer::RunCompletion::Error(error));
                        should_close = true;
                    }
                }
            }
        }

        // Flush output lines into the Running block before potentially finalizing it.
        if !output_lines.is_empty() {
            if let Some(id) = self.execution.active_output_block {
                self.scroll.append_lines_to_running(id, output_lines);
            }
        }

        // Finalize the Running block if the command has finished.
        if let Some(outcome) = completion {
            if let Some(id) = self.execution.active_output_block.take() {
                self.scroll.finalize_running(id, &outcome);
            }
        }

        if should_close {
            self.execution.command_event_rx = None;
        }
    }

    // ===== Rendering =====

    pub(super) fn render(
        &mut self,
        terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    ) -> anyhow::Result<()> {
        self.clear_expired_status();
        self.handle_command_events();

        terminal.draw(|frame| {
            // Paint black background
            frame.render_widget(
                Block::default().style(Style::default().bg(Color::Black)),
                frame.area(),
            );

            // Layout: header (2) + content (flex) + status/prompt (1)
            let main_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(2), // Header
                    Constraint::Min(3),    // Scroll buffer
                    Constraint::Length(1), // Prompt / status
                ])
                .split(frame.area());

            let header_area = main_chunks[0];
            let content_area = main_chunks[1];
            let prompt_area = main_chunks[2];

            // Render header
            self.render_header(frame, header_area);

            // Render scroll buffer
            self.scroll.render(frame, content_area);

            // Render prompt or status bar
            if self.prompt.is_active() {
                let focus_opts = self.focus_entity_opts_for_buffer();
                let resolved = self.commands_with_resolved_options();
                let scion_comps = self.scion_completions();
                let cs = self.prompt.compute_completions(
                    &resolved,
                    &self.repo_basenames(),
                    &self.state_query_names(),
                    &focus_opts,
                    &scion_comps,
                );
                self.prompt.render_palette(frame, content_area, &cs);
                self.prompt.render_prompt(frame, prompt_area, &cs);
            } else if let Some(msg) = &self.status {
                // Render status message
                let unicode = super::supports_unicode();
                let symbol = msg.msg_type.symbol(unicode);
                let fg = msg.msg_type.fg_color();
                let bg = msg.msg_type.bg_color();
                let text = format!(" {symbol} {}", msg.text);
                let status_bar = Paragraph::new(text).style(Style::default().fg(fg).bg(bg));
                frame.render_widget(status_bar, prompt_area);
            } else {
                self.prompt
                    .render_prompt(frame, prompt_area, &CompletionState::default());
            }

            // Render picker overlay on top of everything else if active
            if let Some(picker) = &self.picker {
                picker.render(frame, content_area, " Select ");
            }
        })?;

        Ok(())
    }

    fn render_header(&self, frame: &mut ratatui::Frame, area: Rect) {
        let repos = self.registry.list_repos();
        let (repo_path, branch, is_dirty, ahead, behind) =
            if let Some(idx) = self.context.selected_index {
                if let Some(repo) = repos.get(idx) {
                    let path_str = repo.as_path().display().to_string();
                    let status = self.registry.get_status(repo);
                    (
                        Some(path_str),
                        status.and_then(|s| s.branch.clone()),
                        status.map(|s| s.is_dirty),
                        status.and_then(|s| s.ahead),
                        status.and_then(|s| s.behind),
                    )
                } else {
                    (None, None, None, None, None)
                }
            } else {
                (None, None, None, None, None)
            };

        let stale_focus = self.compute_stale_focus();
        let data = super::header::HeaderData {
            workspace_name: &self.workspace_name,
            repo_path: repo_path.as_deref(),
            branch: branch.as_deref(),
            is_dirty,
            ahead,
            behind,
            focus: &self.focus,
            stale_focus: &stale_focus,
        };
        super::header::render_header(frame, area, &data);
    }

    /// Determine which focused query values are stale by checking in-memory state
    /// opportunistically (no subprocess runs, no disk reads).
    pub(super) fn compute_stale_focus(&self) -> std::collections::HashSet<String> {
        let mut stale = std::collections::HashSet::new();
        for (query_name, focused_value) in &self.focus {
            if let Some(data) = self.context.in_memory_state.get(query_name.as_str()) {
                let entity = self
                    .context
                    .cached_state_queries
                    .as_ref()
                    .and_then(|qs| qs.iter().find(|q| q.name == *query_name))
                    .and_then(|q| q.entity.as_ref());
                let opts = extract_options_from_state(query_name, data, entity);
                if !opts.contains(focused_value) {
                    stale.insert(query_name.clone());
                }
            }
        }
        stale
    }

    // ===== Helpers =====

    /// Get state query names for argument hints.
    fn state_query_names(&self) -> Vec<String> {
        self.context
            .cached_state_queries
            .as_ref()
            .map(|qs| qs.iter().map(|q| q.name.clone()).collect())
            .unwrap_or_default()
    }

    /// Resolve entity options for the query the user is completing as the second arg of `:focus`.
    ///
    /// Returns a map with at most one entry: `{query_name → [option, ...]}`.
    /// Returns an empty map if the current buffer doesn't look like `focus <query> `.
    fn focus_entity_opts_for_buffer(&mut self) -> HashMap<String, Vec<String>> {
        let buffer = match self.prompt.command_line.as_ref() {
            Some(s) => s.text.buffer.clone(),
            None => return HashMap::new(),
        };

        // Parse "focus <rest>"
        let mut parts = buffer.splitn(2, char::is_whitespace);
        let cmd = parts.next().unwrap_or("").to_ascii_lowercase();
        if cmd != "focus" && cmd != "f" {
            return HashMap::new();
        }
        let rest = parts.next().unwrap_or("").trim_start().to_string();

        // Only resolve if there's a space in rest (second arg is being typed)
        if !rest.contains(' ') {
            return HashMap::new();
        }

        let query_name = match rest.split_whitespace().next() {
            Some(q) if !q.is_empty() => q.to_string(),
            _ => return HashMap::new(),
        };

        let repo_name = self
            .context
            .selected_repo_path
            .as_deref()
            .map(graft_common::repo_name_from_path)
            .unwrap_or_default()
            .to_string();

        let opts = self.resolve_options_from(&query_name, &repo_name);
        if opts.is_empty() {
            HashMap::new()
        } else {
            let mut map = HashMap::new();
            map.insert(query_name, opts);
            map
        }
    }

    /// Get basenames of all repos (for argument hints).
    fn repo_basenames(&self) -> Vec<String> {
        self.registry
            .list_repos()
            .iter()
            .map(|r| {
                let path = r.as_path().display().to_string();
                extract_basename(&path).to_string()
            })
            .collect()
    }

    /// Get scion name completions for the selected repo (cached).
    fn scion_completions(&mut self) -> Vec<super::prompt::ArgCompletion> {
        if let Some(cached) = &self.context.cached_scion_completions {
            return cached.clone();
        }
        let Some(repo_path) = self.context.selected_repo_path.clone() else {
            return Vec::new();
        };
        let runtime = graft_common::TmuxRuntime::new().ok();
        let runtime_ref = runtime.as_ref().map(|r| r as &dyn SessionRuntime);
        let Ok(scions) = graft_engine::scion_list(&repo_path, runtime_ref) else {
            self.context.cached_scion_completions = Some(Vec::new());
            return Vec::new();
        };
        let completions: Vec<super::prompt::ArgCompletion> = scions
            .iter()
            .map(|s| {
                let status = match (s.ahead, s.session_active) {
                    (Some(a), Some(true)) => format!("+{a} [session]"),
                    (Some(a), _) => format!("+{a}"),
                    _ => String::new(),
                };
                super::prompt::ArgCompletion {
                    value: s.name.clone(),
                    description: status,
                }
            })
            .collect();
        self.context.cached_scion_completions = Some(completions.clone());
        completions
    }

    /// Load sequences from a graft.yaml into the catalog entries list.
    ///
    /// If `dep_prefix` is `Some("dep_name")`, sequence names are prefixed as `dep_name:seq`.
    /// Sequence names are prefixed with `\u{00bb} ` to distinguish from commands.
    /// Parse errors are reported as status warnings; missing files are silently skipped.
    fn load_sequences_into(
        &mut self,
        yaml_path: &std::path::Path,
        dep_prefix: Option<&str>,
        entries: &mut Vec<(String, String, String)>,
    ) {
        let Ok(content) = std::fs::read_to_string(yaml_path) else {
            return; // File doesn't exist — not an error
        };
        match graft_common::parse_sequences_from_str(&content) {
            Ok(sequences) => {
                for (name, seq) in sequences {
                    let display_name = match dep_prefix {
                        Some(dep) => format!("\u{00bb} {dep}:{name}"),
                        None => format!("\u{00bb} {name}"),
                    };
                    entries.push((
                        display_name,
                        seq.description.unwrap_or_default(),
                        seq.category.unwrap_or_else(|| "uncategorized".to_string()),
                    ));
                }
            }
            Err(e) => {
                let label = dep_prefix.unwrap_or("graft.yaml");
                self.status = Some(StatusMessage::warning(format!(
                    "Failed to parse sequences from {label}: {e}"
                )));
            }
        }
    }

    /// Load available commands for the currently selected repo.
    fn load_commands_for_repo(&mut self) {
        let Some(repo_path) = &self.context.selected_repo_path else {
            return;
        };

        let mut commands = Vec::new();

        // Load local commands
        let repo_base = PathBuf::from(repo_path);
        let graft_yaml_path = repo_base.join("graft.yaml");
        match graft_common::parse_commands(&graft_yaml_path) {
            Ok(cmds) => {
                for (name, cmd) in cmds {
                    commands.push((name, cmd));
                }
            }
            Err(e) => {
                self.status = Some(StatusMessage::warning(format!(
                    "Failed to parse graft.yaml: {e}"
                )));
            }
        }

        // Load dep commands
        let graft_dir = repo_base.join(".graft");
        if let Ok(entries) = std::fs::read_dir(&graft_dir) {
            for entry in entries.flatten() {
                let dep_name = entry.file_name().to_string_lossy().to_string();
                // Skip non-directories and special dirs
                if !entry.file_type().is_ok_and(|ft| ft.is_dir()) {
                    continue;
                }
                if dep_name == "run-state" || dep_name == "runs" {
                    continue;
                }
                let dep_yaml = graft_dir.join(&dep_name).join("graft.yaml");
                match graft_common::parse_commands(&dep_yaml) {
                    Ok(cmds) => {
                        for (name, cmd) in cmds {
                            commands.push((format!("{dep_name}:{name}"), cmd));
                        }
                    }
                    Err(e) => {
                        self.status = Some(StatusMessage::warning(format!(
                            "Failed to parse {dep_name}/graft.yaml: {e}"
                        )));
                    }
                }
            }
        }

        // Sort commands by name
        commands.sort_by(|a, b| a.0.cmp(&b.0));

        self.context.available_commands = commands;
        // Invalidate the resolved-options cache whenever the command list is replaced.
        self.context.resolved_commands = None;
    }

    /// Return `available_commands` with `options_from` args resolved to their live values.
    ///
    /// For each command arg with `options_from` set and no static `options`, attempts to
    /// resolve the state query result (disk cache → in-memory cache → run fresh). Commands
    /// with only static options or no args are returned as-is (cloned).
    ///
    /// The result is cached in `context.resolved_commands` to avoid re-running subprocesses
    /// on every key event or render frame. The cache is cleared after `:run` and on repo switch.
    fn commands_with_resolved_options(&mut self) -> Vec<(String, graft_common::CommandDef)> {
        if let Some(ref cached) = self.context.resolved_commands {
            return cached.clone();
        }

        let repo_name = self
            .context
            .selected_repo_path
            .as_deref()
            .map(graft_common::repo_name_from_path)
            .unwrap_or_default()
            .to_string();

        // Collect the list of (name, query_name) pairs that need patching first,
        // to avoid holding a borrow on self.context while we call resolve_options_from.
        let to_patch: Vec<(String, Vec<String>)> = self
            .context
            .available_commands
            .iter()
            .filter_map(|(name, def)| {
                let query_names: Vec<String> = def
                    .args
                    .as_ref()
                    .map(|args| {
                        args.iter()
                            .filter(|a| a.options_from.is_some() && a.options.is_none())
                            .filter_map(|a| a.options_from.clone())
                            .collect()
                    })
                    .unwrap_or_default();
                if query_names.is_empty() {
                    None
                } else {
                    Some((name.clone(), query_names))
                }
            })
            .collect();

        // Resolve all needed query names up-front (may run subprocess).
        let mut resolved_map: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for (_, query_names) in &to_patch {
            for query_name in query_names {
                if !resolved_map.contains_key(query_name) {
                    let opts = self.resolve_options_from(query_name, &repo_name);
                    resolved_map.insert(query_name.clone(), opts);
                }
            }
        }

        // Build the patched command list.
        let result: Vec<(String, graft_common::CommandDef)> = self
            .context
            .available_commands
            .iter()
            .map(|(name, def)| {
                let needs_patch = def.args.as_ref().is_some_and(|args| {
                    args.iter()
                        .any(|a| a.options_from.is_some() && a.options.is_none())
                });
                if !needs_patch {
                    return (name.clone(), def.clone());
                }
                let mut patched = def.clone();
                if let Some(ref mut args) = patched.args {
                    for arg in args.iter_mut() {
                        if arg.options.is_none() {
                            if let Some(query_name) = &arg.options_from {
                                if let Some(opts) = resolved_map.get(query_name) {
                                    arg.options = Some(opts.clone());
                                }
                            }
                        }
                    }
                }
                (name.clone(), patched)
            })
            .collect();

        // Cache the result so subsequent calls (key events, render frames) are instant.
        self.context.resolved_commands = Some(result.clone());
        result
    }

    /// Resolve a list of string options for `query_name`.
    ///
    /// Lookup order:
    /// 1. Latest disk cache (`read_latest_cached`)
    /// 2. In-memory cache (populated from a previous fresh run this session)
    /// 3. Run the query's bash command as a subprocess (result cached in-memory)
    ///
    /// The `entity` declaration (if any) is threaded through to `extract_options_from_state`
    /// for all three paths so that entity-aware extraction is used consistently.
    fn resolve_options_from(&mut self, query_name: &str, repo_name: &str) -> Vec<String> {
        // Lazily discover state queries to obtain the entity declaration.
        // We do this up-front so all three cache paths can use it.
        let repo_path_opt = self.context.selected_repo_path.clone();
        if self.context.cached_state_queries.is_none() {
            if let Some(ref repo_path) = repo_path_opt {
                let (queries, _) =
                    crate::state::discover_all_state_queries(&PathBuf::from(repo_path));
                self.context.cached_state_queries = Some(queries);
            }
        }

        // Clone the entity so we can pass it through without holding a borrow.
        let entity = self
            .context
            .cached_state_queries
            .as_ref()
            .and_then(|queries| queries.iter().find(|q| q.name == query_name))
            .and_then(|q| q.entity.clone());

        // 1. Try disk cache (may have been written by a previous `graft run … verify`)
        if let Some(result) =
            graft_common::read_latest_cached(&self.workspace_name, repo_name, query_name)
        {
            return extract_options_from_state(query_name, &result.data, entity.as_ref());
        }

        // 2. Try in-memory cache (populated by an earlier fresh run this session)
        if let Some(data) = self.context.in_memory_state.get(query_name).cloned() {
            return extract_options_from_state(query_name, &data, entity.as_ref());
        }

        // 3. Fall back: run the query command as a subprocess (need repo selected)
        if repo_path_opt.is_none() {
            return Vec::new();
        }

        let query = self
            .context
            .cached_state_queries
            .as_ref()
            .and_then(|queries| queries.iter().find(|q| q.name == query_name))
            .cloned();

        let Some(query) = query else {
            return Vec::new();
        };

        let config = graft_common::ProcessConfig {
            command: query.run,
            working_dir: query.working_dir,
            env: None,
            env_remove: vec![],
            log_path: None,
            timeout: Some(std::time::Duration::from_secs(30)),
            stdin: None,
        };

        let output = match graft_common::run_to_completion_with_timeout(&config) {
            Ok(out) if out.success => out,
            _ => return Vec::new(),
        };

        let data: serde_json::Value = match serde_json::from_str(&output.stdout) {
            Ok(v) => v,
            Err(_) => return Vec::new(),
        };

        // Store in in-memory cache for subsequent keypresses this session
        self.context
            .in_memory_state
            .insert(query_name.to_string(), data.clone());

        extract_options_from_state(query_name, &data, entity.as_ref())
    }
}

/// Extract a flat list of string options from a state query result.
///
/// When `entity` is `Some`, uses `entity.collection` (falling back to `query_name`) as the
/// JSON array key and `entity.key` to extract the identity value from each object.
///
/// When `entity` is `None`, preserves the existing hardcoded behavior: looks for a
/// top-level array under `query_name`; bare strings are used as-is; objects with a `path`
/// field yield the parent directory; objects with a `name` field yield that name; items
/// with `status == "done"` are skipped.
pub(super) fn extract_options_from_state(
    query_name: &str,
    data: &serde_json::Value,
    entity: Option<&graft_common::EntityDef>,
) -> Vec<String> {
    if let Some(entity) = entity {
        // Entity-aware extraction: use declared collection key and identity field.
        let collection_key = entity.collection.as_deref().unwrap_or(query_name);
        let Some(arr) = data.get(collection_key).and_then(|v| v.as_array()) else {
            return Vec::new();
        };
        return arr
            .iter()
            .filter_map(|item| {
                item.get(&entity.key)
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string)
            })
            .collect();
    }

    // Hardcoded legacy extraction (backward compatible).
    let Some(arr) = data.get(query_name).and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|item| {
            // Skip items explicitly marked as done (e.g. completed slices)
            if item.get("status").and_then(|v| v.as_str()) == Some("done") {
                return None;
            }
            if let Some(s) = item.as_str() {
                Some(s.to_string())
            } else if let Some(path) = item.get("path").and_then(|v| v.as_str()) {
                // Strip the trailing filename (e.g. "slices/foo/plan.md" → "slices/foo")
                Some(
                    path.rsplit_once('/')
                        .map_or(path, |(dir, _)| dir)
                        .to_string(),
                )
            } else {
                item.get("name")
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string)
            }
        })
        .collect()
}

/// Helper to build a help line.
fn help_line(cmd: &str, desc: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {cmd:<22}"), Style::default().fg(Color::Cyan)),
        Span::styled(desc.to_string(), Style::default().fg(Color::Gray)),
    ])
}
