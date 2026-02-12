//! Terminal UI implementation using ratatui.

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use grove_core::{
    Command, CommandState, FileChangeStatus, GraftYamlLoader, RepoDetail, RepoDetailProvider,
    RepoRegistry, RepoStatus,
};
use grove_engine::GraftYamlConfigLoader;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Terminal,
};
use std::io;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};
use unicode_width::UnicodeWidthStr;

/// Default number of recent commits to show in the detail pane.
const DEFAULT_MAX_COMMITS: usize = 10;

/// Maximum bytes of command output to buffer (1MB)
const MAX_OUTPUT_BYTES: usize = 1_048_576;

/// Check if the terminal supports Unicode characters.
///
/// Returns false for terminals known to have poor Unicode support.
fn supports_unicode() -> bool {
    std::env::var("TERM")
        .map(|term| {
            !term.contains("linux") && !term.contains("ascii") && !term.contains("vt100")
        })
        .unwrap_or(true) // Default to Unicode support
}

/// Message types for status bar.
#[derive(Debug, Clone, PartialEq, Eq)]
enum MessageType {
    Error,
    Warning,
    Info,
    Success,
}

impl MessageType {
    /// Get the symbol for this message type.
    ///
    /// Returns Unicode symbols when supported, ASCII fallback otherwise.
    fn symbol(&self, unicode: bool) -> &'static str {
        match (self, unicode) {
            (MessageType::Error, true) => "✗",
            (MessageType::Error, false) => "X",
            (MessageType::Warning, true) => "⚠",
            (MessageType::Warning, false) => "!",
            (MessageType::Info, true) => "ℹ",
            (MessageType::Info, false) => "i",
            (MessageType::Success, true) => "✓",
            (MessageType::Success, false) => "*",
        }
    }

    /// Get the foreground color for this message type.
    fn fg_color(&self) -> Color {
        match self {
            MessageType::Error => Color::White,
            MessageType::Warning => Color::Black,
            MessageType::Info => Color::White,
            MessageType::Success => Color::Black,
        }
    }

    /// Get the background color for this message type.
    fn bg_color(&self) -> Color {
        match self {
            MessageType::Error => Color::Red,
            MessageType::Warning => Color::Yellow,
            MessageType::Info => Color::Blue,
            MessageType::Success => Color::Green,
        }
    }
}

/// A status bar message with metadata.
#[derive(Debug, Clone)]
struct StatusMessage {
    text: String,
    msg_type: MessageType,
    shown_at: Instant,
}

impl StatusMessage {
    /// Create a new status message.
    fn new(text: impl Into<String>, msg_type: MessageType) -> Self {
        Self {
            text: text.into(),
            msg_type,
            shown_at: Instant::now(),
        }
    }

    /// Create an error message.
    fn error(text: impl Into<String>) -> Self {
        Self::new(text, MessageType::Error)
    }

    /// Create a warning message.
    fn warning(text: impl Into<String>) -> Self {
        Self::new(text, MessageType::Warning)
    }

    /// Create an info message.
    fn info(text: impl Into<String>) -> Self {
        Self::new(text, MessageType::Info)
    }

    /// Create a success message.
    fn success(text: impl Into<String>) -> Self {
        Self::new(text, MessageType::Success)
    }

    /// Check if this message has expired (older than 3 seconds).
    fn is_expired(&self) -> bool {
        self.shown_at.elapsed() > Duration::from_secs(3)
    }
}

/// Which pane currently has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePane {
    RepoList,
    Detail,
    Help,
    CommandPicker,
    CommandOutput,
}

/// Events from async command execution.
#[derive(Debug)]
enum CommandEvent {
    OutputLine(String),
    Completed(i32),
    Failed(String),
}

/// Main TUI application state.
pub struct App<R, D> {
    registry: R,
    detail_provider: D,
    list_state: ListState,
    should_quit: bool,
    active_pane: ActivePane,
    detail_scroll: usize,
    cached_detail: Option<RepoDetail>,
    cached_detail_index: Option<usize>,
    workspace_name: String,
    status_message: Option<StatusMessage>,
    needs_refresh: bool,

    // Command execution state
    command_picker_state: ListState,
    available_commands: Vec<(String, Command)>,
    selected_repo_for_commands: Option<String>,
    output_lines: Vec<String>,
    output_scroll: usize,
    output_bytes: usize,
    output_truncated: bool,
    command_state: CommandState,
    command_name: Option<String>,
    graft_loader: GraftYamlConfigLoader,
    command_event_rx: Option<Receiver<CommandEvent>>,
}

impl<R: RepoRegistry, D: RepoDetailProvider> App<R, D> {
    fn new(registry: R, detail_provider: D, workspace_name: String) -> Self {
        let mut list_state = ListState::default();

        // Only select first item if repos exist
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
            output_lines: Vec::new(),
            output_scroll: 0,
            output_bytes: 0,
            output_truncated: false,
            command_state: CommandState::NotStarted,
            command_name: None,
            graft_loader: GraftYamlConfigLoader::new(),
            command_event_rx: None,
        }
    }

    /// Perform status refresh if needed
    fn handle_refresh_if_needed(&mut self) {
        if self.needs_refresh {
            // Refresh all repositories
            match self.registry.refresh_all() {
                Ok(stats) => {
                    // Show success message with stats (auto-clears after 3 seconds)
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
                    self.status_message = Some(StatusMessage::error(format!("Refresh failed: {}", e)));
                }
            }

            // Clear cached detail to force re-query on next selection
            self.cached_detail = None;
            self.cached_detail_index = None;

            // Clear refresh flag
            self.needs_refresh = false;
        }
    }

    /// Clear expired status messages (older than 3 seconds)
    fn clear_expired_status_message(&mut self) {
        if let Some(msg) = &self.status_message {
            if msg.is_expired() {
                self.status_message = None;
            }
        }
    }

    fn handle_key(&mut self, code: KeyCode) {
        match self.active_pane {
            ActivePane::RepoList => self.handle_key_repo_list(code),
            ActivePane::Detail => self.handle_key_detail(code),
            ActivePane::Help => self.handle_key_help(code),
            ActivePane::CommandPicker => self.handle_key_command_picker(code),
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
                self.active_pane = ActivePane::Detail;
            }
            KeyCode::Char('r') => {
                // Manual refresh - set flag to trigger refresh in event loop
                self.needs_refresh = true;
                self.status_message = Some(StatusMessage::info("Refreshing..."));
            }
            KeyCode::Char('?') => {
                self.active_pane = ActivePane::Help;
            }
            KeyCode::Char('x') => {
                // Load commands for selected repo
                self.load_commands_for_selected_repo();
                if self.available_commands.is_empty() {
                    self.status_message = Some(StatusMessage::warning("No commands defined in graft.yaml"));
                } else {
                    self.active_pane = ActivePane::CommandPicker;
                    self.status_message = None;
                }
            }
            _ => {}
        }
    }

    fn handle_key_detail(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter | KeyCode::Tab => {
                self.active_pane = ActivePane::RepoList;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.detail_scroll = self.detail_scroll.saturating_add(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.detail_scroll = self.detail_scroll.saturating_sub(1);
            }
            _ => {}
        }
    }

    fn handle_key_help(&mut self, code: KeyCode) {
        // Printable keys and standard navigation dismiss help overlay
        // Control keys (Ctrl+C, Ctrl+Z, etc.) are ignored to avoid accidental dismissal
        match code {
            KeyCode::Char(_) | KeyCode::Esc | KeyCode::Enter | KeyCode::Backspace => {
                self.active_pane = ActivePane::RepoList;
            }
            _ => {} // Ignore control keys and other special keys
        }
    }

    fn handle_key_command_picker(&mut self, code: KeyCode) {
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
                // Execute selected command
                self.execute_selected_command();
            }
            KeyCode::Char('q') | KeyCode::Esc => {
                // Close picker
                self.active_pane = ActivePane::RepoList;
                self.available_commands.clear();
                self.selected_repo_for_commands = None;
            }
            _ => {}
        }
    }

    fn handle_key_command_output(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('j') | KeyCode::Down => {
                // Scroll down
                if self.output_scroll + 1 < self.output_lines.len() {
                    self.output_scroll += 1;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                // Scroll up
                if self.output_scroll > 0 {
                    self.output_scroll -= 1;
                }
            }
            KeyCode::Char('q') => {
                // Close output pane
                self.active_pane = ActivePane::RepoList;
                self.output_lines.clear();
                self.output_scroll = 0;
                self.output_bytes = 0;
                self.command_state = CommandState::NotStarted;
                self.command_name = None;
                self.output_truncated = false;
                self.command_event_rx = None;
            }
            _ => {}
        }
    }

    fn next(&mut self) {
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
    }

    fn previous(&mut self) {
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
    }

    /// Load detail for the currently selected repo if not already cached.
    fn ensure_detail_loaded(&mut self) {
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

    /// Load commands for the currently selected repository.
    fn load_commands_for_selected_repo(&mut self) {
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
        let graft_path = format!("{}/graft.yaml", repo_path);
        let graft_config = match self.graft_loader.load_graft(&graft_path) {
            Ok(config) => config,
            Err(e) => {
                self.status_message = Some(StatusMessage::error(format!("Error loading graft.yaml: {e}")));
                return;
            }
        };

        // Populate commands list
        self.available_commands = graft_config.commands.into_iter().collect();
        self.available_commands.sort_by(|a, b| a.0.cmp(&b.0)); // Sort by name
        self.selected_repo_for_commands = Some(repo_path);

        // Select first command if any exist
        if !self.available_commands.is_empty() {
            self.command_picker_state.select(Some(0));
        }
    }

    /// Handle incoming command output events.
    fn handle_command_events(&mut self) {
        let mut should_close = false;

        if let Some(rx) = &self.command_event_rx {
            while let Ok(event) = rx.try_recv() {
                match event {
                    CommandEvent::OutputLine(line) => {
                        let line_bytes = line.len();
                        if self.output_bytes + line_bytes > MAX_OUTPUT_BYTES {
                            self.output_truncated = true;
                            // Stop accepting more output
                            should_close = true;
                        } else {
                            self.output_bytes += line_bytes;
                            self.output_lines.push(line);
                        }
                    }
                    CommandEvent::Completed(exit_code) => {
                        self.command_state = CommandState::Completed { exit_code };
                        should_close = true;
                    }
                    CommandEvent::Failed(error) => {
                        self.command_state = CommandState::Failed { error };
                        should_close = true;
                    }
                }
            }
        }

        if should_close {
            self.command_event_rx = None;
        }
    }

    /// Execute the currently selected command.
    fn execute_selected_command(&mut self) {
        let Some(cmd_idx) = self.command_picker_state.selected() else {
            return;
        };

        if cmd_idx >= self.available_commands.len() {
            return;
        }

        let (cmd_name, _cmd) = &self.available_commands[cmd_idx];
        let Some(repo_path) = &self.selected_repo_for_commands else {
            return;
        };

        // Switch to output pane
        self.active_pane = ActivePane::CommandOutput;
        self.output_lines.clear();
        self.output_scroll = 0;
        self.output_bytes = 0;
        self.output_truncated = false;
        self.command_state = CommandState::Running;
        self.command_name = Some(cmd_name.clone());

        // Create channel for command output
        let (tx, rx) = mpsc::channel();
        self.command_event_rx = Some(rx);

        // Spawn command in background thread
        let cmd_name_clone = cmd_name.clone();
        let repo_path_clone = repo_path.clone();

        std::thread::spawn(move || {
            spawn_command(cmd_name_clone, repo_path_clone, tx);
        });
    }

    fn render(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
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
                    Constraint::Min(3),      // Content area (flexible)
                    Constraint::Length(1),   // Status bar (1 line)
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
            let repos = self.registry.list_repos();
            let pane_width = chunks[0].width;

            let list_border_color = if self.active_pane == ActivePane::RepoList {
                Color::Cyan
            } else {
                Color::DarkGray
            };

            // Build title with workspace name
            let title = format!("Grove: {} (↑↓/jk navigate, x:commands, ?:help)", self.workspace_name);

            // Handle empty workspace case
            if repos.is_empty() {
                let empty_message = vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "No repositories configured",
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Edit your workspace config to add repositories:",
                        Style::default().fg(Color::Gray),
                    )),
                    Line::from(Span::styled(
                        "  ~/.config/grove/workspace.yaml",
                        Style::default().fg(Color::Cyan),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Example:",
                        Style::default().fg(Color::Gray),
                    )),
                    Line::from(Span::styled(
                        "  repositories:",
                        Style::default().fg(Color::DarkGray),
                    )),
                    Line::from(Span::styled(
                        "    - path: ~/src/my-project",
                        Style::default().fg(Color::DarkGray),
                    )),
                ];

                let empty_widget = Paragraph::new(empty_message)
                    .block(
                        Block::default()
                            .title(title)
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(list_border_color)),
                    )
                    .alignment(Alignment::Center);

                frame.render_widget(empty_widget, chunks[0]);
            } else {
                // Normal repo list display
                let items: Vec<ListItem> = repos
                    .iter()
                    .map(|repo_path| {
                        let status = self.registry.get_status(repo_path);
                        let line = format_repo_line(
                            repo_path.as_path().display().to_string(),
                            status,
                            pane_width,
                        );
                        ListItem::new(line)
                    })
                    .collect();

                let list = List::new(items)
                    .block(
                        Block::default()
                            .title(title)
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(list_border_color)),
                    )
                    .highlight_style(
                        Style::default()
                            .bg(Color::Rgb(40, 40, 50))
                            .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol("▶ ");

                frame.render_stateful_widget(list, chunks[0], &mut self.list_state);
            }

            // --- Right pane: detail ---
            let detail_border_color = if self.active_pane == ActivePane::Detail {
                Color::Cyan
            } else {
                Color::DarkGray
            };

            let detail_lines = self.build_detail_lines();

            // Clamp scroll to content height (account for border: 2 lines top+bottom)
            let inner_height = chunks[1].height.saturating_sub(2) as usize;
            let max_scroll = detail_lines.len().saturating_sub(inner_height);
            self.detail_scroll = self.detail_scroll.min(max_scroll);

            let detail_widget = Paragraph::new(detail_lines)
                .block(
                    Block::default()
                        .title("Detail (j/k scroll, q/Esc back)")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(detail_border_color)),
                )
                .scroll((u16::try_from(self.detail_scroll).unwrap_or(u16::MAX), 0));

            frame.render_widget(detail_widget, chunks[1]);

            // --- Help overlay (rendered on top if active) ---
            if self.active_pane == ActivePane::Help {
                self.render_help_overlay(frame);
            }

            // --- Command picker overlay ---
            if self.active_pane == ActivePane::CommandPicker {
                self.render_command_picker_overlay(frame);
            }

            // --- Command output overlay ---
            if self.active_pane == ActivePane::CommandOutput {
                self.render_command_output_overlay(frame);
            }

            // --- Status bar (always rendered at bottom) ---
            self.render_status_bar(frame, status_bar_area);
        })?;

        Ok(())
    }

    /// Render the help overlay as a centered popup
    fn render_help_overlay(&self, frame: &mut ratatui::Frame) {
        let area = frame.area();

        // Validate terminal size - need minimum space for help
        if area.width < 44 || area.height < 20 {
            // Terminal too small, show simplified message
            let msg = "Terminal too small for help. Resize or press any key.";
            let warning = Paragraph::new(msg)
                .alignment(Alignment::Center)
                .block(
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
                format!("Grove v{} - Help", version),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled("Navigation", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
            Line::from("  j, ↓         Move selection down"),
            Line::from("  k, ↑         Move selection up"),
            Line::from("  Enter, Tab   View repository details"),
            Line::from("  q, Esc       Quit (or return from detail pane)"),
            Line::from(""),
            Line::from(Span::styled("Actions", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
            Line::from("  r            Refresh repository status"),
            Line::from("  x            Execute command (from graft.yaml)"),
            Line::from("  ?            Show this help"),
            Line::from(""),
            Line::from(Span::styled("Detail Pane", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
            Line::from("  j, ↓         Scroll down"),
            Line::from("  k, ↑         Scroll up"),
            Line::from("  q, Esc       Return to repository list"),
            Line::from(""),
            Line::from(Span::styled("Status Indicators", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
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
            Line::from(Span::styled("Status Bar", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
            Line::from("  The bottom line shows status messages:"),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("✗", Style::default().fg(Color::Red)),
                Span::raw(" Red    - Errors"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("⚠", Style::default().fg(Color::Yellow)),
                Span::raw(" Yellow - Warnings"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("ℹ", Style::default().fg(Color::Blue)),
                Span::raw(" Blue   - Information"),
            ]),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("✓", Style::default().fg(Color::Green)),
                Span::raw(" Green  - Success"),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Press any key to close",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        // Calculate popup size and position (centered)
        // Ensure minimum viable size
        let popup_width = 60.min(area.width.saturating_sub(4)).max(40);
        let popup_height = (help_text.len() as u16 + 2).min(area.height.saturating_sub(4)).max(20);

        let popup_x = (area.width.saturating_sub(popup_width)) / 2;
        let popup_y = (area.height.saturating_sub(popup_height)) / 2;

        let popup_area = ratatui::layout::Rect {
            x: popup_x,
            y: popup_y,
            width: popup_width,
            height: popup_height,
        };

        // Clear the area behind the popup first
        frame.render_widget(Clear, popup_area);

        // Render help content with solid background
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

    /// Render the command picker overlay as a centered popup.
    fn render_command_picker_overlay(&mut self, frame: &mut ratatui::Frame) {
        let area = centered_rect(70, 80, frame.area());

        // Clear background
        frame.render_widget(Clear, area);

        // Create bordered block
        let block = Block::default()
            .title(" Commands (↑↓/jk: navigate, Enter: execute, q: close) ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::Black));

        // Build list items
        let items: Vec<ListItem> = self
            .available_commands
            .iter()
            .map(|(name, cmd)| {
                let desc = cmd.description.as_deref().unwrap_or("");
                let content = format!("{:<20} {}", name, desc);
                ListItem::new(content)
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().bg(Color::Rgb(40, 40, 50)).add_modifier(Modifier::BOLD))
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(list, area, &mut self.command_picker_state);
    }

    /// Render the command output overlay.
    fn render_command_output_overlay(&mut self, frame: &mut ratatui::Frame) {
        let area = frame.area();

        // Build header based on state
        let header = match &self.command_state {
            CommandState::Running => format!(
                " Running: {} (j/k: scroll, q: close) ",
                self.command_name.as_deref().unwrap_or("unknown")
            ),
            CommandState::Completed { exit_code } => {
                if *exit_code == 0 {
                    format!(
                        " ✓ {}: Completed successfully (exit {}) - Press q to close ",
                        self.command_name.as_deref().unwrap_or("unknown"),
                        exit_code
                    )
                } else {
                    format!(
                        " ✗ {}: Failed with exit code {} - Press q to close ",
                        self.command_name.as_deref().unwrap_or("unknown"),
                        exit_code
                    )
                }
            }
            CommandState::Failed { error } => {
                format!(" ✗ Failed: {} - Press q to close ", error)
            }
            CommandState::NotStarted => " Output ".to_string(),
        };

        let block = Block::default()
            .title(header)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::Black));

        // Get visible lines based on scroll position
        let inner = block.inner(area);
        let visible_height = inner.height as usize;
        let start = self.output_scroll;
        let end = (start + visible_height).min(self.output_lines.len());
        let visible_lines: Vec<Line> = self.output_lines[start..end]
            .iter()
            .map(|line| Line::from(line.clone()))
            .collect();

        // Clamp scroll
        let max_scroll = self.output_lines.len().saturating_sub(visible_height);
        self.output_scroll = self.output_scroll.min(max_scroll);

        let paragraph = Paragraph::new(visible_lines)
            .block(block)
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);

        // Show truncation warning if needed
        if self.output_truncated {
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

    /// Render the status bar at the bottom of the screen.
    fn render_status_bar(&self, frame: &mut ratatui::Frame, area: Rect) {
        let unicode = supports_unicode();

        let (mut text, fg_color, bg_color) = if let Some(msg) = &self.status_message {
            let symbol = msg.msg_type.symbol(unicode);
            let fg = msg.msg_type.fg_color();
            let bg = msg.msg_type.bg_color();
            (format!(" {} {}", symbol, msg.text), fg, bg)
        } else {
            // Default status when no message
            (
                " Ready • Press ? for help".to_string(),
                Color::White,
                Color::DarkGray,
            )
        };

        // Truncate with ellipsis if message is too long
        let max_width = area.width as usize;
        if text.width() > max_width {
            // Need to account for ellipsis width (3 characters)
            let target_width = max_width.saturating_sub(3);

            // Truncate to target width
            let mut truncated = String::new();
            let mut current_width = 0;

            for ch in text.chars() {
                let ch_width = UnicodeWidthStr::width(ch.to_string().as_str());
                if current_width + ch_width > target_width {
                    break;
                }
                truncated.push(ch);
                current_width += ch_width;
            }

            truncated.push_str("...");
            text = truncated;
        }

        let status_bar = Paragraph::new(text)
            .style(Style::default().fg(fg_color).bg(bg_color));

        frame.render_widget(status_bar, area);
    }

    /// Build the lines for the detail pane based on cached detail.
    fn build_detail_lines(&self) -> Vec<Line<'static>> {
        let Some(detail) = &self.cached_detail else {
            return vec![Line::from(Span::styled(
                "No repository selected",
                Style::default().fg(Color::Gray),
            ))];
        };

        let mut lines: Vec<Line<'static>> = Vec::new();

        // Show error as warning if present (but continue rendering partial data)
        if let Some(error) = &detail.error {
            lines.push(Line::from(Span::styled(
                format!("Error: {error}"),
                Style::default().fg(Color::Red),
            )));
            lines.push(Line::from(""));
        }

        // Branch/status header from registry
        if let Some(index) = self.cached_detail_index {
            let repos = self.registry.list_repos();
            if let Some(repo_path) = repos.get(index) {
                if let Some(status) = self.registry.get_status(repo_path) {
                    let mut header_spans = Vec::new();

                    let branch = status
                        .branch
                        .as_ref()
                        .map_or_else(|| "[detached]".to_string(), Clone::clone);
                    header_spans.push(Span::styled(branch, Style::default().fg(Color::Cyan)));

                    let dirty_indicator = if status.is_dirty { " ●" } else { " ○" };
                    let dirty_color = if status.is_dirty {
                        Color::Yellow
                    } else {
                        Color::Green
                    };
                    header_spans.push(Span::styled(
                        dirty_indicator,
                        Style::default().fg(dirty_color),
                    ));

                    if let Some(ahead) = status.ahead.filter(|&n| n > 0) {
                        header_spans.push(Span::styled(
                            format!(" ↑{ahead}"),
                            Style::default().fg(Color::Green),
                        ));
                    }
                    if let Some(behind) = status.behind.filter(|&n| n > 0) {
                        header_spans.push(Span::styled(
                            format!(" ↓{behind}"),
                            Style::default().fg(Color::Red),
                        ));
                    }

                    lines.push(Line::from(header_spans));
                    lines.push(Line::from(""));
                }
            }
        }

        // Changed files section
        if detail.changed_files.is_empty() {
            lines.push(Line::from(Span::styled(
                "No uncommitted changes",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                format!("Changed Files ({})", detail.changed_files.len()),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));

            for change in &detail.changed_files {
                let (indicator, color) = format_file_change_indicator(&change.status);
                lines.push(Line::from(vec![
                    Span::styled(format!("  {indicator} "), Style::default().fg(color)),
                    Span::styled(change.path.clone(), Style::default().fg(Color::White)),
                ]));
            }
        }

        // Separator
        lines.push(Line::from(""));

        // Commits section
        if detail.commits.is_empty() {
            lines.push(Line::from(Span::styled(
                "No commits",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                format!("Recent Commits ({})", detail.commits.len()),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )));

            for commit in &detail.commits {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {} ", commit.hash),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::styled(commit.subject.clone(), Style::default().fg(Color::White)),
                ]));
                lines.push(Line::from(Span::styled(
                    format!("       {} - {}", commit.author, commit.relative_date),
                    Style::default().fg(Color::DarkGray),
                )));
            }
        }

        lines
    }
}

/// Extract the basename (final component) from a path.
///
/// # Examples
/// - `/home/user/src/graft` → `graft`
/// - `~/projects/repo` → `repo`
/// - `/tmp` → `tmp`
fn extract_basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Compact a path to fit within a maximum width using abbreviation strategies.
///
/// Applies transformations in order:
/// 1. Home directory shown as "~" (e.g., `/home/user` → `~`)
/// 2. Parent directory components abbreviated to first character (preserves last 2 components)
/// 3. Fallback to prefix truncation with "[..]" if still too wide
///
/// # Examples
/// - `/home/user/projects/graft` → `~/projects/graft` (if width allows)
/// - `/home/user/very/long/nested/project-name` → `~/v/l/n/project-name` (fish-style)
/// - Very long path that exceeds max_width → `[..]project-name`
fn compact_path(path: &str, max_width: usize) -> String {
    // First, collapse home directory to tilde
    let tilde_path = if let Ok(home) = std::env::var("HOME") {
        if path.starts_with(&home) {
            path.replacen(&home, "~", 1)
        } else {
            path.to_string()
        }
    } else {
        path.to_string()
    };

    let current_width = tilde_path.width();

    // If it fits already, we're done
    if current_width <= max_width {
        return tilde_path.to_string();
    }

    // Split into components
    let parts: Vec<&str> = tilde_path.split('/').collect();

    // If we have fewer than 3 components, just truncate with prefix
    if parts.len() < 3 {
        return prefix_truncate(&tilde_path, max_width);
    }

    // Fish-style abbreviation: abbreviate all but last 2 components
    let preserve_count = 2;
    let mut abbreviated = String::new();

    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            abbreviated.push('/');
        }

        // Preserve last N components and empty parts (for leading /)
        if i >= parts.len() - preserve_count || part.is_empty() {
            abbreviated.push_str(part);
        } else {
            // Abbreviate to first character (or empty if component is empty)
            if let Some(first_char) = part.chars().next() {
                abbreviated.push(first_char);
            }
        }
    }

    // Check if abbreviated version fits
    if abbreviated.width() <= max_width {
        return abbreviated;
    }

    // Last resort: prefix truncation
    prefix_truncate(&abbreviated, max_width)
}

/// Truncate a string from the start with "[..]" prefix.
fn prefix_truncate(s: &str, max_width: usize) -> String {
    const PREFIX: &str = "[..]";
    const PREFIX_WIDTH: usize = 4; // "[..]".width()

    if s.width() <= max_width {
        return s.to_string();
    }

    if max_width <= PREFIX_WIDTH {
        // Not enough room for prefix, just take what we can
        return s.chars().take(max_width).collect();
    }

    let target_width = max_width - PREFIX_WIDTH;
    let mut truncated = String::from(PREFIX);
    let mut current_width = 0;

    // Take characters from the end that fit within target_width
    for ch in s.chars().rev() {
        let ch_width = UnicodeWidthStr::width(ch.to_string().as_str());
        if current_width + ch_width > target_width {
            break;
        }
        current_width += ch_width;
        truncated.insert(PREFIX_WIDTH, ch);
    }

    truncated
}

/// Map a `FileChangeStatus` to an indicator character and color.
fn format_file_change_indicator(status: &FileChangeStatus) -> (&'static str, Color) {
    match status {
        FileChangeStatus::Modified => ("M", Color::Yellow),
        FileChangeStatus::Added => ("A", Color::Green),
        FileChangeStatus::Deleted => ("D", Color::Red),
        FileChangeStatus::Renamed => ("R", Color::Cyan),
        FileChangeStatus::Copied => ("C", Color::Cyan),
        FileChangeStatus::Unknown => ("?", Color::Gray),
    }
}

/// Format a repository status line for display in the TUI.
///
/// Returns `Line<'static>` because all data is owned (no borrowing from input parameters).
/// The 'static lifetime indicates the Line owns its data, not that it's statically allocated.
///
/// # Arguments
/// - `path`: Full path to the repository
/// - `status`: Optional repository status information
/// - `pane_width`: Available width for the pane (used to compact long paths)
fn format_repo_line(path: String, status: Option<&RepoStatus>, pane_width: u16) -> Line<'static> {
    match status {
        Some(status) => {
            // Check for error first - safe pattern matching without unwrap
            if let Some(error_msg) = &status.error {
                // For errors, calculate error message width and compact path accordingly
                let error_text = format!("[error: {error_msg}]");
                // Overhead components:
                //   2: List widget highlight symbol "▶ " (added by ratatui when selected)
                //   1: Space after path
                //   error_text.width(): The error message itself
                //   3: Safety margin (prevents text from touching right border, List padding)
                let overhead = 2 + 1 + error_text.width() + 3;
                let max_path_width = (pane_width as usize).saturating_sub(overhead);
                let compacted_path = compact_path(&path, max_path_width);

                let error_color = if error_msg.contains("timed out") {
                    Color::Yellow
                } else {
                    Color::Red
                };

                Line::from(vec![
                    Span::styled(compacted_path, Style::default().fg(Color::White)),
                    Span::raw(" "),
                    Span::styled(error_text, Style::default().fg(error_color)),
                ])
            } else {
                // Build status indicators first to calculate their width
                let branch = status
                    .branch
                    .as_ref()
                    .map_or_else(|| "[detached]".to_string(), |b| format!("[{b}]"));

                let dirty_indicator = if status.is_dirty { "●" } else { "○" };
                let dirty_color = if status.is_dirty {
                    Color::Yellow
                } else {
                    Color::Green
                };

                let ahead_text = status
                    .ahead
                    .filter(|&n| n > 0)
                    .map(|n| format!("↑{n}"))
                    .unwrap_or_default();

                let behind_text = status
                    .behind
                    .filter(|&n| n > 0)
                    .map(|n| format!("↓{n}"))
                    .unwrap_or_default();

                // Calculate status width WITHOUT branch (for tight space fallback)
                let mut minimal_status_width = 1 + 1; // space + dirty
                if !ahead_text.is_empty() {
                    minimal_status_width += 1 + ahead_text.width(); // space + ahead
                }
                if !behind_text.is_empty() {
                    minimal_status_width += 1 + behind_text.width(); // space + behind
                }

                // Tiered display strategy based on available width:
                // - Very tight (< 15): basename only
                // - Tight (15-25): compacted path without branch
                // - Normal (>= 25): try to show with branch

                if pane_width < 15 {
                    // Very tight: show just basename
                    // Format: basename ● [↑n] [↓n]
                    let basename = extract_basename(&path);
                    // Overhead: highlight (2) + minimal_status_width + safety margin (3)
                    let overhead = 2 + minimal_status_width + 3;
                    let max_basename_width = (pane_width as usize).saturating_sub(overhead);

                    // Truncate basename if needed (shouldn't happen often)
                    let display_name = if basename.width() > max_basename_width {
                        &basename[..max_basename_width.min(basename.len())]
                    } else {
                        basename
                    };

                    let mut spans = vec![
                        Span::styled(display_name.to_string(), Style::default().fg(Color::White)),
                        Span::raw(" "),
                        Span::styled(dirty_indicator, Style::default().fg(dirty_color)),
                    ];

                    if !ahead_text.is_empty() {
                        spans.push(Span::raw(" "));
                        spans.push(Span::styled(ahead_text, Style::default().fg(Color::Green)));
                    }

                    if !behind_text.is_empty() {
                        spans.push(Span::raw(" "));
                        spans.push(Span::styled(behind_text, Style::default().fg(Color::Red)));
                    }

                    Line::from(spans)
                } else {
                    // Calculate status width WITH branch
                    let full_status_width = 1 + branch.width() + minimal_status_width;

                    // Try with branch first
                    // Overhead: highlight (2) + full_status_width + safety margin (3)
                    let overhead_with_branch = 2 + full_status_width + 3;
                    let max_path_width_with_branch = (pane_width as usize).saturating_sub(overhead_with_branch);
                    let compacted_path_with_branch = compact_path(&path, max_path_width_with_branch);

                    // If path is severely compacted (uses [..] prefix or display width < 8), drop branch
                    let use_branch = !compacted_path_with_branch.starts_with("[..]")
                        && compacted_path_with_branch.width() >= 8;

                    if use_branch {
                        // Normal: show with branch
                        // Format: path [branch] ● [↑n] [↓n]
                        let mut spans = vec![
                            Span::styled(compacted_path_with_branch, Style::default().fg(Color::White)),
                            Span::raw(" "),
                            Span::styled(branch, Style::default().fg(Color::Cyan)),
                            Span::raw(" "),
                            Span::styled(dirty_indicator, Style::default().fg(dirty_color)),
                        ];

                        if !ahead_text.is_empty() {
                            spans.push(Span::raw(" "));
                            spans.push(Span::styled(ahead_text, Style::default().fg(Color::Green)));
                        }

                        if !behind_text.is_empty() {
                            spans.push(Span::raw(" "));
                            spans.push(Span::styled(behind_text, Style::default().fg(Color::Red)));
                        }

                        Line::from(spans)
                    } else {
                        // Tight: drop branch, show more of path
                        // Format: path ● [↑n] [↓n]
                        // Overhead: highlight (2) + minimal_status_width + safety margin (3)
                        let overhead_without_branch = 2 + minimal_status_width + 3;
                        let max_path_width = (pane_width as usize).saturating_sub(overhead_without_branch);
                        let compacted_path = compact_path(&path, max_path_width);

                        let mut spans = vec![
                            Span::styled(compacted_path, Style::default().fg(Color::White)),
                            Span::raw(" "),
                            Span::styled(dirty_indicator, Style::default().fg(dirty_color)),
                        ];

                        if !ahead_text.is_empty() {
                            spans.push(Span::raw(" "));
                            spans.push(Span::styled(ahead_text, Style::default().fg(Color::Green)));
                        }

                        if !behind_text.is_empty() {
                            spans.push(Span::raw(" "));
                            spans.push(Span::styled(behind_text, Style::default().fg(Color::Red)));
                        }

                        Line::from(spans)
                    }
                }
            }
        }
        None => {
            // For loading state, calculate overhead and compact path
            let loading_text = "[loading...]";
            // Overhead: highlight (2) + space (1) + loading text + safety margin (3)
            let overhead = 2 + 1 + loading_text.width() + 3;
            let max_path_width = (pane_width as usize).saturating_sub(overhead);
            let compacted_path = compact_path(&path, max_path_width);

            Line::from(vec![
                Span::styled(compacted_path, Style::default().fg(Color::White)),
                Span::raw(" "),
                Span::styled(loading_text, Style::default().fg(Color::Gray)),
            ])
        }
    }
}

/// Helper to create a centered rectangle.
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Spawn a graft command in the background and send output via channel.
fn spawn_command(command_name: String, repo_path: String, tx: Sender<CommandEvent>) {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};

    // Spawn graft run command
    let result = Command::new("graft")
        .arg("run")
        .arg(&command_name)
        .current_dir(&repo_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    let mut child = match result {
        Ok(child) => child,
        Err(e) => {
            let _ = tx.send(CommandEvent::Failed(format!("Failed to spawn graft: {e}")));
            return;
        }
    };

    // Capture stdout in a thread
    let stdout = child.stdout.take();
    let tx_stdout = tx.clone();
    let stdout_thread = stdout.map(|out| {
        std::thread::spawn(move || {
            let reader = BufReader::new(out);
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        if tx_stdout.send(CommandEvent::OutputLine(line)).is_err() {
                            break; // Channel closed
                        }
                    }
                    Err(e) => {
                        let _ = tx_stdout.send(CommandEvent::Failed(format!("Read error: {e}")));
                        break;
                    }
                }
            }
        })
    });

    // Capture stderr in a thread
    let stderr = child.stderr.take();
    let tx_stderr = tx.clone();
    let stderr_thread = stderr.map(|err| {
        std::thread::spawn(move || {
            let reader = BufReader::new(err);
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        if tx_stderr.send(CommandEvent::OutputLine(line)).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = tx_stderr.send(CommandEvent::Failed(format!("Read error: {e}")));
                        break;
                    }
                }
            }
        })
    });

    // Wait for child to complete
    match child.wait() {
        Ok(status) => {
            // Wait for output threads to finish
            if let Some(thread) = stdout_thread {
                let _ = thread.join();
            }
            if let Some(thread) = stderr_thread {
                let _ = thread.join();
            }

            let exit_code = status.code().unwrap_or(-1);
            let _ = tx.send(CommandEvent::Completed(exit_code));
        }
        Err(e) => {
            let _ = tx.send(CommandEvent::Failed(format!("Wait failed: {e}")));
        }
    }
}

pub fn run<R: RepoRegistry, D: RepoDetailProvider>(
    registry: R,
    detail_provider: D,
    workspace_name: String,
) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new(registry, detail_provider, workspace_name);

    // Main event loop
    let result = loop {
        // If refresh requested, render "Refreshing..." first, then refresh
        if app.needs_refresh {
            app.render(&mut terminal)?;  // Show "Refreshing..." message
            app.handle_refresh_if_needed();  // Do the refresh
        }

        app.render(&mut terminal)?;

        if app.should_quit {
            break Ok(());
        }

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    app.handle_key(key.code);
                }
            }
        }
    };

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

#[cfg(test)]
#[path = "tui_tests.rs"]
mod tests;
