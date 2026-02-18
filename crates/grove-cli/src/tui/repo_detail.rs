//! `RepoDetail` view: unified scrollable view with all sections.
//!
//! Replaces the tabbed detail pane — changes, commits, state queries, and
//! commands are all shown vertically in a single scrollable view.

use super::{
    format_file_change_indicator, App, ArgumentInputMode, ArgumentInputState, Block, Borders,
    Color, GraftYamlLoader, KeyCode, Line, ListState, Modifier, Paragraph, Rect,
    RepoDetailProvider, RepoRegistry, Span, StatusMessage, Style, View,
};

impl<R: RepoRegistry, D: RepoDetailProvider> App<R, D> {
    /// Handle keys when in the `RepoDetail` view.
    pub(super) fn handle_key_repo_detail(&mut self, code: KeyCode) {
        match code {
            // q pops back one level
            KeyCode::Char('q') | KeyCode::Tab => {
                self.pop_view();
            }
            // Escape goes home (Dashboard) from anywhere
            KeyCode::Esc => {
                self.reset_to_dashboard();
            }
            KeyCode::Char('?') => {
                self.push_view(View::Help);
            }
            // Scroll the unified detail view
            KeyCode::Char('j') | KeyCode::Down => {
                self.detail_scroll = self.detail_scroll.saturating_add(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.detail_scroll = self.detail_scroll.saturating_sub(1);
            }
            // Refresh state queries (r key)
            KeyCode::Char('r') => {
                self.refresh_selected_state_query();
            }
            // Execute selected command
            KeyCode::Enter => {
                self.execute_selected_command();
            }
            // Navigate command picker forward/back
            KeyCode::Char('n') => {
                let i = self.command_picker_state.selected().unwrap_or(0);
                if i + 1 < self.available_commands.len() {
                    self.command_picker_state.select(Some(i + 1));
                }
            }
            KeyCode::Char('p') => {
                let i = self.command_picker_state.selected().unwrap_or(0);
                if i > 0 {
                    self.command_picker_state.select(Some(i - 1));
                }
            }
            _ => {}
        }
    }

    /// Render the full-width `RepoDetail` view.
    pub(super) fn render_repo_detail_view(&mut self, frame: &mut ratatui::Frame, area: Rect) {
        // Load data needed for sections (lazy, cached)
        self.load_commands_for_selected_repo();
        self.ensure_state_loaded_if_needed();

        let repo_title = self.repo_detail_title();

        let block = Block::default()
            .title(repo_title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let lines = self.build_repo_detail_lines();

        // Clamp scroll to content height
        let inner_height = inner.height as usize;
        let max_scroll = lines.len().saturating_sub(inner_height);
        let clamped_scroll = self.detail_scroll.min(max_scroll);

        let paragraph =
            Paragraph::new(lines).scroll((u16::try_from(clamped_scroll).unwrap_or(u16::MAX), 0));

        frame.render_widget(paragraph, inner);
    }

    /// Build the title line for the detail view block.
    fn repo_detail_title(&self) -> Line<'static> {
        if let Some(index) = self.cached_detail_index {
            let repos = self.registry.list_repos();
            if let Some(repo_path) = repos.get(index) {
                let path_str = repo_path.as_path().display().to_string();

                if let Some(status) = self.registry.get_status(repo_path) {
                    let branch = status
                        .branch
                        .as_ref()
                        .map_or_else(|| "[detached]".to_string(), Clone::clone);
                    let dirty = if status.is_dirty { " ●" } else { " ○" };
                    let dirty_color = if status.is_dirty {
                        Color::Yellow
                    } else {
                        Color::Green
                    };

                    let mut spans = vec![
                        Span::raw(" "),
                        Span::styled(path_str, Style::default().fg(Color::White)),
                        Span::styled(
                            format!(" ── {branch}"),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled(dirty, Style::default().fg(dirty_color)),
                    ];

                    if let Some(ahead) = status.ahead.filter(|&n| n > 0) {
                        spans.push(Span::styled(
                            format!(" ↑{ahead}"),
                            Style::default().fg(Color::Green),
                        ));
                    }
                    if let Some(behind) = status.behind.filter(|&n| n > 0) {
                        spans.push(Span::styled(
                            format!(" ↓{behind}"),
                            Style::default().fg(Color::Red),
                        ));
                    }

                    spans.push(Span::raw(" "));
                    return Line::from(spans);
                }

                return Line::from(vec![
                    Span::raw(" "),
                    Span::styled(path_str, Style::default().fg(Color::White)),
                    Span::raw(" "),
                ]);
            }
        }

        Line::from(Span::styled(
            " Repository Detail ",
            Style::default().fg(Color::White),
        ))
    }

    /// Build all lines for the unified detail view.
    pub(super) fn build_repo_detail_lines(&self) -> Vec<Line<'static>> {
        let mut lines: Vec<Line<'static>> = Vec::new();

        // Section 1: Changed Files + Recent Commits
        self.append_changes_section(&mut lines);

        lines.push(Line::from(""));

        // Section 2: State Queries
        self.append_state_section(&mut lines);

        lines.push(Line::from(""));

        // Section 3: Commands
        self.append_commands_section(&mut lines);

        lines
    }

    /// Append changed files and recent commits.
    fn append_changes_section(&self, lines: &mut Vec<Line<'static>>) {
        let Some(detail) = &self.cached_detail else {
            lines.push(Line::from(Span::styled(
                "No repository selected",
                Style::default().fg(Color::Gray),
            )));
            return;
        };

        // Show error as warning if present (but continue rendering partial data)
        if let Some(error) = &detail.error {
            lines.push(Line::from(Span::styled(
                format!("Error: {error}"),
                Style::default().fg(Color::Red),
            )));
            lines.push(Line::from(""));
        }

        // Changed files
        if detail.changed_files.is_empty() {
            lines.push(Line::from(Span::styled(
                "No uncommitted changes",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                format!("Changed Files ({})", detail.changed_files.len()),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));

            for change in &detail.changed_files {
                let (indicator, color) = format_file_change_indicator(&change.status);
                lines.push(Line::from(vec![
                    Span::styled(format!("  {indicator} "), Style::default().fg(color)),
                    Span::styled(change.path.clone(), Style::default().fg(Color::White)),
                ]));
            }
        }

        lines.push(Line::from(""));

        // Recent commits
        if detail.commits.is_empty() {
            lines.push(Line::from(Span::styled(
                "No commits",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                format!("Recent Commits ({})", detail.commits.len()),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )));

            for commit in &detail.commits {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {} ", commit.hash),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::styled(commit.subject.clone(), Style::default().fg(Color::White)),
                ]));
                lines.push(Line::from(Span::styled(
                    format!("       {} - {}", commit.author, commit.relative_date),
                    Style::default().fg(Color::DarkGray),
                )));
            }
        }
    }

    /// Append state queries section.
    fn append_state_section(&self, lines: &mut Vec<Line<'static>>) {
        lines.push(Line::from(Span::styled(
            "State Queries",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));

        if self.state_queries.is_empty() {
            lines.push(Line::from(Span::styled(
                "  No state queries defined in graft.yaml",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            for (idx, query) in self.state_queries.iter().enumerate() {
                if let Some(Some(result)) = self.state_results.get(idx) {
                    let age = result.metadata.time_ago();
                    let data_summary = result.summary();
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!("  {:<14}", query.name),
                            Style::default().fg(Color::Cyan),
                        ),
                        Span::raw("  "),
                        Span::raw(format!("{data_summary:<45}")),
                        Span::styled(format!("({age})"), Style::default().fg(Color::DarkGray)),
                    ]));
                } else {
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!("  {:<14}", query.name),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::raw("  "),
                        Span::styled("(no cached data)", Style::default().fg(Color::DarkGray)),
                    ]));
                }
            }
        }
    }

    /// Append commands section.
    fn append_commands_section(&self, lines: &mut Vec<Line<'static>>) {
        lines.push(Line::from(Span::styled(
            "Commands",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )));

        if self.available_commands.is_empty() {
            lines.push(Line::from(Span::styled(
                "  No commands defined in graft.yaml",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            let selected_idx = self.command_picker_state.selected();
            for (i, (name, cmd)) in self.available_commands.iter().enumerate() {
                let desc = cmd.description.as_deref().unwrap_or("");
                let is_selected = selected_idx == Some(i);
                if is_selected {
                    lines.push(Line::from(vec![
                        Span::styled("▶ ", Style::default().fg(Color::Cyan)),
                        Span::styled(
                            format!("{name:<20} {desc}"),
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]));
                } else {
                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(
                            format!("{name:<20} {desc}"),
                            Style::default().fg(Color::White),
                        ),
                    ]));
                }
            }
        }
    }

    /// Ensure state queries are loaded for the current repo (lazy, only if empty).
    fn ensure_state_loaded_if_needed(&mut self) {
        if self.state_queries.is_empty() {
            if let Some(selected) = self.list_state.selected() {
                let repos = self.registry.list_repos();
                if let Some(repo) = repos.get(selected) {
                    let repo_path_str = repo.as_path().to_str().unwrap_or("").to_string();
                    self.load_state_queries(&repo_path_str);
                }
            }
        }
    }

    /// Load commands for the currently selected repository (cached).
    pub(super) fn load_commands_for_selected_repo(&mut self) {
        let Some(selected) = self.list_state.selected() else {
            return;
        };

        let repos = self.registry.list_repos();
        if selected >= repos.len() {
            return;
        }

        let repo_path = repos[selected].as_path().display().to_string();

        // Check cache — avoid re-parsing if same repo
        if self.selected_repo_for_commands.as_ref() == Some(&repo_path) {
            return;
        }

        let graft_path = format!("{repo_path}/graft.yaml");
        let graft_config = match self.graft_loader.load_graft(&graft_path) {
            Ok(config) => config,
            Err(e) => {
                self.status_message = Some(StatusMessage::error(format!(
                    "Error loading graft.yaml: {e}"
                )));
                self.available_commands.clear();
                self.selected_repo_for_commands = Some(repo_path);
                return;
            }
        };

        self.available_commands = graft_config.commands.into_iter().collect();
        self.available_commands.sort_by(|a, b| a.0.cmp(&b.0));
        self.selected_repo_for_commands = Some(repo_path);

        if !self.available_commands.is_empty() {
            self.command_picker_state.select(Some(0));
        }
    }

    /// Execute the currently selected command.
    pub(super) fn execute_selected_command(&mut self) {
        let Some(cmd_idx) = self.command_picker_state.selected() else {
            return;
        };

        if cmd_idx >= self.available_commands.len() {
            return;
        }

        let (cmd_name, _cmd) = &self.available_commands[cmd_idx];

        self.argument_input = Some(ArgumentInputState {
            buffer: String::new(),
            cursor_pos: 0,
            command_name: cmd_name.clone(),
        });
        self.argument_input_mode = ArgumentInputMode::Active;
    }

    /// Load state queries for the selected repository.
    pub(super) fn load_state_queries(&mut self, repo_path: &str) {
        use crate::state::{compute_workspace_hash, discover_state_queries, read_latest_cached};
        use std::path::Path;

        // Clear previous state
        self.state_queries.clear();
        self.state_results.clear();
        self.state_panel_list_state = ListState::default();

        let graft_yaml_path = Path::new(repo_path).join("graft.yaml");
        if !graft_yaml_path.exists() {
            return;
        }

        match discover_state_queries(&graft_yaml_path) {
            Ok(queries) => {
                self.state_queries = queries;

                let workspace_hash = compute_workspace_hash(&self.workspace_name);
                let repo_name = Path::new(repo_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");

                for query in &self.state_queries {
                    match read_latest_cached(&workspace_hash, repo_name, &query.name) {
                        Ok(result) => self.state_results.push(Some(result)),
                        Err(e) => {
                            log::debug!("No cache for query {}: {e}", query.name);
                            self.state_results.push(None);
                        }
                    }
                }

                if !self.state_queries.is_empty() && self.state_results.iter().all(Option::is_none)
                {
                    self.status_message = Some(StatusMessage::info(
                        "No cached data. Press 'r' to refresh selected query.".to_string(),
                    ));
                }

                if !self.state_queries.is_empty() {
                    self.state_panel_list_state.select(Some(0));
                }
            }
            Err(e) => {
                log::warn!("Failed to discover state queries: {e}");
                self.status_message = Some(StatusMessage::error(format!(
                    "Failed to load state queries: {e}"
                )));
            }
        }
    }

    /// Refresh the currently selected state query.
    pub(super) fn refresh_selected_state_query(&mut self) {
        use std::process::Command;

        let Some(selected) = self.state_panel_list_state.selected() else {
            self.status_message = Some(StatusMessage::warning("No query selected".to_string()));
            return;
        };

        let (query_name, run_command) = match self.state_queries.get(selected) {
            Some(q) => (q.name.clone(), q.run.clone()),
            None => return,
        };

        let repos = self.registry.list_repos();
        let Some(repo_idx) = self.list_state.selected() else {
            self.status_message =
                Some(StatusMessage::warning("No repository selected".to_string()));
            return;
        };

        let repo_path = match repos.get(repo_idx) {
            Some(r) => r.as_path(),
            None => return,
        };

        self.status_message = Some(StatusMessage::info(format!("Refreshing {query_name}...")));

        let args = match shell_words::split(&run_command) {
            Ok(args) => args,
            Err(e) => {
                self.status_message = Some(StatusMessage::error(format!(
                    "Failed to parse command '{run_command}': {e}"
                )));
                return;
            }
        };

        if args.is_empty() {
            self.status_message = Some(StatusMessage::error(format!(
                "Empty command for query '{query_name}'"
            )));
            return;
        }

        let result = Command::new(&args[0])
            .args(&args[1..])
            .current_dir(repo_path)
            .output();

        let success = match result {
            Ok(output) => {
                if output.status.success() {
                    true
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    self.status_message = Some(StatusMessage::error(format!(
                        "Command failed: {}",
                        stderr.trim()
                    )));
                    false
                }
            }
            Err(e) => {
                self.status_message = Some(StatusMessage::error(format!(
                    "Failed to execute '{run_command}': {e}"
                )));
                false
            }
        };

        if success {
            self.reload_state_query_cache(selected, repo_path);
            self.status_message = Some(StatusMessage::success(format!("Refreshed {query_name}")));
        }
    }

    /// Reload cache for a specific query index.
    fn reload_state_query_cache(&mut self, query_index: usize, repo_path: &std::path::Path) {
        use crate::state::{compute_workspace_hash, read_latest_cached};

        if query_index >= self.state_queries.len() {
            return;
        }

        let query = &self.state_queries[query_index];

        let workspace_hash = compute_workspace_hash(&self.workspace_name);
        let repo_name = repo_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        match read_latest_cached(&workspace_hash, repo_name, &query.name) {
            Ok(result) => {
                if query_index < self.state_results.len() {
                    self.state_results[query_index] = Some(result);
                }
            }
            Err(e) => {
                log::warn!("Failed to reload cache for {}: {e}", query.name);
                if query_index < self.state_results.len() {
                    self.state_results[query_index] = None;
                }
            }
        }
    }
}
