//! Main render method and layout composition.

use super::{
    io, App, ArgumentInputMode, Constraint, CrosstermBackend, Direction, Layout,
    RepoDetailProvider, RepoRegistry, Result, Terminal, View,
};

impl<R: RepoRegistry, D: RepoDetailProvider> App<R, D> {
    pub(super) fn render(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<()> {
        // Clear expired status messages
        self.clear_expired_status_message();

        // Handle command events
        self.handle_command_events();

        self.ensure_detail_loaded();

        terminal.draw(|frame| {
            // Main layout: content area + status bar at bottom
            let main_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(3),    // Content area (flexible)
                    Constraint::Length(1), // Status bar (1 line)
                ])
                .split(frame.area());

            let content_area = main_chunks[0];
            let status_bar_area = main_chunks[1];

            // Dispatch to per-view full-width renderer
            match self.current_view().clone() {
                View::Dashboard => {
                    self.render_repo_list(frame, content_area);
                }
                View::RepoDetail(_) => {
                    self.render_repo_detail_view(frame, content_area);
                }
                View::Help => {
                    self.render_help_view(frame, content_area);
                }
                View::CommandOutput => {
                    self.render_command_output_view(frame, content_area);
                }
            }

            // --- Overlays rendered on top when active ---
            if self.argument_input_mode == ArgumentInputMode::Active {
                self.render_argument_input_overlay(frame);
            }

            if self.show_stop_confirmation {
                self.render_stop_confirmation_dialog(frame);
            }

            // --- Status bar (always rendered at bottom) ---
            self.render_status_bar(frame, status_bar_area);
        })?;

        Ok(())
    }
}
