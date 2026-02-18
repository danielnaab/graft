//! Main render method and layout composition.

use super::{
    io, App, ArgumentInputMode, Color, Constraint, CrosstermBackend, Direction, Layout, Line,
    Paragraph, RepoDetailProvider, RepoRegistry, Result, Span, Style, Terminal, View,
};

impl<R: RepoRegistry, D: RepoDetailProvider> App<R, D> {
    /// Render the vim-style `:` command line at the bottom of the screen.
    ///
    /// Shows `:<buffer>` with a cursor indicator. Replaces the hint bar when active.
    pub(super) fn render_command_line(&self, frame: &mut ratatui::Frame, area: super::Rect) {
        let Some(state) = &self.command_line else {
            return;
        };

        let chars: Vec<char> = state.buffer.chars().collect();
        let before_cursor: String = chars[..state.cursor_pos].iter().collect();
        let after_cursor: String = chars[state.cursor_pos..].iter().collect();

        // Build spans: ":" prompt, text before cursor, cursor block, text after cursor
        let mut spans = vec![
            Span::styled(":", Style::default().fg(Color::Cyan)),
            Span::styled(before_cursor, Style::default().fg(Color::White)),
        ];

        if after_cursor.is_empty() {
            // Cursor at end: show underscore
            spans.push(Span::styled("_", Style::default().fg(Color::White)));
        } else {
            let mut after_chars = after_cursor.chars();
            let cursor_char = after_chars.next().unwrap_or(' ');
            let rest: String = after_chars.collect();
            // Cursor char highlighted as block
            spans.push(Span::styled(
                cursor_char.to_string(),
                Style::default().fg(Color::Black).bg(Color::White),
            ));
            if !rest.is_empty() {
                spans.push(Span::styled(rest, Style::default().fg(Color::White)));
            }
        }

        // Ratatui clips content to the widget area naturally.
        let line = Line::from(spans);
        let widget = Paragraph::new(line).style(Style::default().bg(Color::DarkGray));
        frame.render_widget(widget, area);
    }

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

            // --- Bottom bar: command line when active, status bar otherwise ---
            if self.command_line.is_some() {
                self.render_command_line(frame, status_bar_area);
            } else {
                self.render_status_bar(frame, status_bar_area);
            }
        })?;

        Ok(())
    }
}
