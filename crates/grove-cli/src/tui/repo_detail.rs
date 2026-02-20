//! `RepoDetail` view: unified cursor-driven view with all sections.
//!
//! A single cursor moves through all selectable items (file changes, commits,
//! state queries, commands) across all sections. Section headers and blank
//! lines are skipped by the cursor.

use super::{
    format_file_change_indicator, App, ArgumentInputState, Block, Borders, Color, DetailItem,
    GraftYamlLoader, KeyCode, Line, Modifier, Paragraph, Rect, RepoDetailProvider, RepoRegistry,
    Span, StatusMessage, Style, View,
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
    /// Visual order: file changes, commits, state queries, commands.
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
            // Execute selected command (silent no-op on non-command items)
            KeyCode::Enter => {
                self.execute_selected_command();
            }
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

        // Section 3: Commands
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
                if let Some(Some(result)) = self.state_results.get(idx) {
                    let age = result.metadata.time_ago();
                    let data_summary = crate::state::format_state_summary(result);
                    m.push(
                        Line::from(vec![
                            Span::styled(
                                format!("  {:<14}", query.name),
                                Style::default().fg(Color::Cyan),
                            ),
                            Span::raw("  "),
                            Span::raw(format!("{data_summary:<45}")),
                            Span::styled(format!("({age})"), Style::default().fg(Color::Gray)),
                        ]),
                        Some(*item_idx),
                    );
                } else {
                    m.push(
                        Line::from(vec![
                            Span::styled(
                                format!("  {:<14}", query.name),
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

    /// Ensure state queries are loaded for the current repo (lazy, only if empty).
    fn ensure_state_loaded_if_needed(&mut self) {
        if self.state_queries.is_empty() {
            if let Some(selected) = self.list_state.selected() {
                let repos = self.registry.list_repos();
                if let Some(repo) = repos.get(selected) {
                    let repo_path_str = repo.as_path().to_str().unwrap_or("").to_string();
                    self.load_state_queries(&repo_path_str);
                    self.rebuild_detail_items();
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
        self.selected_repo_for_commands = Some(repo_path);
        self.rebuild_detail_items();
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
                self.form_input = Some(super::FormInputState::from_schema(
                    cmd_name.clone(),
                    args.clone(),
                ));
                return;
            }
        }

        // No schema — existing free-text input
        self.argument_input = Some(ArgumentInputState {
            text: super::text_buffer::TextBuffer::new(),
            command_name: cmd_name.clone(),
        });
    }

    /// Load state queries for the selected repository.
    pub(super) fn load_state_queries(&mut self, repo_path: &str) {
        use crate::state::{discover_state_queries, read_latest_cached};
        use std::path::Path;

        // Clear previous state
        self.state_queries.clear();
        self.state_results.clear();

        let graft_yaml_path = Path::new(repo_path).join("graft.yaml");
        if !graft_yaml_path.exists() {
            return;
        }

        match discover_state_queries(&graft_yaml_path) {
            Ok(queries) => {
                self.state_queries = queries;

                let repo_name = Path::new(repo_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");

                for query in &self.state_queries {
                    if let Some(result) =
                        read_latest_cached(&self.workspace_name, repo_name, &query.name)
                    {
                        self.state_results.push(Some(result));
                    } else {
                        log::debug!("No cache for query {}", query.name);
                        self.state_results.push(None);
                    }
                }

                if !self.state_queries.is_empty() && self.state_results.iter().all(Option::is_none)
                {
                    self.status_message = Some(StatusMessage::info(
                        "No cached data. Press 'r' to refresh state queries.".to_string(),
                    ));
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

        let repo_name = repo_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

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
