//! Transcript-paradigm TUI: scrolling content area with command prompt.
//!
//! Replaces the old spatial dashboard with a single scrolling transcript.
//! Every action is triggered from the prompt, results appear as blocks in the scroll buffer.

use crossterm::event::KeyCode;
use graft_common::CommandDef;
use grove_core::{CommandState, RepoDetail, RepoDetailProvider, RepoRegistry};
use grove_engine::GraftYamlConfigLoader;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::Instant;

use super::command_exec::CommandEvent;
use super::command_line::CliCommand;
use super::formatting::{extract_basename, format_file_change_indicator};
use super::prompt::{CompletionState, PromptState};
use super::scroll_buffer::{BlockId, ContentBlock, ScrollBuffer};
use super::status_bar::StatusMessage;

/// Default number of recent commits to show.
const DEFAULT_MAX_COMMITS: usize = 10;

/// Maximum lines of command output to buffer.
const MAX_OUTPUT_LINES: usize = 10_000;

/// Number of lines to drop when buffer is full.
const LINES_TO_DROP: usize = 1_000;

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
        }
    }

    /// Invalidate caches but keep the current repo selection.
    fn invalidate_caches(&mut self) {
        self.cached_detail = None;
        self.cached_detail_index = None;
        self.available_commands.clear();
        self.cached_state_queries = None;
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
    pub(super) output_lines: Vec<String>,
    pub(super) output_scroll: usize,
    pub(super) output_truncated_start: bool,
    pub(super) command_start_time: Option<Instant>,
    pub(super) current_log_path: Option<std::path::PathBuf>,
    #[allow(dead_code)]
    pub(super) active_output_block: Option<BlockId>,
}

impl ExecutionState {
    fn new() -> Self {
        Self {
            command_event_rx: None,
            running_command_pid: None,
            command_state: CommandState::NotStarted,
            command_name: None,
            output_lines: Vec::new(),
            output_scroll: 0,
            output_truncated_start: false,
            command_start_time: None,
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

    // Execution
    pub(super) execution: ExecutionState,

    // Status
    pub(super) status: Option<StatusMessage>,

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
            execution: ExecutionState::new(),
            status: None,
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
    pub(super) fn handle_key(&mut self, code: KeyCode, modifiers: crossterm::event::KeyModifiers) {
        // Command line intercepts all keys when active
        if self.prompt.is_active() {
            let cs = self.prompt.compute_completions(
                &self.context.available_commands,
                &self.repo_basenames(),
                &self.state_query_names(),
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
            CliCommand::Run(command_name, args) => self.cmd_run(&command_name, args),
            CliCommand::Status => self.cmd_status(),
            CliCommand::Catalog(cat) => self.cmd_catalog(cat.as_deref()),
            CliCommand::State(name) => self.cmd_state(name.as_deref()),
            CliCommand::Invalidate(name) => self.cmd_invalidate(name.as_deref()),
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

        let headers = vec![
            "#".to_string(),
            "Repository".to_string(),
            "Branch".to_string(),
            "Status".to_string(),
        ];

        let mut rows = Vec::new();
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

            rows.push(vec![
                Span::styled(format!("{}", i + 1), idx_style),
                Span::styled(basename, name_style),
                branch,
                dirty,
            ]);
        }

        self.scroll.push(ContentBlock::Table {
            id: BlockId::new(),
            title: "Repositories".to_string(),
            headers,
            rows,
            collapsed: false,
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
            help_line("Enter", "Toggle collapse on focused block"),
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
        self.execution.output_lines.clear();
        self.execution.output_scroll = 0;
        self.execution.output_truncated_start = false;
        self.execution.running_command_pid = None;
        self.execution.command_start_time = Some(Instant::now());
        self.execution.current_log_path = None;

        let cmd_name = command_name.to_string();
        let repo = repo_path;
        std::thread::spawn(move || {
            super::command_exec::spawn_command(cmd_name, args, repo, run_ctx, tx);
        });

        // Push a status line for the running command
        self.scroll.push(ContentBlock::Text {
            id: BlockId::new(),
            lines: vec![Line::from(vec![
                Span::styled("\u{25b6} Running: ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    command_name.to_string(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ])],
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

        let rows: Vec<Vec<Span<'static>>> = entries
            .into_iter()
            .map(|(name, desc, cat)| {
                vec![
                    Span::styled(name, Style::default().fg(Color::Cyan)),
                    Span::styled(desc, Style::default().fg(Color::White)),
                    Span::styled(cat, Style::default().fg(Color::DarkGray)),
                ]
            })
            .collect();

        self.scroll.push(ContentBlock::Table {
            id: BlockId::new(),
            title: "Catalog".to_string(),
            headers,
            rows,
            collapsed: false,
        });
    }

    /// `:state [name]` — show cached state queries.
    #[allow(clippy::too_many_lines)]
    fn cmd_state(&mut self, name: Option<&str>) {
        let Some(repo_path) = self.context.selected_repo_path.clone() else {
            self.status = Some(StatusMessage::warning("No repository selected"));
            return;
        };

        // Discover and cache state queries
        if self.context.cached_state_queries.is_none() {
            let graft_yaml_path = PathBuf::from(&repo_path).join("graft.yaml");
            match crate::state::discover_state_queries(&graft_yaml_path) {
                Ok(queries) => {
                    self.context.cached_state_queries = Some(queries);
                }
                Err(e) => {
                    self.status = Some(StatusMessage::warning(format!(
                        "Failed to discover state queries: {e}"
                    )));
                    self.context.cached_state_queries = Some(Vec::new());
                }
            }
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
                    "Deterministic".to_string(),
                ];

                let rows: Vec<Vec<Span<'static>>> = queries
                    .iter()
                    .map(|q| {
                        let cached = graft_common::read_latest_cached(
                            &self.workspace_name,
                            repo_name,
                            &q.name,
                        );
                        let (summary, age) = match &cached {
                            Some(result) => (
                                crate::state::format_state_summary(result),
                                result.metadata.time_ago(),
                            ),
                            None => ("(not cached)".to_string(), "-".to_string()),
                        };
                        let det = if q.deterministic { "yes" } else { "no" };

                        vec![
                            Span::styled(q.name.clone(), Style::default().fg(Color::Cyan)),
                            Span::styled(summary, Style::default().fg(Color::White)),
                            Span::styled(age, Style::default().fg(Color::DarkGray)),
                            Span::styled(det.to_string(), Style::default().fg(Color::DarkGray)),
                        ]
                    })
                    .collect();

                self.scroll.push(ContentBlock::Table {
                    id: BlockId::new(),
                    title: "State Queries".to_string(),
                    headers,
                    rows,
                    collapsed: false,
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
    /// Output lines are streamed directly into the scroll buffer's last Text block
    /// (the "Running:" block created by `cmd_run`), making output visible in real time.
    pub(super) fn handle_command_events(&mut self) {
        let mut should_close = false;
        let mut new_lines: Vec<Line<'static>> = Vec::new();

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
                        self.execution.output_lines.push(line.clone());
                        new_lines.push(Line::from(line));

                        // Truncate if buffer is too large
                        if self.execution.output_lines.len() > MAX_OUTPUT_LINES {
                            self.execution.output_lines.drain(0..LINES_TO_DROP);
                            self.execution.output_truncated_start = true;
                        }
                    }
                    CommandEvent::Completed(exit_code) => {
                        self.execution.command_state = CommandState::Completed { exit_code };

                        let unicode = super::supports_unicode();
                        new_lines.push(Line::from(""));
                        if exit_code == 0 {
                            let symbol = if unicode { "\u{2713}" } else { "*" };
                            new_lines.push(Line::from(Span::styled(
                                format!("{symbol} Completed successfully"),
                                Style::default().fg(Color::Green),
                            )));
                        } else {
                            let symbol = if unicode { "\u{2717}" } else { "X" };
                            new_lines.push(Line::from(Span::styled(
                                format!("{symbol} Failed with exit code {exit_code}"),
                                Style::default().fg(Color::Red),
                            )));
                        }

                        should_close = true;
                    }
                    CommandEvent::Failed(error) => {
                        self.execution.command_state = CommandState::Failed {
                            error: error.clone(),
                        };

                        new_lines.push(Line::from(Span::styled(
                            format!("Error: {error}"),
                            Style::default().fg(Color::Red),
                        )));

                        should_close = true;
                    }
                }
            }
        }

        // Append any new output lines to the running command's block in the scroll buffer
        if !new_lines.is_empty() {
            self.scroll.append_lines_to_last(new_lines);
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
                let cs = self.prompt.compute_completions(
                    &self.context.available_commands,
                    &self.repo_basenames(),
                    &self.state_query_names(),
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

        let data = super::header::HeaderData {
            workspace_name: &self.workspace_name,
            repo_path: repo_path.as_deref(),
            branch: branch.as_deref(),
            is_dirty,
            ahead,
            behind,
        };
        super::header::render_header(frame, area, &data);
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
    }
}

/// Helper to build a help line.
fn help_line(cmd: &str, desc: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {cmd:<22}"), Style::default().fg(Color::Cyan)),
        Span::styled(desc.to_string(), Style::default().fg(Color::Gray)),
    ])
}
