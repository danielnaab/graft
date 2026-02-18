//! Overlay rendering: help, argument input, command output, stop confirmation.

use super::{
    Alignment, App, ArgumentInputState, Block, Borders, Clear, Color, CommandState, KeyCode, Line,
    Modifier, Paragraph, Rect, RepoDetailProvider, RepoRegistry, Span, StatusMessage, Style, Wrap,
};

impl<R: RepoRegistry, D: RepoDetailProvider> App<R, D> {
    /// Handle keys in argument input overlay.
    pub(super) fn handle_key_argument_input(&mut self, code: KeyCode) {
        let Some(state) = &mut self.argument_input else {
            return;
        };

        match code {
            KeyCode::Enter => {
                let args = if state.buffer.is_empty() {
                    Vec::new()
                } else if let Ok(parsed_args) = shell_words::split(&state.buffer) {
                    parsed_args
                } else {
                    self.status_message = Some(StatusMessage::error(
                        "Cannot execute: fix parsing error first",
                    ));
                    return;
                };

                let command_name = state.command_name.clone();

                self.argument_input = None;
                // Dismiss the overlay, then push CommandOutput view.
                self.push_view(super::View::CommandOutput);

                self.execute_command_with_args(command_name, args);
            }
            KeyCode::Esc => {
                self.argument_input = None;
                // Dismiss the overlay; underlying view stays unchanged.
            }
            KeyCode::Left => {
                if state.cursor_pos > 0 {
                    state.cursor_pos -= 1;
                }
            }
            KeyCode::Right => {
                let char_count = state.buffer.chars().count();
                if state.cursor_pos < char_count {
                    state.cursor_pos += 1;
                }
            }
            KeyCode::Home => {
                state.cursor_pos = 0;
            }
            KeyCode::End => {
                state.cursor_pos = state.buffer.chars().count();
            }
            KeyCode::Char(c) => {
                let mut chars: Vec<char> = state.buffer.chars().collect();
                chars.insert(state.cursor_pos, c);
                state.buffer = chars.into_iter().collect();
                state.cursor_pos += 1;
            }
            KeyCode::Backspace => {
                if state.cursor_pos > 0 {
                    let mut chars: Vec<char> = state.buffer.chars().collect();
                    chars.remove(state.cursor_pos - 1);
                    state.buffer = chars.into_iter().collect();
                    state.cursor_pos -= 1;
                }
            }
            _ => {}
        }
    }

    /// Handle keys in command output overlay.
    pub(super) fn handle_key_command_output(&mut self, code: KeyCode) {
        if self.show_stop_confirmation {
            match code {
                KeyCode::Char('y' | 'Y') => {
                    if let Some(pid) = self.running_command_pid {
                        #[cfg(unix)]
                        {
                            use nix::sys::signal::{kill, Signal};
                            use nix::unistd::Pid;

                            match kill(Pid::from_raw(pid.cast_signed()), Signal::SIGTERM) {
                                Ok(()) => {
                                    self.status_message =
                                        Some(StatusMessage::info("Stopping command..."));
                                }
                                Err(e) => {
                                    self.status_message = Some(StatusMessage::error(format!(
                                        "Failed to stop command: {e}"
                                    )));
                                }
                            }
                        }
                        #[cfg(not(unix))]
                        {
                            self.status_message = Some(StatusMessage::warning(
                                "Command cancellation not supported on Windows",
                            ));
                        }

                        self.running_command_pid = None;
                    }

                    self.show_stop_confirmation = false;
                    self.clear_command_output_state();
                    self.pop_view();
                }
                KeyCode::Char('n' | 'N') | KeyCode::Esc => {
                    self.show_stop_confirmation = false;
                }
                _ => {}
            }
            return;
        }

        match code {
            KeyCode::Char('j') | KeyCode::Down => {
                if self.output_scroll + 1 < self.output_lines.len() {
                    self.output_scroll += 1;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.output_scroll > 0 {
                    self.output_scroll -= 1;
                }
            }
            KeyCode::Char('q') => {
                if matches!(self.command_state, CommandState::Running) {
                    self.show_stop_confirmation = true;
                } else {
                    // q pops back one level
                    self.clear_command_output_state();
                    self.pop_view();
                }
            }
            KeyCode::Esc => {
                if matches!(self.command_state, CommandState::Running) {
                    self.show_stop_confirmation = true;
                } else {
                    // Escape goes home (Dashboard)
                    self.clear_command_output_state();
                    self.reset_to_dashboard();
                }
            }
            _ => {}
        }
    }

    /// Clear command output state after closing the `CommandOutput` view.
    fn clear_command_output_state(&mut self) {
        self.output_lines.clear();
        self.output_scroll = 0;
        self.command_state = CommandState::NotStarted;
        self.command_name = None;
        self.output_truncated_start = false;
        self.command_event_rx = None;
        self.running_command_pid = None;
    }

    /// Render the help view as a full-width centered popup.
    #[allow(
        clippy::unused_self,
        clippy::too_many_lines,
        clippy::cast_possible_truncation
    )]
    pub(super) fn render_help_view(&self, frame: &mut ratatui::Frame, area: Rect) {
        if area.width < 44 || area.height < 20 {
            let msg = "Terminal too small for help. Resize or press any key.";
            let warning = Paragraph::new(msg).alignment(Alignment::Center).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            );
            frame.render_widget(warning, area);
            return;
        }

        let version = env!("CARGO_PKG_VERSION");

        let help_text = vec![
            Line::from(Span::styled(
                format!("Grove v{version} — Help"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Dashboard",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from("  j/k, ↑↓      Navigate repositories"),
            Line::from("  Enter/Tab     Open repository detail"),
            Line::from("  r             Refresh all repository statuses"),
            Line::from("  q             Quit"),
            Line::from("  Esc           No-op (already home)"),
            Line::from(""),
            Line::from(Span::styled(
                "Repository Detail",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from("  j/k, ↑↓      Scroll content"),
            Line::from("  n / p         Select next / previous command"),
            Line::from("  Enter         Run selected command (opens arg dialog)"),
            Line::from("  r             Refresh state queries"),
            Line::from("  q, Tab        Back to Dashboard"),
            Line::from("  Esc           Go to Dashboard (from any view)"),
            Line::from(""),
            Line::from(Span::styled(
                "Command Line  ( : )",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from("  :help         Show this help"),
            Line::from("  :quit         Quit Grove"),
            Line::from("  :refresh      Refresh all repositories"),
            Line::from("  :repo <n>     Jump to repo (name or 1-based index)"),
            Line::from("  :run <cmd>    Execute command in current repository"),
            Line::from("  :state        Refresh state queries"),
            Line::from("  j/k           Navigate palette  |  Esc: cancel"),
            Line::from(""),
            Line::from(Span::styled(
                "Status Indicators",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("●", Style::default().fg(Color::Yellow)),
                Span::raw("  Uncommitted changes (dirty)"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("○", Style::default().fg(Color::Green)),
                Span::raw("  Clean working tree"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("↑n", Style::default().fg(Color::Green)),
                Span::raw("  Commits ahead of remote"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("↓n", Style::default().fg(Color::Red)),
                Span::raw("  Commits behind remote"),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "q: close   Esc: home",
                Style::default().fg(Color::Gray),
            )),
        ];

        let popup_width = 60.min(area.width.saturating_sub(4)).max(40);
        let popup_height = (help_text.len() as u16 + 2)
            .min(area.height.saturating_sub(4))
            .max(20);

        let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
        let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;

        let popup_area = Rect {
            x: popup_x,
            y: popup_y,
            width: popup_width,
            height: popup_height,
        };

        frame.render_widget(Clear, popup_area);

        let help_widget = Paragraph::new(help_text)
            .block(
                Block::default()
                    .title("Help")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .style(Style::default().bg(Color::Black))
            .alignment(Alignment::Left);

        frame.render_widget(help_widget, popup_area);
    }

    /// Render the argument input overlay as a centered dialog.
    #[allow(clippy::too_many_lines)]
    pub(super) fn render_argument_input_overlay(&self, frame: &mut ratatui::Frame) {
        let area = frame.area();

        let Some(state) = &self.argument_input else {
            return;
        };

        let dialog_width = 70.min(area.width.saturating_sub(4));
        let dialog_height = 9;

        let x = area.x + (area.width.saturating_sub(dialog_width)) / 2;
        let y = area.y + (area.height.saturating_sub(dialog_height)) / 2;

        let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

        frame.render_widget(Clear, dialog_area);

        let title = format!(" Arguments for '{}' ", state.command_name);

        let chars: Vec<char> = state.buffer.chars().collect();
        let before_cursor: String = chars[..state.cursor_pos].iter().collect();
        let after_cursor: String = chars[state.cursor_pos..].iter().collect();

        let input_text = if after_cursor.is_empty() {
            format!("> {before_cursor}_")
        } else {
            format!("> {before_cursor}▊{after_cursor}")
        };

        let (preview_text, preview_style) = Self::format_argument_preview(state);

        let help = "← →  Home  End: navigate   Enter: run   Esc: cancel";

        let content = vec![
            Line::from(""),
            Line::from(input_text).style(Style::default().fg(Color::Cyan)),
            Line::from(""),
            Line::from(preview_text).style(preview_style),
            Line::from(""),
            Line::from(help).style(Style::default().fg(Color::Gray)),
        ];

        let paragraph = Paragraph::new(content)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(Style::default().fg(Color::Yellow))
                    .style(Style::default().bg(Color::Black)),
            )
            .alignment(Alignment::Left);

        frame.render_widget(paragraph, dialog_area);
    }

    /// Format the command preview line showing how arguments will be parsed.
    fn format_argument_preview(state: &ArgumentInputState) -> (String, Style) {
        if state.buffer.is_empty() {
            return (
                format!("Will execute: graft run {}", state.command_name),
                Style::default().fg(Color::Gray),
            );
        }

        match shell_words::split(&state.buffer) {
            Ok(args) => {
                let quoted_args: Vec<String> = args
                    .iter()
                    .map(|arg| {
                        if arg.contains(' ') || arg.contains('\"') || arg.contains('\'') {
                            format!("'{arg}'")
                        } else {
                            arg.clone()
                        }
                    })
                    .collect();

                let preview = if quoted_args.is_empty() {
                    format!("Will execute: graft run {}", state.command_name)
                } else {
                    format!(
                        "Will execute: graft run {} {}",
                        state.command_name,
                        quoted_args.join(" ")
                    )
                };

                (preview, Style::default().fg(Color::Green))
            }
            Err(e) => (
                format!("⚠ Parse error: {e} - fix before running"),
                Style::default().fg(Color::Red),
            ),
        }
    }

    /// Render the command output view (full-width).
    #[allow(clippy::too_many_lines, clippy::cast_possible_truncation)]
    pub(super) fn render_command_output_view(&mut self, frame: &mut ratatui::Frame, area: Rect) {
        let header = match &self.command_state {
            CommandState::Running => format!(
                " Running: {} (j/k: scroll, q: close) ",
                self.command_name.as_deref().unwrap_or("unknown")
            ),
            CommandState::Completed { exit_code } => {
                if *exit_code == 0 {
                    format!(
                        " ✓ {}: Completed successfully (exit {exit_code}) - Press q to close ",
                        self.command_name.as_deref().unwrap_or("unknown"),
                    )
                } else {
                    format!(
                        " ✗ {}: Failed with exit code {exit_code} - Press q to close ",
                        self.command_name.as_deref().unwrap_or("unknown"),
                    )
                }
            }
            CommandState::Failed { error } => {
                format!(" ✗ Failed: {error} - Press q to close ")
            }
            CommandState::NotStarted => " Output ".to_string(),
        };

        let block = Block::default()
            .title(header)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::Black));

        let inner = block.inner(area);
        let visible_height = inner.height as usize;
        let start = self.output_scroll;
        let end = (start + visible_height).min(self.output_lines.len());
        let visible_lines: Vec<Line> = self.output_lines[start..end]
            .iter()
            .map(|line| Line::from(line.clone()))
            .collect();

        let max_scroll = self.output_lines.len().saturating_sub(visible_height);
        self.output_scroll = self.output_scroll.min(max_scroll);

        let paragraph = Paragraph::new(visible_lines)
            .block(block)
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);

        if self.output_truncated_start {
            let warning_area = Rect {
                x: area.x + 2,
                y: area.y + area.height.saturating_sub(3),
                width: 50.min(area.width.saturating_sub(4)),
                height: 1,
            };
            let warning = Paragraph::new(" ⚠ Output truncated (exceeded 1MB limit) ")
                .style(Style::default().fg(Color::Black).bg(Color::Yellow));
            frame.render_widget(warning, warning_area);
        }
    }

    /// Render the stop confirmation dialog as a centered popup.
    #[allow(clippy::unused_self)]
    pub(super) fn render_stop_confirmation_dialog(&self, frame: &mut ratatui::Frame) {
        let dialog_width = 60;
        let dialog_height = 7;

        let area = frame.area();
        let x = area.x + (area.width.saturating_sub(dialog_width)) / 2;
        let y = area.y + (area.height.saturating_sub(dialog_height)) / 2;

        let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

        frame.render_widget(Clear, dialog_area);

        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "Stop running command?",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "This will send SIGTERM to the process.",
                Style::default().fg(Color::White),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "y = Yes, stop   n = No, continue   Esc = Cancel",
                Style::default().fg(Color::Cyan),
            )),
        ];

        let dialog = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow))
                    .title(" Confirm ")
                    .style(Style::default().bg(Color::Black)),
            )
            .alignment(Alignment::Center);

        frame.render_widget(dialog, dialog_area);
    }
}
