//! Main render method and layout composition.

use super::{
    io, ActivePane, App, Block, Borders, Color, Constraint, CrosstermBackend, DetailTab, Direction,
    Layout, RepoDetailProvider, RepoRegistry, Result, Style, Terminal,
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

            // Split content area into repo list and detail pane
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                .split(content_area);

            // --- Left pane: repo list ---
            self.render_repo_list(frame, chunks[0]);

            // --- Right pane: detail with tabs ---
            let detail_border_color = if self.active_pane == ActivePane::Detail {
                Color::Cyan
            } else {
                Color::DarkGray
            };

            let tab_title = self.render_tab_header();

            let block = Block::default()
                .title(tab_title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(detail_border_color));

            let inner = block.inner(chunks[1]);
            frame.render_widget(block, chunks[1]);

            // Render active tab content
            match self.active_tab {
                DetailTab::Changes => self.render_changes_tab(frame, inner),
                DetailTab::State => self.render_state_tab(frame, inner),
                DetailTab::Commands => self.render_commands_tab(frame, inner),
            }

            // --- Overlays (rendered on top if active) ---
            if self.active_pane == ActivePane::Help {
                self.render_help_overlay(frame);
            }

            if self.active_pane == ActivePane::ArgumentInput {
                self.render_argument_input_overlay(frame);
            }

            if self.active_pane == ActivePane::CommandOutput {
                self.render_command_output_overlay(frame);
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
