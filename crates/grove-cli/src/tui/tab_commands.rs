//! Commands tab: command list and execution.

use super::{
    Alignment, App, ArgumentInputMode, ArgumentInputState, Color, GraftYamlLoader, KeyCode, Line,
    List, ListItem, Modifier, Paragraph, Rect, RepoDetailProvider, RepoRegistry, StatusMessage,
    Style,
};

impl<R: RepoRegistry, D: RepoDetailProvider> App<R, D> {
    /// Handle keys when the Commands tab is active.
    pub(super) fn handle_key_commands_tab(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('j') | KeyCode::Down => {
                let i = self.command_picker_state.selected().unwrap_or(0);
                if i + 1 < self.available_commands.len() {
                    self.command_picker_state.select(Some(i + 1));
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let i = self.command_picker_state.selected().unwrap_or(0);
                if i > 0 {
                    self.command_picker_state.select(Some(i - 1));
                }
            }
            KeyCode::Enter => {
                self.execute_selected_command();
            }
            _ => {}
        }
    }

    /// Render the Commands tab content in the given area.
    pub(super) fn render_commands_tab(&mut self, frame: &mut ratatui::Frame, area: Rect) {
        if self.available_commands.is_empty() {
            let empty_text = vec![
                Line::from(""),
                Line::from("  No commands defined in graft.yaml")
                    .style(Style::default().fg(Color::Yellow)),
                Line::from(""),
                Line::from("  Commands let you run project tasks from Grove:")
                    .style(Style::default().fg(Color::DarkGray)),
                Line::from("  • Build, test, lint, deploy")
                    .style(Style::default().fg(Color::DarkGray)),
                Line::from("  • Custom project scripts")
                    .style(Style::default().fg(Color::DarkGray)),
                Line::from(""),
                Line::from("  Example graft.yaml configuration:")
                    .style(Style::default().fg(Color::White)),
                Line::from(""),
                Line::from("    commands:").style(Style::default().fg(Color::Cyan)),
                Line::from("      test:").style(Style::default().fg(Color::Cyan)),
                Line::from("        run: \"cargo test\"").style(Style::default().fg(Color::Green)),
                Line::from("        description: \"Run test suite\"")
                    .style(Style::default().fg(Color::Green)),
                Line::from("      lint:").style(Style::default().fg(Color::Cyan)),
                Line::from("        run: \"cargo clippy\"")
                    .style(Style::default().fg(Color::Green)),
            ];

            let paragraph = Paragraph::new(empty_text).alignment(Alignment::Left);
            frame.render_widget(paragraph, area);
        } else {
            let items: Vec<ListItem> = self
                .available_commands
                .iter()
                .map(|(name, cmd)| {
                    let desc = cmd.description.as_deref().unwrap_or("");
                    let content = format!("{name:<20} {desc}");
                    ListItem::new(content)
                })
                .collect();

            let list = List::new(items)
                .highlight_style(
                    Style::default()
                        .bg(Color::Rgb(40, 40, 50))
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("▶ ");

            frame.render_stateful_widget(list, area, &mut self.command_picker_state);
        }
    }

    /// Load commands for the currently selected repository.
    pub(super) fn load_commands_for_selected_repo(&mut self) {
        let Some(selected) = self.list_state.selected() else {
            return;
        };

        let repos = self.registry.list_repos();
        if selected >= repos.len() {
            return;
        }

        let repo_path = repos[selected].as_path().display().to_string();

        // Check cache - avoid re-parsing if same repo
        if self.selected_repo_for_commands.as_ref() == Some(&repo_path) {
            return; // Already loaded
        }

        // Load graft.yaml
        let graft_path = format!("{repo_path}/graft.yaml");
        let graft_config = match self.graft_loader.load_graft(&graft_path) {
            Ok(config) => config,
            Err(e) => {
                self.status_message = Some(StatusMessage::error(format!(
                    "Error loading graft.yaml: {e}"
                )));
                self.available_commands.clear();
                self.selected_repo_for_commands = Some(repo_path);
                return;
            }
        };

        // Populate commands list
        self.available_commands = graft_config.commands.into_iter().collect();
        self.available_commands.sort_by(|a, b| a.0.cmp(&b.0));
        self.selected_repo_for_commands = Some(repo_path);

        if !self.available_commands.is_empty() {
            self.command_picker_state.select(Some(0));
        }
    }

    /// Execute the currently selected command.
    pub(super) fn execute_selected_command(&mut self) {
        let Some(cmd_idx) = self.command_picker_state.selected() else {
            return;
        };

        if cmd_idx >= self.available_commands.len() {
            return;
        }

        let (cmd_name, _cmd) = &self.available_commands[cmd_idx];

        self.argument_input = Some(ArgumentInputState {
            buffer: String::new(),
            cursor_pos: 0,
            command_name: cmd_name.clone(),
        });
        self.argument_input_mode = ArgumentInputMode::Active;
    }
}
