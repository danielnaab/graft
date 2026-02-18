//! Main render method and layout composition.

use super::{
    io, App, Block, Borders, Clear, Color, Constraint, CrosstermBackend, Direction, Layout, Line,
    List, ListItem, ListState, Paragraph, RepoDetailProvider, RepoRegistry, Result, Span, Style,
    Terminal, View,
};
use crate::tui::command_line::filtered_palette;

impl<R: RepoRegistry, D: RepoDetailProvider> App<R, D> {
    /// Render the command palette popup above the command line area.
    ///
    /// The palette is shown whenever the command line is active. It lists all
    /// commands whose name contains the current buffer text (case-insensitive),
    /// with the currently selected entry highlighted. The popup is anchored just
    /// above the status bar row.
    pub(super) fn render_command_palette(
        &self,
        frame: &mut ratatui::Frame,
        above_area: super::Rect,
    ) {
        let Some(state) = &self.command_line else {
            return;
        };

        let entries = filtered_palette(&state.buffer);
        if entries.is_empty() {
            return;
        }

        // Palette height: entries + 2 for border (cap at available space - 1).
        // PALETTE_COMMANDS has at most 6 entries so the cast is safe, but we
        // clamp anyway to defend against future entries and pass clippy.
        let max_height = above_area.height.saturating_sub(1);
        let entries_u16 = u16::try_from(entries.len()).unwrap_or(u16::MAX);
        let palette_height = entries_u16.saturating_add(2).min(max_height);
        if palette_height < 3 {
            return; // Not enough space to show anything meaningful
        }

        // Palette width: longest "command  description" line + borders, capped.
        let max_entry_len = entries
            .iter()
            .map(|e| e.command.len() + 2 + e.description.len())
            .max()
            .unwrap_or(20);
        let max_entry_width = u16::try_from(max_entry_len).unwrap_or(u16::MAX);
        let palette_width = max_entry_width.saturating_add(4).min(above_area.width);

        // Position: bottom-right of content area, just above the status bar.
        let x = above_area.x;
        let y = above_area
            .y
            .saturating_add(above_area.height)
            .saturating_sub(palette_height);

        let palette_area = super::Rect {
            x,
            y,
            width: palette_width,
            height: palette_height,
        };

        // Clear background before drawing
        frame.render_widget(Clear, palette_area);

        // Build list items
        let items: Vec<ListItem> = entries
            .iter()
            .map(|e| {
                let line = Line::from(vec![
                    Span::styled(
                        format!("{:<10}", e.command),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::styled(
                        format!("  {}", e.description),
                        Style::default().fg(Color::White),
                    ),
                ]);
                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Commands ")
                    .border_style(Style::default().fg(Color::Cyan))
                    .style(Style::default().fg(Color::White).bg(Color::Black)),
            )
            .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan));

        let mut list_state = ListState::default();
        list_state.select(Some(state.palette_selected));

        frame.render_stateful_widget(list, palette_area, &mut list_state);
    }

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
            // Clear every cell before drawing so no previous terminal content
            // bleeds through areas that widgets don't explicitly paint.
            frame.render_widget(Clear, frame.area());

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
            if self.argument_input.is_some() {
                self.render_argument_input_overlay(frame);
            }

            if self.show_stop_confirmation {
                self.render_stop_confirmation_dialog(frame);
            }

            // --- Bottom bar: command line when active, status bar otherwise ---
            if self.command_line.is_some() {
                // Render palette popup above the status bar, then the command line itself.
                self.render_command_palette(frame, content_area);
                self.render_command_line(frame, status_bar_area);
            } else {
                self.render_status_bar(frame, status_bar_area);
            }
        })?;

        Ok(())
    }
}
