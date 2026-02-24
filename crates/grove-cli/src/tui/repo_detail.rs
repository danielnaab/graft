//! `RepoDetail` view: unified cursor-driven view with all sections.
//!
//! A single cursor moves through all selectable items (file changes, commits,
//! state queries, commands) across all sections. Section headers and blank
//! lines are skipped by the cursor.

use super::{
    format_file_change_indicator, App, ArgumentInputState, Block, Borders, Color, CommandState,
    DetailItem, GraftYamlLoader, KeyCode, Line, Modifier, Paragraph, Rect, RepoDetailProvider,
    RepoRegistry, Span, StatusMessage, Style, View,
};

/// Pairs rendered lines with their corresponding `detail_items` index.
///
/// `item_indices[i]` is `Some(idx)` when line `i` is part of selectable item
/// `detail_items[idx]`, or `None` for headers / blank separators.
struct LineMapping {
    lines: Vec<Line<'static>>,
    item_indices: Vec<Option<usize>>,
}

impl LineMapping {
    fn new() -> Self {
        Self {
            lines: Vec::new(),
            item_indices: Vec::new(),
        }
    }

    fn push(&mut self, line: Line<'static>, item_idx: Option<usize>) {
        self.lines.push(line);
        self.item_indices.push(item_idx);
    }
}

impl<R: RepoRegistry, D: RepoDetailProvider> App<R, D> {
    // ===== Cursor infrastructure =====

    /// Rebuild the flat item list from current data.
    ///
    /// Visual order: file changes, commits, state queries, recent runs, run-state entries, commands.
    /// Clamps cursor to valid range.
    pub(super) fn rebuild_detail_items(&mut self) {
        self.detail_items.clear();

        if let Some(detail) = &self.cached_detail {
            for i in 0..detail.changed_files.len() {
                self.detail_items.push(DetailItem::FileChange(i));
            }
            for i in 0..detail.commits.len() {
                self.detail_items.push(DetailItem::Commit(i));
            }
        }

        for i in 0..self.state_queries.len() {
            self.detail_items.push(DetailItem::StateQuery(i));
        }

        for i in 0..self.recent_runs.len() {
            self.detail_items.push(DetailItem::Run(i));
        }

        for i in 0..self.run_state_entries.len() {
            self.detail_items.push(DetailItem::RunState(i));
        }

        for i in 0..self.available_commands.len() {
            self.detail_items.push(DetailItem::Command(i));
        }

        // Clamp cursor
        if self.detail_items.is_empty() {
            self.detail_cursor = 0;
        } else if self.detail_cursor >= self.detail_items.len() {
            self.detail_cursor = self.detail_items.len() - 1;
        }
    }

    /// Returns the item currently under the cursor, if any.
    pub(super) fn current_detail_item(&self) -> Option<&DetailItem> {
        self.detail_items.get(self.detail_cursor)
    }

    // ===== Key handling =====

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
            // Move cursor down
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.detail_items.is_empty() && self.detail_cursor + 1 < self.detail_items.len()
                {
                    self.detail_cursor += 1;
                }
            }
            // Move cursor up
            KeyCode::Char('k') | KeyCode::Up => {
                self.detail_cursor = self.detail_cursor.saturating_sub(1);
            }
            // Refresh state queries (r key)
            KeyCode::Char('r') => {
                self.refresh_state_queries();
            }
            // Execute selected command, open run log, or toggle state query expand
            KeyCode::Enter => match self.current_detail_item() {
                Some(DetailItem::StateQuery(idx)) => {
                    let idx = *idx;
                    if !self.expanded_state_queries.remove(&idx) {
                        self.expanded_state_queries.insert(idx);
                    }
                }
                Some(DetailItem::Run(idx)) => {
                    let idx = *idx;
                    self.open_run_log(idx);
                }
                Some(DetailItem::RunState(idx)) => {
                    let idx = *idx;
                    if !self.expanded_run_state.remove(&idx) {
                        self.expanded_run_state.insert(idx);
                    }
                }
                _ => {
                    self.execute_selected_command();
                }
            },
            _ => {}
        }
    }

    // ===== Rendering =====

    /// Render the full-width `RepoDetail` view with cursor highlight and auto-scroll.
    #[allow(clippy::cast_possible_truncation)]
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

        let mut mapping = self.build_line_mapping();

        // Apply cursor highlight by replacing lines in-place
        let highlight_style = Style::default().bg(Color::DarkGray);
        for i in 0..mapping.lines.len() {
            if mapping.item_indices[i] == Some(self.detail_cursor) && !self.detail_items.is_empty()
            {
                let line = std::mem::take(&mut mapping.lines[i]);
                mapping.lines[i] = line.patch_style(highlight_style);
            }
        }

        // Auto-scroll to keep cursor visible
        let inner_height = inner.height as usize;
        if !self.detail_items.is_empty() {
            let cursor_first = mapping
                .item_indices
                .iter()
                .position(|idx| *idx == Some(self.detail_cursor));
            let cursor_last = mapping
                .item_indices
                .iter()
                .rposition(|idx| *idx == Some(self.detail_cursor));

            if let (Some(first), Some(last)) = (cursor_first, cursor_last) {
                // Scroll down if cursor is below viewport
                if last >= self.detail_scroll + inner_height {
                    self.detail_scroll = last.saturating_sub(inner_height - 1);
                }
                // Scroll up if cursor is above viewport
                if first < self.detail_scroll {
                    self.detail_scroll = first;
                }
            }
        }

        // Clamp scroll to content height
        let max_scroll = mapping.lines.len().saturating_sub(inner_height);
        self.detail_scroll = self.detail_scroll.min(max_scroll);

        let paragraph = Paragraph::new(mapping.lines)
            .scroll((u16::try_from(self.detail_scroll).unwrap_or(u16::MAX), 0));

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
                        Span::styled(format!(" ── {branch}"), Style::default().fg(Color::Gray)),
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

    /// Build all lines for the unified detail view (used by tests that only need lines).
    #[cfg(test)]
    pub(super) fn build_repo_detail_lines(&self) -> Vec<Line<'static>> {
        self.build_line_mapping().lines
    }

    /// Build the line mapping that pairs each rendered line with its item index.
    fn build_line_mapping(&self) -> LineMapping {
        let mut m = LineMapping::new();
        let mut item_counter: usize = 0;

        // Section 1: Changed Files + Recent Commits
        self.append_changes_section_mapped(&mut m, &mut item_counter);

        m.push(Line::from(""), None);

        // Section 2: State Queries
        self.append_state_section_mapped(&mut m, &mut item_counter);

        m.push(Line::from(""), None);

        // Section 3: Recent Runs
        self.append_runs_section_mapped(&mut m, &mut item_counter);

        m.push(Line::from(""), None);

        // Section 4: Run State entries
        self.append_run_state_section_mapped(&mut m, &mut item_counter);

        m.push(Line::from(""), None);

        // Section 5: Commands
        self.append_commands_section_mapped(&mut m, &mut item_counter);

        m
    }

    /// Append changed files and recent commits with item mapping.
    fn append_changes_section_mapped(&self, m: &mut LineMapping, item_idx: &mut usize) {
        let Some(detail) = &self.cached_detail else {
            m.push(
                Line::from(Span::styled(
                    "No repository selected",
                    Style::default().fg(Color::Gray),
                )),
                None,
            );
            return;
        };

        // Show error as warning if present (but continue rendering partial data)
        if let Some(error) = &detail.error {
            m.push(
                Line::from(Span::styled(
                    format!("Error: {error}"),
                    Style::default().fg(Color::Red),
                )),
                None,
            );
            m.push(Line::from(""), None);
        }

        // Changed files
        if detail.changed_files.is_empty() {
            m.push(
                Line::from(Span::styled(
                    "No uncommitted changes",
                    Style::default().fg(Color::Gray),
                )),
                None,
            );
        } else {
            m.push(
                Line::from(Span::styled(
                    format!("Changed Files ({})", detail.changed_files.len()),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )),
                None,
            );

            for change in &detail.changed_files {
                let (indicator, color) = format_file_change_indicator(&change.status);
                m.push(
                    Line::from(vec![
                        Span::styled(format!("  {indicator} "), Style::default().fg(color)),
                        Span::styled(change.path.clone(), Style::default().fg(Color::White)),
                    ]),
                    Some(*item_idx),
                );
                *item_idx += 1;
            }
        }

        m.push(Line::from(""), None);

        // Recent commits
        if detail.commits.is_empty() {
            m.push(
                Line::from(Span::styled("No commits", Style::default().fg(Color::Gray))),
                None,
            );
        } else {
            m.push(
                Line::from(Span::styled(
                    format!("Recent Commits ({})", detail.commits.len()),
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )),
                None,
            );

            for commit in &detail.commits {
                // First line: hash + subject (selectable)
                m.push(
                    Line::from(vec![
                        Span::styled(
                            format!("  {} ", commit.hash),
                            Style::default().fg(Color::Yellow),
                        ),
                        Span::styled(commit.subject.clone(), Style::default().fg(Color::White)),
                    ]),
                    Some(*item_idx),
                );
                // Second line: author + date (same item)
                m.push(
                    Line::from(Span::styled(
                        format!("       {} - {}", commit.author, commit.relative_date),
                        Style::default().fg(Color::Gray),
                    )),
                    Some(*item_idx),
                );
                *item_idx += 1;
            }
        }
    }

    /// Append state queries section with item mapping.
    fn append_state_section_mapped(&self, m: &mut LineMapping, item_idx: &mut usize) {
        m.push(
            Line::from(Span::styled(
                "State Queries",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            None,
        );

        if self.state_queries.is_empty() {
            m.push(
                Line::from(Span::styled(
                    "  No state queries defined in graft.yaml",
                    Style::default().fg(Color::Gray),
                )),
                None,
            );
        } else {
            for (idx, query) in self.state_queries.iter().enumerate() {
                let is_expanded = self.expanded_state_queries.contains(&idx);
                let chevron = if is_expanded { "▾" } else { "▸" };

                if let Some(Some(result)) = self.state_results.get(idx) {
                    let age = result.metadata.time_ago();
                    let data_summary = crate::state::format_state_summary(result);
                    m.push(
                        Line::from(vec![
                            Span::styled(
                                format!("  {chevron} {:<12}", query.name),
                                Style::default().fg(Color::Cyan),
                            ),
                            Span::raw("  "),
                            Span::raw(format!("{data_summary:<45}")),
                            Span::styled(format!("({age})"), Style::default().fg(Color::Gray)),
                        ]),
                        Some(*item_idx),
                    );

                    // Render expanded data lines (not part of cursor highlight)
                    if is_expanded {
                        for line in format_state_expanded_lines(&result.data) {
                            m.push(line, None);
                        }
                    }
                } else {
                    m.push(
                        Line::from(vec![
                            Span::styled(
                                format!("  {chevron} {:<12}", query.name),
                                Style::default().fg(Color::Gray),
                            ),
                            Span::raw("  "),
                            Span::styled("(no cached data)", Style::default().fg(Color::Gray)),
                        ]),
                        Some(*item_idx),
                    );
                }
                *item_idx += 1;
            }
        }
    }

    /// Append recent runs section with item mapping.
    fn append_runs_section_mapped(&self, m: &mut LineMapping, item_idx: &mut usize) {
        m.push(
            Line::from(Span::styled(
                "Recent Runs",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            None,
        );

        if self.recent_runs.is_empty() {
            m.push(
                Line::from(Span::styled(
                    "  No command runs recorded yet",
                    Style::default().fg(Color::Gray),
                )),
                None,
            );
        } else {
            for (i, run) in self.recent_runs.iter().enumerate() {
                let status_color = match run.exit_code {
                    Some(0) => Color::Green,
                    Some(_) => Color::Red,
                    None => Color::Yellow,
                };
                let is_selected =
                    self.detail_items.get(self.detail_cursor) == Some(&DetailItem::Run(i));
                let prefix = if is_selected { "▶ " } else { "  " };
                let style = if is_selected {
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                m.push(
                    Line::from(vec![
                        Span::styled(prefix, Style::default().fg(Color::Cyan)),
                        Span::styled(format!("{:<20}", run.command), style),
                        Span::styled(
                            format!("{:<10}", run.status_display()),
                            Style::default().fg(status_color),
                        ),
                        Span::styled(run.time_ago(), Style::default().fg(Color::Gray)),
                    ]),
                    Some(*item_idx),
                );
                *item_idx += 1;
            }
        }
    }

    /// Append run-state entries section with item mapping.
    ///
    /// Shows a "Run State" header (blue, bold) followed by one row per entry:
    /// `"  ▸ {name:<12}  {summary:<40} (← {producer})"`.
    /// Empty state shows a gray "No run state" placeholder.
    fn append_run_state_section_mapped(&self, m: &mut LineMapping, item_idx: &mut usize) {
        m.push(
            Line::from(Span::styled(
                "Run State",
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            )),
            None,
        );

        if self.run_state_entries.is_empty() {
            m.push(
                Line::from(Span::styled(
                    "  No run state",
                    Style::default().fg(Color::Gray),
                )),
                None,
            );
            return;
        }

        for (idx, (name, value)) in self.run_state_entries.iter().enumerate() {
            let is_expanded = self.expanded_run_state.contains(&idx);
            let chevron = if is_expanded { "▾" } else { "▸" };
            let summary = truncate_str(&format_value_compact(value), 40);
            let producer = self.run_state_producers.get(name.as_str());

            let mut spans = vec![
                Span::styled(
                    format!("  {chevron} {name:<12}  "),
                    Style::default().fg(Color::White),
                ),
                Span::styled(format!("{summary:<40}"), Style::default().fg(Color::White)),
            ];
            if let Some(prod) = producer {
                spans.push(Span::styled(
                    format!(" (← {prod})"),
                    Style::default().fg(Color::Gray),
                ));
            }
            m.push(Line::from(spans), Some(*item_idx));

            // Render expanded JSON lines (non-selectable)
            if is_expanded {
                for line in format_state_expanded_lines(value) {
                    m.push(line, None);
                }
                // Show consumers if any
                if let Some(consumers) = self.run_state_consumers.get(name.as_str()) {
                    if !consumers.is_empty() {
                        m.push(
                            Line::from(Span::styled(
                                format!("  reads: {}", consumers.join(", ")),
                                Style::default().fg(Color::Gray),
                            )),
                            None,
                        );
                    }
                }
            }

            *item_idx += 1;
        }
    }

    /// Append commands section with item mapping.
    fn append_commands_section_mapped(&self, m: &mut LineMapping, item_idx: &mut usize) {
        m.push(
            Line::from(Span::styled(
                "Commands",
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            )),
            None,
        );

        if self.available_commands.is_empty() {
            m.push(
                Line::from(Span::styled(
                    "  No commands defined in graft.yaml",
                    Style::default().fg(Color::Gray),
                )),
                None,
            );
        } else {
            for (i, (name, cmd)) in self.available_commands.iter().enumerate() {
                let desc = cmd.description.as_deref().unwrap_or("");
                let is_selected =
                    self.detail_items.get(self.detail_cursor) == Some(&DetailItem::Command(i));
                if is_selected {
                    m.push(
                        Line::from(vec![
                            Span::styled("▶ ", Style::default().fg(Color::Cyan)),
                            Span::styled(
                                format!("{name:<20} {desc}"),
                                Style::default()
                                    .fg(Color::White)
                                    .add_modifier(Modifier::BOLD),
                            ),
                        ]),
                        Some(*item_idx),
                    );
                } else {
                    m.push(
                        Line::from(vec![
                            Span::raw("  "),
                            Span::styled(
                                format!("{name:<20} {desc}"),
                                Style::default().fg(Color::White),
                            ),
                        ]),
                        Some(*item_idx),
                    );
                }
                *item_idx += 1;
            }
        }
    }

    /// Ensure state queries and recent runs are loaded for the current repo (lazy, once per repo).
    fn ensure_state_loaded_if_needed(&mut self) {
        if !self.state_loaded {
            if let Some(selected) = self.list_state.selected() {
                let repos = self.registry.list_repos();
                if let Some(repo) = repos.get(selected) {
                    let repo_path_str = repo.as_path().to_str().unwrap_or("").to_string();
                    self.load_state_queries(&repo_path_str);
                    self.load_recent_runs(&repo_path_str);
                    self.load_run_state_entries(&repo_path_str);
                    self.state_loaded = true;
                    self.rebuild_detail_items();
                }
            }
        }
    }

    /// Load recent runs for the selected repository.
    fn load_recent_runs(&mut self, repo_path: &str) {
        let repo_name = graft_common::repo_name_from_path(repo_path);
        self.recent_runs = graft_common::list_runs(&self.workspace_name, repo_name, 50);
    }

    /// Load run-state entries from `.graft/run-state/` in the selected repository.
    ///
    /// Enumerates all `*.json` files in the directory, parses them, and stores
    /// `(name, value)` pairs sorted alphabetically by name. Missing directory is
    /// handled gracefully (results in an empty list).
    ///
    /// Producer/consumer maps are built separately in
    /// `load_commands_for_selected_repo()` from `available_commands`.
    fn load_run_state_entries(&mut self, repo_path: &str) {
        use std::path::Path;

        self.run_state_entries.clear();

        let run_state_dir = Path::new(repo_path).join(".graft").join("run-state");
        let Ok(read_dir) = std::fs::read_dir(&run_state_dir) else {
            return; // Directory absent or unreadable — treat as empty
        };

        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let Some(name) = path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(str::to_string)
            else {
                continue;
            };
            let Ok(content) = std::fs::read_to_string(&path) else {
                continue;
            };
            let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) else {
                continue;
            };
            self.run_state_entries.push((name, value));
        }

        self.run_state_entries.sort_by(|a, b| a.0.cmp(&b.0));
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
                self.rebuild_detail_items();
                return;
            }
        };

        self.available_commands = graft_config.commands.into_iter().collect();

        // Load commands from dependencies (qualified as dep:cmd)
        for dep_name in &graft_config.dependency_names {
            let dep_graft_path = format!("{repo_path}/.graft/{dep_name}/graft.yaml");
            if let Ok(dep_config) = self.graft_loader.load_graft(&dep_graft_path) {
                for (cmd_name, cmd) in dep_config.commands {
                    let qualified = format!("{dep_name}:{cmd_name}");
                    self.available_commands.push((qualified, cmd));
                }
            }
        }

        self.available_commands.sort_by(|a, b| a.0.cmp(&b.0));

        // Rebuild producer/consumer maps from the now-complete available_commands.
        // This covers both root and dep commands since available_commands includes both.
        self.run_state_producers.clear();
        self.run_state_consumers.clear();
        for (cmd_name, cmd) in &self.available_commands {
            for state_name in &cmd.writes {
                self.run_state_producers
                    .insert(state_name.clone(), cmd_name.clone());
            }
            for state_name in &cmd.reads {
                self.run_state_consumers
                    .entry(state_name.clone())
                    .or_default()
                    .push(cmd_name.clone());
            }
        }

        self.selected_repo_for_commands = Some(repo_path);
        self.rebuild_detail_items();
    }

    /// Open a past run's log file in the `CommandOutput` view (read-only).
    fn open_run_log(&mut self, run_idx: usize) {
        let Some(run) = self.recent_runs.get(run_idx) else {
            return;
        };

        let repo_path = match &self.selected_repo_for_commands {
            Some(p) => p.clone(),
            None => return,
        };

        let repo_name = graft_common::repo_name_from_path(&repo_path);

        let log_content =
            graft_common::read_run_log(&self.workspace_name, repo_name, &run.log_file);

        // Load log content into output view
        self.output_lines.clear();
        self.output_scroll = 0;
        self.output_truncated_start = false;

        if let Some(content) = log_content {
            for line in content.lines() {
                self.output_lines.push(line.to_string());
            }
        } else {
            self.output_lines.push("(no log file found)".to_string());
        }

        // Show run info in header.
        // For runs with no exit code (interrupted), use exit_code -1 rather than
        // Running, which would trigger the stop-confirmation dialog on quit.
        self.command_name = Some(format!("Run: {} ({})", run.command, run.time_ago()));
        self.command_state = CommandState::Completed {
            exit_code: run.exit_code.unwrap_or(-1),
        };
        self.command_event_rx = None;
        self.running_command_pid = None;

        self.push_view(View::CommandOutput);
    }

    /// Execute the currently selected command.
    ///
    /// If the command has an `args` schema, show the form overlay.
    /// Otherwise, fall back to the free-text argument input.
    /// Silent no-op when cursor is not on a Command item.
    pub(super) fn execute_selected_command(&mut self) {
        let cmd_idx = match self.current_detail_item() {
            Some(DetailItem::Command(idx)) => *idx,
            _ => return,
        };

        if cmd_idx >= self.available_commands.len() {
            return;
        }

        let (cmd_name, cmd) = &self.available_commands[cmd_idx];

        if let Some(args) = &cmd.args {
            if !args.is_empty() {
                let mut form = super::FormInputState::from_schema(cmd_name.clone(), args.clone());
                self.inject_dynamic_options(&mut form);
                self.form_input = Some(form);
                return;
            }
        }

        // No schema — existing free-text input
        self.argument_input = Some(ArgumentInputState {
            text: super::text_buffer::TextBuffer::new(),
            command_name: cmd_name.clone(),
        });
    }

    /// Inject dynamic options from state query results into form fields
    /// that have `options_from` set.
    fn inject_dynamic_options(&self, form: &mut super::FormInputState) {
        for field in &mut form.fields {
            let Some(query_name) = &field.def.options_from else {
                continue;
            };

            // Find the state query index by name
            let query_idx = self
                .state_queries
                .iter()
                .position(|q| q.name == *query_name);

            let Some(idx) = query_idx else {
                continue;
            };

            // Get the cached result
            let Some(Some(result)) = self.state_results.get(idx) else {
                continue;
            };

            let options = extract_options_from_state(&result.data, query_name);
            if !options.is_empty() {
                field.def.options = Some(options);
                // Reset choice index to 0
                if matches!(field.value, super::FieldValue::Choice(_)) {
                    field.value = super::FieldValue::Choice(0);
                }
            }
        }
    }

    /// Load state queries for the selected repository (root + dependencies).
    pub(super) fn load_state_queries(&mut self, repo_path: &str) {
        use crate::state::{discover_state_queries, read_latest_cached};
        use std::path::Path;

        // Clear previous state
        self.state_queries.clear();
        self.state_results.clear();
        self.expanded_state_queries.clear();

        let graft_yaml_path = Path::new(repo_path).join("graft.yaml");
        if !graft_yaml_path.exists() {
            return;
        }

        // Load root state queries
        match discover_state_queries(&graft_yaml_path) {
            Ok(queries) => {
                self.state_queries = queries;
            }
            Err(e) => {
                log::warn!("Failed to discover state queries: {e}");
                self.status_message = Some(StatusMessage::error(format!(
                    "Failed to load state queries: {e}"
                )));
                return;
            }
        }

        // Load dependency state queries (with script paths pre-resolved to absolute)
        match graft_common::parse_dependency_names(&graft_yaml_path) {
            Ok(dep_names) => {
                for dep_name in &dep_names {
                    let dep_graft_path =
                        Path::new(repo_path).join(format!(".graft/{dep_name}/graft.yaml"));
                    match discover_state_queries(&dep_graft_path) {
                        Ok(mut dep_queries) => {
                            let dep_dir = Path::new(repo_path).join(format!(".graft/{dep_name}"));
                            for query in &mut dep_queries {
                                query.run =
                                    graft_engine::resolve_script_in_command(&query.run, &dep_dir);
                            }
                            self.state_queries.extend(dep_queries);
                        }
                        Err(e) => {
                            log::warn!("Failed to load state queries from dep '{dep_name}': {e}");
                        }
                    }
                }
            }
            Err(e) => {
                log::warn!("Failed to parse dependency names: {e}");
            }
        }

        // Load cached results for all queries
        let repo_name = graft_common::repo_name_from_path(repo_path);
        for query in &self.state_queries {
            if let Some(result) = read_latest_cached(&self.workspace_name, repo_name, &query.name) {
                self.state_results.push(Some(result));
            } else {
                log::debug!("No cache for query {}", query.name);
                self.state_results.push(None);
            }
        }

        if !self.state_queries.is_empty() && self.state_results.iter().all(Option::is_none) {
            self.status_message = Some(StatusMessage::info(
                "No cached data. Press 'r' to refresh state queries.".to_string(),
            ));
        }
    }

    /// Refresh all state queries for the currently selected repository.
    ///
    /// Executes each query via `graft_engine::execute_state_query`, which uses
    /// `ProcessHandle` with timeout protection. Writes results to cache and
    /// updates in-memory results. Reports overall success/failure in the status bar.
    pub(super) fn refresh_state_queries(&mut self) {
        if self.state_queries.is_empty() {
            self.status_message = Some(StatusMessage::info(
                "No state queries defined in graft.yaml".to_string(),
            ));
            return;
        }

        let repos = self.registry.list_repos();
        let Some(repo_idx) = self.list_state.selected() else {
            self.status_message =
                Some(StatusMessage::warning("No repository selected".to_string()));
            return;
        };

        let Some(repo_path) = repos.get(repo_idx).map(grove_core::RepoPath::as_path) else {
            return;
        };
        let repo_path = repo_path.to_path_buf();

        let repo_name =
            graft_common::repo_name_from_path(repo_path.to_str().unwrap_or("unknown")).to_string();

        // Resolve HEAD once; fall back to "unknown" so queries still run on bare repos.
        let commit_hash =
            graft_common::get_current_commit(&repo_path).unwrap_or_else(|_| "unknown".to_string());

        let total = self.state_queries.len();
        let mut failed = 0usize;

        for (i, query) in self.state_queries.iter().enumerate() {
            let graft_query = graft_engine::StateQuery {
                name: query.name.clone(),
                run: query.run.clone(),
                cache: graft_engine::StateCache {
                    deterministic: query.deterministic,
                    ttl: None,
                },
                timeout: query.timeout,
            };

            match graft_engine::state::execute_state_query(&graft_query, &repo_path, &commit_hash) {
                Ok(result) => {
                    if let Err(e) = graft_common::state::write_cached_state(
                        &self.workspace_name,
                        &repo_name,
                        &result,
                    ) {
                        log::warn!("Failed to write cache for '{}': {e}", query.name);
                    }
                    if i < self.state_results.len() {
                        self.state_results[i] = Some(result);
                    }
                }
                Err(e) => {
                    log::warn!("Query '{}' failed: {e}", query.name);
                    failed += 1;
                }
            }
        }

        self.rebuild_detail_items();

        self.status_message = if failed == 0 {
            Some(StatusMessage::success(format!(
                "Refreshed {total} state quer{}",
                if total == 1 { "y" } else { "ies" }
            )))
        } else {
            Some(StatusMessage::warning(format!(
                "Refreshed {}/{total} state quer{} ({failed} failed)",
                total - failed,
                if total == 1 { "y" } else { "ies" }
            )))
        };
    }
}

/// Extract option strings from a state query JSON result.
///
/// Supports two shapes:
/// 1. Top-level array: `[{slug: "a"}, ...]` or `["a", "b"]`
/// 2. Object with array field matching query name: `{slices: [{slug: "a"}, ...]}`
///
/// For objects in the array, tries `slug`, then `name`, then `path` fields.
/// Extract a display string from a single JSON array element.
///
/// For strings, returns the value directly. For objects, tries `slug`, then
/// `name`, then `path`.
fn extract_option(item: &serde_json::Value) -> Option<String> {
    if let Some(s) = item.as_str() {
        return Some(s.to_string());
    }
    if let Some(obj) = item.as_object() {
        for key in &["slug", "name", "path"] {
            if let Some(val) = obj.get(*key).and_then(|v| v.as_str()) {
                return Some(val.to_string());
            }
        }
    }
    None
}

/// Format a JSON value as colored `Line`s for the expanded state query view.
fn format_state_expanded_lines(data: &serde_json::Value) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    match data {
        serde_json::Value::Object(obj) => {
            for (key, value) in obj {
                if let serde_json::Value::Array(arr) = value {
                    lines.push(Line::from(vec![
                        Span::styled(format!("      {key}"), Style::default().fg(Color::Cyan)),
                        Span::styled(": ", Style::default().fg(Color::Gray)),
                        Span::styled(
                            format!("[{} items]", arr.len()),
                            Style::default().fg(Color::Gray),
                        ),
                    ]));
                    for (i, item) in arr.iter().enumerate().take(20) {
                        lines.push(Line::from(vec![
                            Span::styled(
                                format!("        [{i}] "),
                                Style::default().fg(Color::Gray),
                            ),
                            Span::styled(
                                format_value_compact(item),
                                Style::default().fg(Color::White),
                            ),
                        ]));
                    }
                    if arr.len() > 20 {
                        lines.push(Line::from(Span::styled(
                            format!("        ... and {} more", arr.len() - 20),
                            Style::default().fg(Color::Gray),
                        )));
                    }
                } else {
                    lines.push(Line::from(vec![
                        Span::styled(format!("      {key}"), Style::default().fg(Color::Cyan)),
                        Span::styled(": ", Style::default().fg(Color::Gray)),
                        Span::styled(
                            format_value_compact(value),
                            Style::default().fg(Color::White),
                        ),
                    ]));
                }
            }
        }
        serde_json::Value::Array(arr) => {
            for (i, item) in arr.iter().enumerate().take(30) {
                lines.push(Line::from(vec![
                    Span::styled(format!("      [{i}] "), Style::default().fg(Color::Gray)),
                    Span::styled(
                        format_value_compact(item),
                        Style::default().fg(Color::White),
                    ),
                ]));
            }
            if arr.len() > 30 {
                lines.push(Line::from(Span::styled(
                    format!("      ... and {} more", arr.len() - 30),
                    Style::default().fg(Color::Gray),
                )));
            }
        }
        other => {
            lines.push(Line::from(Span::styled(
                format!("      {}", format_value_compact(other)),
                Style::default().fg(Color::White),
            )));
        }
    }

    lines
}

/// Maximum length for a compact value before truncation.
const MAX_COMPACT_LEN: usize = 80;

/// Format a JSON value compactly for a single line.
///
/// Strings are quoted for clarity. Long values are truncated.
fn format_value_compact(value: &serde_json::Value) -> String {
    let raw = match value {
        serde_json::Value::String(s) => format!("\"{s}\""),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Object(obj) => {
            let pairs: Vec<String> = obj
                .iter()
                .map(|(k, v)| format!("{k}: {}", format_value_compact(v)))
                .collect();
            format!("{{{}}}", pairs.join(", "))
        }
        serde_json::Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(format_value_compact).collect();
            format!("[{}]", items.join(", "))
        }
    };
    truncate_str(&raw, MAX_COMPACT_LEN)
}

/// Truncate a string to `max_len` chars, appending `...` if truncated.
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let mut truncated = s[..max_len.saturating_sub(3)].to_string();
        truncated.push_str("...");
        truncated
    }
}

fn extract_options_from_state(data: &serde_json::Value, query_name: &str) -> Vec<String> {
    let arr: &[serde_json::Value] = if let Some(arr) = data.as_array() {
        arr
    } else if let Some(obj) = data.as_object() {
        match obj.get(query_name).and_then(|v| v.as_array()) {
            Some(a) => a,
            None => return Vec::new(),
        }
    } else {
        return Vec::new();
    };

    arr.iter()
        .filter(|item| {
            // Skip items explicitly marked as done (e.g. completed slices).
            item.as_object()
                .and_then(|obj| obj.get("status"))
                .and_then(|v| v.as_str())
                != Some("done")
        })
        .filter_map(extract_option)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        extract_options_from_state, format_state_expanded_lines, format_value_compact, truncate_str,
    };
    use serde_json::json;

    #[test]
    fn extracts_slugs_from_slices_query_shape() {
        // Real list-slices.sh output shape — done slice is filtered out.
        let data = json!({
            "slices": [
                {"path": "slices/foo/plan.md", "status": "draft", "slug": "foo", "steps_total": 3, "steps_done": 1},
                {"path": "slices/bar/plan.md", "status": "done", "slug": "bar", "steps_total": 2, "steps_done": 2}
            ],
            "counts": {"draft": 1, "done": 1}
        });
        let opts = extract_options_from_state(&data, "slices");
        assert_eq!(opts, vec!["foo"]);
    }

    #[test]
    fn filters_out_done_slices() {
        let data = json!({
            "slices": [
                {"slug": "alpha", "status": "draft"},
                {"slug": "beta",  "status": "in_progress"},
                {"slug": "gamma", "status": "done"},
                {"slug": "delta", "status": "accepted"},
            ]
        });
        let opts = extract_options_from_state(&data, "slices");
        assert_eq!(opts, vec!["alpha", "beta", "delta"]);
    }

    #[test]
    fn items_without_status_field_are_kept() {
        // Generic option lists (not slices) should pass through unchanged.
        let data = json!(["alpha", "beta", "gamma"]);
        let opts = extract_options_from_state(&data, "anything");
        assert_eq!(opts, vec!["alpha", "beta", "gamma"]);
    }

    #[test]
    fn extracts_from_plain_string_array() {
        let data = json!(["alpha", "beta", "gamma"]);
        let opts = extract_options_from_state(&data, "anything");
        assert_eq!(opts, vec!["alpha", "beta", "gamma"]);
    }

    #[test]
    fn extracts_name_when_no_slug() {
        let data = json!({
            "items": [
                {"name": "first"},
                {"name": "second"}
            ]
        });
        let opts = extract_options_from_state(&data, "items");
        assert_eq!(opts, vec!["first", "second"]);
    }

    #[test]
    fn extracts_path_as_last_resort() {
        let data = json!({
            "files": [
                {"path": "/tmp/a"},
                {"path": "/tmp/b"}
            ]
        });
        let opts = extract_options_from_state(&data, "files");
        assert_eq!(opts, vec!["/tmp/a", "/tmp/b"]);
    }

    #[test]
    fn prefers_slug_over_name_over_path() {
        let data = json!([
            {"slug": "s", "name": "n", "path": "p"}
        ]);
        let opts = extract_options_from_state(&data, "x");
        assert_eq!(opts, vec!["s"]);
    }

    #[test]
    fn returns_empty_for_missing_key() {
        let data = json!({"other": [{"slug": "a"}]});
        let opts = extract_options_from_state(&data, "slices");
        assert!(opts.is_empty());
    }

    #[test]
    fn returns_empty_for_empty_data() {
        let opts = extract_options_from_state(&json!(null), "slices");
        assert!(opts.is_empty());

        let opts = extract_options_from_state(&json!({}), "slices");
        assert!(opts.is_empty());

        let opts = extract_options_from_state(&json!([]), "slices");
        assert!(opts.is_empty());
    }

    #[test]
    fn skips_objects_with_no_recognized_fields() {
        let data = json!([
            {"slug": "good"},
            {"unrelated": "bad"},
            {"name": "also_good"}
        ]);
        let opts = extract_options_from_state(&data, "x");
        assert_eq!(opts, vec!["good", "also_good"]);
    }

    // ===== format_value_compact tests =====

    #[test]
    fn compact_quotes_strings() {
        let v = json!("hello");
        assert_eq!(format_value_compact(&v), "\"hello\"");
    }

    #[test]
    fn compact_formats_numbers_and_bools() {
        assert_eq!(format_value_compact(&json!(42)), "42");
        assert_eq!(format_value_compact(&json!(true)), "true");
        assert_eq!(format_value_compact(&json!(null)), "null");
    }

    #[test]
    fn compact_formats_flat_object() {
        let v = json!({"a": 1});
        let s = format_value_compact(&v);
        assert!(s.contains("a: 1"), "got: {s}");
    }

    #[test]
    fn compact_truncates_long_values() {
        // Build a value that will exceed MAX_COMPACT_LEN
        let long_str = "x".repeat(100);
        let v = json!(long_str);
        let s = format_value_compact(&v);
        assert!(
            s.len() <= super::MAX_COMPACT_LEN,
            "should be truncated, len={}",
            s.len()
        );
        assert!(s.ends_with("..."), "should end with ellipsis: {s}");
    }

    // ===== format_state_expanded_lines tests =====

    #[test]
    fn expanded_flat_object_has_key_per_line() {
        let data = json!({"format": "OK", "lint": "OK", "tests": "OK"});
        let lines = format_state_expanded_lines(&data);
        assert_eq!(lines.len(), 3, "one line per key");
    }

    #[test]
    fn expanded_array_shows_indexed_items() {
        let data = json!(["a", "b", "c"]);
        let lines = format_state_expanded_lines(&data);
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn expanded_object_with_nested_array() {
        let data = json!({"items": [1, 2, 3], "count": 3});
        let lines = format_state_expanded_lines(&data);
        // "count: 3" + "items: [3 items]" + 3 array items = 5 lines
        assert_eq!(lines.len(), 5);
    }

    #[test]
    fn expanded_null_produces_single_line() {
        let lines = format_state_expanded_lines(&json!(null));
        assert_eq!(lines.len(), 1);
    }

    // ===== truncate_str tests =====

    #[test]
    fn truncate_short_string_unchanged() {
        assert_eq!(truncate_str("abc", 10), "abc");
    }

    #[test]
    fn truncate_long_string_adds_ellipsis() {
        let s = "a".repeat(20);
        let t = truncate_str(&s, 10);
        assert_eq!(t.len(), 10);
        assert!(t.ends_with("..."));
    }
}
