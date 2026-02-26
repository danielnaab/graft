//! Overlay rendering: help, argument input, form input, command output, stop confirmation.

use super::{
    Alignment, App, ArgumentInputState, Block, Borders, Clear, Color, CommandState, FieldValue,
    FormInputState, KeyCode, KeyModifiers, Line, Modifier, Paragraph, Rect, RepoDetailProvider,
    RepoRegistry, Span, StatusMessage, Style, Wrap,
};

impl<R: RepoRegistry, D: RepoDetailProvider> App<R, D> {
    /// Handle keys in argument input overlay.
    pub(super) fn handle_key_argument_input(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        let Some(state) = &mut self.argument_input else {
            return;
        };

        // Handle Ctrl shortcuts first
        if modifiers.contains(KeyModifiers::CONTROL) {
            match code {
                KeyCode::Char('u') => {
                    state.text.clear();
                    return;
                }
                KeyCode::Char('w') => {
                    state.text.delete_word_backward();
                    return;
                }
                _ => {}
            }
        }

        match code {
            KeyCode::Enter => {
                let args = if state.text.buffer.is_empty() {
                    Vec::new()
                } else if let Ok(parsed_args) = shell_words::split(&state.text.buffer) {
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
                state.text.move_left();
            }
            KeyCode::Right => {
                state.text.move_right();
            }
            KeyCode::Home => {
                state.text.move_home();
            }
            KeyCode::End => {
                state.text.move_end();
            }
            KeyCode::Delete => {
                state.text.delete_forward();
            }
            KeyCode::Char(c) => {
                state.text.insert_char(c);
            }
            KeyCode::Backspace => {
                state.text.backspace();
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

        // Reload recent runs so any new run from this session is visible.
        if let Some(repo_path) = &self.selected_repo_for_commands {
            let repo_name = graft_common::repo_name_from_path(repo_path);
            self.recent_runs = graft_common::list_runs(&self.workspace_name, repo_name, 50);
            self.rebuild_detail_items();
        }
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
            Line::from("  j/k, ↑↓      Navigate items"),
            Line::from("  Enter         Run command (when selected)"),
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

        let chars: Vec<char> = state.text.buffer.chars().collect();
        let before_cursor: String = chars[..state.text.cursor_pos].iter().collect();
        let after_cursor: String = chars[state.text.cursor_pos..].iter().collect();

        let input_text = if after_cursor.is_empty() {
            format!("> {before_cursor}_")
        } else {
            format!("> {before_cursor}▊{after_cursor}")
        };

        let (preview_text, preview_style) = Self::format_argument_preview(state);

        let help = "← → Home End: nav  Ctrl+U: clear  Ctrl+W: del word  Enter: run  Esc: cancel";

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
        if state.text.buffer.is_empty() {
            return (
                format!("Will execute: graft run {}", state.command_name),
                Style::default().fg(Color::Gray),
            );
        }

        match shell_words::split(&state.text.buffer) {
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

    // ===== Form input overlay (schema-driven argument form) =====

    /// Handle keys in the form input overlay.
    #[allow(clippy::too_many_lines)]
    pub(super) fn handle_key_form_input(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        let Some(state) = &mut self.form_input else {
            return;
        };

        match code {
            KeyCode::Esc => {
                self.form_input = None;
                return;
            }
            KeyCode::Enter => {
                // Validate required fields
                if let Some(err) = Self::validate_form(state) {
                    self.status_message = Some(StatusMessage::error(err));
                    return;
                }

                // Find the command definition (run string, working_dir, env)
                let cmd_info = self
                    .available_commands
                    .iter()
                    .find(|(name, _)| *name == state.command_name)
                    .map(|(_, cmd)| (cmd.run.clone(), cmd.working_dir.clone(), cmd.env.clone()));

                let (cmd_run, working_dir, env) = cmd_info.unwrap_or_default();

                // Resolve relative script paths and inject GRAFT_DEP_DIR for dep commands.
                let (resolved_run, env) = if let Some(repo_path) = &self.selected_repo_for_commands
                {
                    if let Some((dep, _)) = state.command_name.split_once(':') {
                        let source_dir =
                            std::path::PathBuf::from(repo_path).join(".graft").join(dep);
                        let resolved =
                            graft_engine::resolve_script_in_command(&cmd_run, &source_dir);
                        let mut env_map = env.unwrap_or_default();
                        env_map.insert(
                            "GRAFT_DEP_DIR".to_string(),
                            source_dir.to_string_lossy().to_string(),
                        );
                        (resolved, Some(env_map))
                    } else {
                        let source_dir = std::path::PathBuf::from(repo_path);
                        let resolved =
                            graft_engine::resolve_script_in_command(&cmd_run, &source_dir);
                        (resolved, env)
                    }
                } else {
                    (cmd_run, env)
                };

                let shell_cmd = Self::assemble_args(&resolved_run, &state.fields);
                let command_name = state.command_name.clone();

                self.form_input = None;
                self.push_view(super::View::CommandOutput);
                self.execute_command_assembled(command_name, shell_cmd, working_dir, env);
                return;
            }
            KeyCode::Tab => {
                if !state.fields.is_empty() {
                    state.focused = (state.focused + 1) % state.fields.len();
                }
                return;
            }
            KeyCode::BackTab => {
                if !state.fields.is_empty() {
                    state.focused = if state.focused == 0 {
                        state.fields.len() - 1
                    } else {
                        state.focused - 1
                    };
                }
                return;
            }
            // Down/Up navigate between fields for text/flag fields, but cycle
            // within options for choice fields (handled by per-field code below).
            KeyCode::Down | KeyCode::Up => {
                let is_choice = state
                    .fields
                    .get(state.focused)
                    .is_some_and(|f| matches!(f.value, FieldValue::Choice(_)));
                if !is_choice && !state.fields.is_empty() {
                    if code == KeyCode::Down {
                        state.focused = (state.focused + 1) % state.fields.len();
                    } else {
                        state.focused = if state.focused == 0 {
                            state.fields.len() - 1
                        } else {
                            state.focused - 1
                        };
                    }
                    return;
                }
                // Choice field: fall through to per-field handler.
            }
            _ => {}
        }

        // Per-field-type key handling on the focused field
        if state.focused >= state.fields.len() {
            return;
        }

        let field = &mut state.fields[state.focused];
        match &mut field.value {
            FieldValue::Text(buf) => {
                if modifiers.contains(KeyModifiers::CONTROL) {
                    match code {
                        KeyCode::Char('u') => buf.clear(),
                        KeyCode::Char('w') => buf.delete_word_backward(),
                        _ => {}
                    }
                    return;
                }
                match code {
                    KeyCode::Char(c) => buf.insert_char(c),
                    KeyCode::Backspace => buf.backspace(),
                    KeyCode::Delete => buf.delete_forward(),
                    KeyCode::Left => buf.move_left(),
                    KeyCode::Right => buf.move_right(),
                    KeyCode::Home => buf.move_home(),
                    KeyCode::End => buf.move_end(),
                    _ => {}
                }
            }
            FieldValue::Choice(idx) => {
                let option_count = field.def.options.as_ref().map_or(0, Vec::len);
                if option_count > 0 {
                    match code {
                        KeyCode::Up | KeyCode::Left | KeyCode::Char('k' | 'h') => {
                            *idx = if *idx == 0 {
                                option_count - 1
                            } else {
                                *idx - 1
                            };
                        }
                        KeyCode::Down | KeyCode::Right | KeyCode::Char('j' | 'l') => {
                            *idx = (*idx + 1) % option_count;
                        }
                        _ => {}
                    }
                }
            }
            FieldValue::Flag(on) => {
                if code == KeyCode::Char(' ') {
                    *on = !*on;
                }
            }
        }
    }

    /// Validate all required form fields. Returns `Some(error_message)` if invalid.
    pub(super) fn validate_form(state: &FormInputState) -> Option<String> {
        for field in &state.fields {
            if !field.def.required {
                continue;
            }
            match &field.value {
                FieldValue::Text(buf) if buf.buffer.trim().is_empty() => {
                    return Some(format!("Required field '{}' is empty", field.def.name));
                }
                FieldValue::Choice(_) if field.def.options.as_ref().is_none_or(Vec::is_empty) => {
                    return Some(format!(
                        "No options available for '{}' — refresh state queries with 'r'",
                        field.def.name
                    ));
                }
                _ => {}
            }
        }
        None
    }

    /// Assemble a shell command string from form field values.
    ///
    /// Two modes:
    /// - **Template interpolation**: if `run` contains `{name}` placeholders, replace them
    ///   with shell-escaped field values (via `graft_engine::substitute_named_placeholders`).
    /// - **Auto-assembly** (default): append positional args, then named `--flag` / `--key val`.
    pub(super) fn assemble_args(run: &str, fields: &[super::FormField]) -> String {
        if graft_engine::has_placeholders(run) {
            let named_args: Vec<(String, String)> = fields
                .iter()
                .filter_map(|field| {
                    Self::field_value_as_string(field).map(|val| (field.def.name.clone(), val))
                })
                .collect();
            let refs: Vec<(&str, &str)> = named_args
                .iter()
                .map(|(n, v)| (n.as_str(), v.as_str()))
                .collect();
            return graft_engine::substitute_named_placeholders(run, &refs).0;
        }

        let mut parts = vec![run.to_string()];

        // Positional args first (in definition order)
        for field in fields.iter().filter(|f| f.def.positional) {
            if let Some(val) = Self::field_value_as_string(field) {
                if !val.is_empty() {
                    parts.push(shell_words::quote(&val).into_owned());
                }
            }
        }

        // Named args
        for field in fields.iter().filter(|f| !f.def.positional) {
            match &field.value {
                FieldValue::Flag(true) => {
                    parts.push(format!("--{}", field.def.name));
                }
                FieldValue::Text(buf) if !buf.buffer.is_empty() => {
                    parts.push(format!("--{}", field.def.name));
                    parts.push(shell_words::quote(&buf.buffer).into_owned());
                }
                FieldValue::Choice(idx) => {
                    if let Some(options) = &field.def.options {
                        if let Some(val) = options.get(*idx) {
                            parts.push(format!("--{}", field.def.name));
                            parts.push(shell_words::quote(val).into_owned());
                        }
                    }
                }
                _ => {}
            }
        }

        parts.join(" ")
    }

    /// Extract the string value of a field (for assembly/interpolation).
    fn field_value_as_string(field: &super::FormField) -> Option<String> {
        match &field.value {
            FieldValue::Text(buf) => Some(buf.buffer.clone()),
            FieldValue::Choice(idx) => field
                .def
                .options
                .as_ref()
                .and_then(|opts| opts.get(*idx).cloned()),
            FieldValue::Flag(on) => {
                if *on {
                    Some("true".to_string())
                } else {
                    Some("false".to_string())
                }
            }
        }
    }

    /// Render the form input overlay as a centered dialog.
    #[allow(clippy::too_many_lines, clippy::cast_possible_truncation)]
    pub(super) fn render_form_input_overlay(&self, frame: &mut ratatui::Frame) {
        let area = frame.area();

        let Some(state) = &self.form_input else {
            return;
        };

        // Calculate dialog height: 1 blank + fields (1 label line + N option lines for
        // Choice, else 1 widget line; plus 1 optional desc line) + 1 blank + 1 footer + 2 border.
        let mut content_lines: usize = 0;
        for field in &state.fields {
            content_lines += 1; // label line
            if let FieldValue::Choice(_) = &field.value {
                // One line per option instead of one wide horizontal line.
                content_lines += field.def.options.as_ref().map_or(0, Vec::len);
            }
            if field.def.description.is_some() {
                content_lines += 1; // description line
            }
        }
        // header blank + content + blank + footer = content_lines + 3
        let inner_height = content_lines + 3;
        let dialog_height = u16::try_from(inner_height + 2)
            .unwrap_or(u16::MAX)
            .min(area.height.saturating_sub(2));
        let dialog_width = 70u16.min(area.width.saturating_sub(4));

        let x = area.x + (area.width.saturating_sub(dialog_width)) / 2;
        let y = area.y + (area.height.saturating_sub(dialog_height)) / 2;

        let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

        frame.render_widget(Clear, dialog_area);

        let title = format!(" Arguments for '{}' ", state.command_name);

        let mut lines: Vec<Line<'static>> = Vec::new();
        lines.push(Line::from(""));

        let inner_width = dialog_width.saturating_sub(4) as usize; // borders + padding
        let label_width = 16usize.min(inner_width / 3);

        for (i, field) in state.fields.iter().enumerate() {
            let is_focused = i == state.focused;

            let label_style = if is_focused {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let req_marker = if field.def.required { " *" } else { "" };
            let label = format!(
                "  {:<width$}",
                format!("{}{req_marker}", field.def.name),
                width = label_width
            );

            let mut spans = vec![Span::styled(label, label_style)];

            // Widget rendering.  Choice fields are rendered vertically (one option
            // per line); all others append their widget to the label spans line.
            match &field.value {
                FieldValue::Text(buf) => {
                    let widget_width = inner_width.saturating_sub(label_width + 4);
                    let display_val: String = if buf.buffer.is_empty() && !is_focused {
                        format!("[{:_<width$}]", "", width = widget_width)
                    } else if is_focused {
                        let chars: Vec<char> = buf.buffer.chars().collect();
                        let before: String = chars[..buf.cursor_pos].iter().collect();
                        let after: String = chars[buf.cursor_pos..].iter().collect();
                        if after.is_empty() {
                            format!("[{before}_]")
                        } else {
                            format!("[{before}|{after}]")
                        }
                    } else {
                        let truncated: String = buf.buffer.chars().take(widget_width).collect();
                        format!("[{truncated}]")
                    };
                    let text_style = if is_focused {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default().fg(Color::Gray)
                    };
                    spans.push(Span::styled(display_val, text_style));
                    lines.push(Line::from(spans));
                }
                FieldValue::Choice(selected_idx) => {
                    // Label line (without options).
                    lines.push(Line::from(spans));
                    // One line per option, indented under the label.
                    let indent = " ".repeat(label_width + 4);
                    if let Some(options) = &field.def.options {
                        for (oi, opt) in options.iter().enumerate() {
                            let is_selected = oi == *selected_idx;
                            let marker = if is_selected { "● " } else { "  " };
                            let opt_style = if is_focused && is_selected {
                                Style::default()
                                    .fg(Color::Cyan)
                                    .add_modifier(Modifier::BOLD)
                            } else if is_selected {
                                Style::default().fg(Color::White)
                            } else {
                                Style::default().fg(Color::DarkGray)
                            };
                            lines.push(Line::from(Span::styled(
                                format!("{indent}{marker}{opt}"),
                                opt_style,
                            )));
                        }
                    } else {
                        lines.push(Line::from(Span::styled(
                            format!("{indent}(no options — refresh with r)"),
                            Style::default().fg(Color::DarkGray),
                        )));
                    }
                }
                FieldValue::Flag(on) => {
                    let marker = if *on { "[x]" } else { "[ ]" };
                    let flag_style = if is_focused {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    spans.push(Span::styled(marker.to_string(), flag_style));
                    lines.push(Line::from(spans));
                }
            }

            // Description line (applies to all field types).
            if let Some(desc) = &field.def.description {
                let indent = " ".repeat(label_width + 2);
                lines.push(Line::from(Span::styled(
                    format!("{indent}{desc}"),
                    Style::default().fg(Color::DarkGray),
                )));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Tab: next field  j/k: select option  Enter: run  Esc: cancel",
            Style::default().fg(Color::Gray),
        )));

        let paragraph = Paragraph::new(lines)
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

    /// Handle keys in the approval overlay.
    ///
    /// `a` → approve, push `CommandOutput`, execute approve command, reload run-state.
    /// `r` → reject, same with reject command.
    /// `Esc` → dismiss without action.
    pub(super) fn handle_key_approval_overlay(&mut self, code: KeyCode) {
        let Some(state) = self.approval_overlay.clone() else {
            return;
        };

        match code {
            KeyCode::Char('a') => {
                self.approval_overlay = None;
                self.push_view(super::View::CommandOutput);
                self.execute_command_with_args(state.approve_cmd, vec![]);
                self.run_state_entries.clear();
            }
            KeyCode::Char('r') => {
                self.approval_overlay = None;
                self.push_view(super::View::CommandOutput);
                self.execute_command_with_args(state.reject_cmd, vec![]);
                self.run_state_entries.clear();
            }
            KeyCode::Esc => {
                self.approval_overlay = None;
            }
            _ => {}
        }
    }

    /// Render the approval overlay as a centered modal.
    ///
    /// Displays the sequence name, message, and `a`/`r`/`Esc` keybindings.
    /// Follows the same centered-popup pattern as `render_stop_confirmation_dialog`.
    pub(super) fn render_approval_overlay(&self, frame: &mut ratatui::Frame) {
        let Some(state) = &self.approval_overlay else {
            return;
        };

        let dialog_width = 64;
        let dialog_height = 9;

        let area = frame.area();
        let x = area.x + (area.width.saturating_sub(dialog_width)) / 2;
        let y = area.y + (area.height.saturating_sub(dialog_height)) / 2;

        let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

        frame.render_widget(Clear, dialog_area);

        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("Sequence '{}' completed.", state.sequence),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                state.message.as_str(),
                Style::default().fg(Color::White),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "a = Approve   r = Reject   Esc = Cancel",
                Style::default().fg(Color::Cyan),
            )),
        ];

        let dialog = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow))
                    .title(" Review Checkpoint ")
                    .style(Style::default().bg(Color::Black)),
            )
            .alignment(Alignment::Center);

        frame.render_widget(dialog, dialog_area);
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
