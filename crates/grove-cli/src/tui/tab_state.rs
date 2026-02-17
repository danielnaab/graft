//! State tab: state query display and management.

use super::{
    Alignment, App, Color, KeyCode, Line, List, ListItem, ListState, Modifier, Paragraph, Rect,
    RepoDetailProvider, RepoRegistry, Span, StatusMessage, Style,
};

impl<R: RepoRegistry, D: RepoDetailProvider> App<R, D> {
    /// Handle keys when the State tab is active.
    pub(super) fn handle_key_state_tab(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('j') | KeyCode::Down => {
                let i = self.state_panel_list_state.selected().unwrap_or(0);
                if i + 1 < self.state_queries.len() {
                    self.state_panel_list_state.select(Some(i + 1));
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let i = self.state_panel_list_state.selected().unwrap_or(0);
                if i > 0 {
                    self.state_panel_list_state.select(Some(i - 1));
                }
            }
            KeyCode::Char('r') => {
                self.refresh_selected_state_query();
            }
            _ => {}
        }
    }

    /// Render the State tab content in the given area.
    pub(super) fn render_state_tab(&mut self, frame: &mut ratatui::Frame, area: Rect) {
        // Build list items with query name, summary, and cache age
        let items: Vec<ListItem> = self
            .state_queries
            .iter()
            .enumerate()
            .map(|(idx, query)| {
                let line = if let Some(Some(result)) = self.state_results.get(idx) {
                    let age = result.metadata.time_ago();
                    let data_summary = result.summary();

                    Line::from(vec![
                        Span::styled(
                            format!("{:<14}", query.name),
                            Style::default().fg(Color::Cyan),
                        ),
                        Span::raw("  "),
                        Span::raw(format!("{data_summary:<45}")),
                        Span::styled(format!("({age})"), Style::default().fg(Color::DarkGray)),
                    ])
                } else {
                    Line::from(vec![
                        Span::styled(
                            format!("{:<14}", query.name),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::raw("  "),
                        Span::styled("(no cached data)", Style::default().fg(Color::DarkGray)),
                    ])
                };

                ListItem::new(line)
            })
            .collect();

        if items.is_empty() {
            let empty_text = vec![
                Line::from(""),
                Line::from("  No state queries defined in graft.yaml")
                    .style(Style::default().fg(Color::Yellow)),
                Line::from(""),
                Line::from("  State queries track project metrics over time:")
                    .style(Style::default().fg(Color::DarkGray)),
                Line::from("  • Code coverage, test counts, lint warnings")
                    .style(Style::default().fg(Color::DarkGray)),
                Line::from("  • Task/issue counts, PR status")
                    .style(Style::default().fg(Color::DarkGray)),
                Line::from("  • Documentation health, broken links")
                    .style(Style::default().fg(Color::DarkGray)),
                Line::from(""),
                Line::from("  Example graft.yaml configuration:")
                    .style(Style::default().fg(Color::White)),
                Line::from(""),
                Line::from("    state:").style(Style::default().fg(Color::Cyan)),
                Line::from("      coverage:").style(Style::default().fg(Color::Cyan)),
                Line::from("        run: \"pytest --cov --cov-report=json\"")
                    .style(Style::default().fg(Color::Green)),
                Line::from("        cache:").style(Style::default().fg(Color::Cyan)),
                Line::from("          deterministic: true")
                    .style(Style::default().fg(Color::Green)),
                Line::from("        description: \"Code coverage metrics\"")
                    .style(Style::default().fg(Color::Green)),
            ];

            let paragraph = Paragraph::new(empty_text).alignment(Alignment::Left);
            frame.render_widget(paragraph, area);
        } else {
            let list = List::new(items)
                .highlight_style(
                    Style::default()
                        .bg(Color::Rgb(40, 40, 50))
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("▶ ");

            frame.render_stateful_widget(list, area, &mut self.state_panel_list_state);
        }
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
