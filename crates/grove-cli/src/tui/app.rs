//! App struct construction, key dispatch, navigation, and data loading.

use super::{
    ActivePane, App, CommandState, DetailTab, GraftYamlConfigLoader, KeyCode, ListState,
    RepoDetail, RepoDetailProvider, RepoRegistry, StatusMessage, View, DEFAULT_MAX_COMMITS,
};

impl<R: RepoRegistry, D: RepoDetailProvider> App<R, D> {
    pub(super) fn new(registry: R, detail_provider: D, workspace_name: String) -> Self {
        let mut list_state = ListState::default();

        let repos = registry.list_repos();
        if !repos.is_empty() {
            list_state.select(Some(0));
        }

        Self {
            registry,
            detail_provider,
            list_state,
            should_quit: false,
            active_pane: ActivePane::RepoList,
            view_stack: vec![View::Dashboard],
            active_tab: DetailTab::Changes,
            detail_scroll: 0,
            cached_detail: None,
            cached_detail_index: None,
            workspace_name,
            status_message: None,
            needs_refresh: false,

            // Command execution state
            command_picker_state: ListState::default(),
            available_commands: Vec::new(),
            selected_repo_for_commands: None,
            argument_input: None,
            output_lines: Vec::new(),
            output_scroll: 0,
            output_truncated_start: false,
            command_state: CommandState::NotStarted,
            command_name: None,
            graft_loader: GraftYamlConfigLoader::new(),
            command_event_rx: None,
            running_command_pid: None,
            show_stop_confirmation: false,

            // State query panel
            state_queries: Vec::new(),
            state_results: Vec::new(),
            state_panel_list_state: ListState::default(),
        }
    }

    /// Perform status refresh if needed
    pub(super) fn handle_refresh_if_needed(&mut self) {
        if self.needs_refresh {
            match self.registry.refresh_all() {
                Ok(stats) => {
                    self.status_message = if stats.all_successful() {
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
                    self.status_message =
                        Some(StatusMessage::error(format!("Refresh failed: {e}")));
                }
            }

            self.cached_detail = None;
            self.cached_detail_index = None;
            self.needs_refresh = false;
        }
    }

    /// Clear expired status messages (older than 3 seconds)
    pub(super) fn clear_expired_status_message(&mut self) {
        if let Some(msg) = &self.status_message {
            if msg.is_expired() {
                self.status_message = None;
            }
        }
    }

    pub(super) fn handle_key(&mut self, code: KeyCode) {
        match self.active_pane {
            ActivePane::RepoList => self.handle_key_repo_list(code),
            ActivePane::Detail => self.handle_key_detail(code),
            ActivePane::Help => self.handle_key_help(code),
            ActivePane::ArgumentInput => self.handle_key_argument_input(code),
            ActivePane::CommandOutput => self.handle_key_command_output(code),
        }
    }

    fn handle_key_repo_list(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.next();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.previous();
            }
            KeyCode::Enter | KeyCode::Tab => {
                if self.list_state.selected().is_some() {
                    self.active_pane = ActivePane::Detail;
                    self.active_tab = DetailTab::Changes;
                }
            }
            KeyCode::Char('r') => {
                self.needs_refresh = true;
                self.status_message = Some(StatusMessage::info("Refreshing..."));
            }
            KeyCode::Char('?') => {
                self.active_pane = ActivePane::Help;
            }
            KeyCode::Char('x') => {
                if self.list_state.selected().is_some() {
                    self.load_commands_for_selected_repo();
                    self.active_pane = ActivePane::Detail;
                    self.active_tab = DetailTab::Commands;
                }
            }
            KeyCode::Char('s') => {
                if self.list_state.selected().is_some() {
                    self.ensure_state_loaded();
                    self.active_pane = ActivePane::Detail;
                    self.active_tab = DetailTab::State;
                }
            }
            _ => {}
        }
    }

    fn handle_key_detail(&mut self, code: KeyCode) {
        match code {
            // Global detail keys
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Tab => {
                self.active_pane = ActivePane::RepoList;
            }
            KeyCode::Char('?') => {
                self.active_pane = ActivePane::Help;
            }
            // Tab switching by number (with legacy shortcuts merged)
            KeyCode::Char('1') => {
                self.active_tab = DetailTab::Changes;
            }
            KeyCode::Char('2' | 's') => {
                self.ensure_state_loaded();
                self.active_tab = DetailTab::State;
            }
            KeyCode::Char('3' | 'x') => {
                self.load_commands_for_selected_repo();
                self.active_tab = DetailTab::Commands;
            }
            // Delegate to active tab handler
            _ => match self.active_tab {
                DetailTab::Changes => self.handle_key_changes_tab(code),
                DetailTab::State => self.handle_key_state_tab(code),
                DetailTab::Commands => self.handle_key_commands_tab(code),
            },
        }
    }

    fn handle_key_help(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char(_) | KeyCode::Esc | KeyCode::Enter | KeyCode::Backspace => {
                self.active_pane = ActivePane::RepoList;
            }
            _ => {}
        }
    }

    pub(super) fn next(&mut self) {
        let repos = self.registry.list_repos();
        if repos.is_empty() {
            self.list_state.select(None);
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= repos.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));

        // Invalidate tab data for lazy reload
        self.selected_repo_for_commands = None;
        self.available_commands.clear();
        self.state_queries.clear();
        self.state_results.clear();
    }

    pub(super) fn previous(&mut self) {
        let repos = self.registry.list_repos();
        if repos.is_empty() {
            self.list_state.select(None);
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    repos.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));

        // Invalidate tab data for lazy reload
        self.selected_repo_for_commands = None;
        self.available_commands.clear();
        self.state_queries.clear();
        self.state_results.clear();
    }

    /// Load detail for the currently selected repo if not already cached.
    pub(super) fn ensure_detail_loaded(&mut self) {
        let selected = self.list_state.selected();
        if selected == self.cached_detail_index && self.cached_detail.is_some() {
            return;
        }

        let Some(index) = selected else {
            self.cached_detail = None;
            self.cached_detail_index = None;
            return;
        };

        let repos = self.registry.list_repos();
        if index >= repos.len() {
            self.cached_detail = None;
            self.cached_detail_index = None;
            return;
        }

        let detail = match self
            .detail_provider
            .get_detail(&repos[index], DEFAULT_MAX_COMMITS)
        {
            Ok(d) => d,
            Err(e) => RepoDetail::with_error(e.to_string()),
        };

        self.cached_detail = Some(detail);
        self.cached_detail_index = Some(index);
        self.detail_scroll = 0;
    }

    /// Ensure state queries are loaded for the current repo.
    pub(super) fn ensure_state_loaded(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            let repos = self.registry.list_repos();
            if let Some(repo) = repos.get(selected) {
                let repo_path_str = repo.as_path().to_str().unwrap_or("");
                self.load_state_queries(repo_path_str);
            }
        }
    }

    // ===== View stack helpers =====
    // Task 2 will use these; suppress dead-code warnings while additive.
    #[allow(dead_code)]
    /// Returns the current (top-of-stack) view.
    pub(super) fn current_view(&self) -> &View {
        self.view_stack.last().expect("view_stack is never empty")
    }

    #[allow(dead_code)]
    /// Push a view onto the stack and update the `active_pane` bridge.
    pub(super) fn push_view(&mut self, view: View) {
        self.view_stack.push(view);
        self.sync_active_pane();
    }

    #[allow(dead_code)]
    /// Pop the top view from the stack (minimum: Dashboard stays).
    /// Updates the `active_pane` bridge.
    pub(super) fn pop_view(&mut self) {
        if self.view_stack.len() > 1 {
            self.view_stack.pop();
        }
        self.sync_active_pane();
    }

    #[allow(dead_code)]
    /// Reset the stack to just Dashboard.
    pub(super) fn reset_to_dashboard(&mut self) {
        self.view_stack.clear();
        self.view_stack.push(View::Dashboard);
        self.sync_active_pane();
    }

    #[allow(dead_code)]
    /// Reset the stack to a single specified view (replaces everything).
    pub(super) fn reset_to_view(&mut self, view: View) {
        self.view_stack.clear();
        self.view_stack.push(view);
        self.sync_active_pane();
    }

    #[allow(dead_code)]
    /// Bridge: derive the legacy `ActivePane` value from the current view stack
    /// so that rendering code that still uses `active_pane` continues to work.
    pub(super) fn active_pane_from_view(&self) -> ActivePane {
        match self.current_view() {
            View::Dashboard => ActivePane::RepoList,
            View::RepoDetail(_) => ActivePane::Detail,
            View::CommandOutput => ActivePane::CommandOutput,
            View::Help => ActivePane::Help,
        }
    }

    #[allow(dead_code)]
    /// Sync `active_pane` from the view stack (bridge method).
    fn sync_active_pane(&mut self) {
        // ArgumentInput is an overlay, not a stack view — don't clobber it.
        if self.active_pane != ActivePane::ArgumentInput {
            self.active_pane = self.active_pane_from_view();
        }
    }
}
