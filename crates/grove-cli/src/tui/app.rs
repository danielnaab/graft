//! App struct construction, key dispatch, navigation, and data loading.

use super::{
    App, CommandLineState, CommandState, GraftYamlConfigLoader, KeyCode, KeyEvent, KeyModifiers,
    ListState, RepoDetail, RepoDetailProvider, RepoRegistry, StatusMessage, View,
    DEFAULT_MAX_COMMITS,
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
            view_stack: vec![View::Dashboard],
            command_line: None,
            command_history: Vec::new(),
            detail_scroll: 0,
            cached_detail: None,
            cached_detail_index: None,
            workspace_name,
            status_message: None,
            needs_refresh: false,

            // Unified cursor for detail view
            detail_cursor: 0,
            detail_items: Vec::new(),

            // Command execution state
            available_commands: Vec::new(),
            selected_repo_for_commands: None,
            argument_input: None,
            form_input: None,
            output_lines: Vec::new(),
            output_scroll: 0,
            output_truncated_start: false,
            command_state: CommandState::NotStarted,
            command_name: None,
            graft_loader: GraftYamlConfigLoader::new(),
            command_event_rx: None,
            running_command_pid: None,
            show_stop_confirmation: false,

            // State queries
            state_queries: Vec::new(),
            state_results: Vec::new(),
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
            self.detail_items.clear();
            self.detail_cursor = 0;
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

    /// Top-level key event handler that threads modifiers to overlays.
    ///
    /// Called from the event loop with the full `KeyEvent`. Delegates
    /// to `handle_key` with the extracted code and modifiers.
    pub(super) fn handle_key_event(&mut self, key: KeyEvent) {
        self.handle_key(key.code, key.modifiers);
    }

    pub(super) fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        // Form overlay — intercept before argument input and view dispatch.
        if self.form_input.is_some() {
            self.handle_key_form_input(code, modifiers);
            return;
        }

        // ArgumentInput is an overlay — intercept before view dispatch.
        if self.argument_input.is_some() {
            self.handle_key_argument_input(code, modifiers);
            return;
        }

        // Command line is an overlay — intercept before view dispatch.
        if self.command_line.is_some() {
            self.handle_key_command_line(code, modifiers);
            return;
        }

        // `:` activates command line from any view.
        if code == KeyCode::Char(':') {
            self.command_line = Some(CommandLineState {
                text: super::text_buffer::TextBuffer::new(),
                palette_selected: 0,
                history_index: None,
                history_draft: String::new(),
            });
            return;
        }

        match self.current_view() {
            View::Dashboard => self.handle_key_dashboard(code),
            View::RepoDetail(_) => self.handle_key_repo_detail(code),
            View::Help => self.handle_key_help(code),
            View::CommandOutput => self.handle_key_command_output(code),
        }
    }

    fn handle_key_dashboard(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') => {
                // q from Dashboard quits the application
                self.should_quit = true;
            }
            KeyCode::Esc => {
                // Escape goes home (already at Dashboard — no-op, but consistent semantics)
                self.reset_to_dashboard();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.next();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.previous();
            }
            // Enter, Tab, 'x', 's' all open RepoDetail (unified view shows all sections)
            KeyCode::Enter | KeyCode::Tab | KeyCode::Char('x' | 's') => {
                if let Some(idx) = self.list_state.selected() {
                    self.push_view(View::RepoDetail(idx));
                }
            }
            KeyCode::Char('r') => {
                self.needs_refresh = true;
                self.status_message = Some(StatusMessage::info("Refreshing..."));
            }
            KeyCode::Char('?') => {
                self.push_view(View::Help);
            }
            _ => {}
        }
    }

    fn handle_key_help(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc => {
                // Escape goes home (Dashboard), even from Help
                self.reset_to_dashboard();
            }
            KeyCode::Char(_) | KeyCode::Enter | KeyCode::Backspace => {
                self.pop_view();
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

        // Invalidate cached repo data for lazy reload
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

        // Invalidate cached repo data for lazy reload
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
        self.detail_cursor = 0;
        self.rebuild_detail_items();
    }

    // ===== View stack helpers =====

    /// Returns the current (top-of-stack) view.
    pub(super) fn current_view(&self) -> &View {
        self.view_stack.last().expect("view_stack is never empty")
    }

    /// Push a view onto the stack.
    pub(super) fn push_view(&mut self, view: View) {
        self.view_stack.push(view);
    }

    /// Pop the top view from the stack (minimum: Dashboard stays).
    pub(super) fn pop_view(&mut self) {
        if self.view_stack.len() > 1 {
            self.view_stack.pop();
        }
    }

    /// Reset the stack to just Dashboard.
    pub(super) fn reset_to_dashboard(&mut self) {
        self.view_stack.clear();
        self.view_stack.push(View::Dashboard);
    }

    /// Reset the stack to a single specified view (replaces everything).
    pub(super) fn reset_to_view(&mut self, view: View) {
        self.view_stack.clear();
        self.view_stack.push(view);
    }
}
