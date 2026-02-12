//! Terminal UI implementation using ratatui.

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use grove_core::{FileChangeStatus, RepoDetail, RepoDetailProvider, RepoRegistry, RepoStatus};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};
use std::io;
use unicode_width::UnicodeWidthStr;

/// Default number of recent commits to show in the detail pane.
const DEFAULT_MAX_COMMITS: usize = 10;

/// Which pane currently has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePane {
    RepoList,
    Detail,
    Help,
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
    status_message: Option<String>,
    needs_refresh: bool,
}

impl<R: RepoRegistry, D: RepoDetailProvider> App<R, D> {
    fn new(registry: R, detail_provider: D, workspace_name: String) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

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
        }
    }

    /// Perform status refresh if needed and clear the refresh flag
    fn handle_refresh_if_needed(&mut self) {
        if self.needs_refresh {
            // Refresh all repositories
            let _ = self.registry.refresh_all();

            // Clear cached detail to force re-query on next selection
            self.cached_detail = None;
            self.cached_detail_index = None;

            // Clear refresh flag and status message
            self.needs_refresh = false;
            self.status_message = None;
        }
    }

    fn handle_key(&mut self, code: KeyCode) {
        match self.active_pane {
            ActivePane::RepoList => self.handle_key_repo_list(code),
            ActivePane::Detail => self.handle_key_detail(code),
            ActivePane::Help => self.handle_key_help(code),
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
                self.status_message = Some("Refreshing...".to_string());
            }
            KeyCode::Char('?') => {
                self.active_pane = ActivePane::Help;
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
        // Any key dismisses help overlay
        match code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('?') => {
                self.active_pane = ActivePane::RepoList;
            }
            _ => {
                self.active_pane = ActivePane::RepoList;
            }
        }
    }

    fn next(&mut self) {
        let repos = self.registry.list_repos();
        if repos.is_empty() {
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

    fn render(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
        self.ensure_detail_loaded();

        terminal.draw(|frame| {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                .split(frame.area());

            // --- Left pane: repo list ---
            let repos = self.registry.list_repos();
            let pane_width = chunks[0].width;

            let list_border_color = if self.active_pane == ActivePane::RepoList {
                Color::Cyan
            } else {
                Color::DarkGray
            };

            // Build title with workspace name and optional status message
            let title = if let Some(msg) = &self.status_message {
                format!("Grove: {} - {}", self.workspace_name, msg)
            } else {
                format!("Grove: {} (↑↓/jk navigate, ?help)", self.workspace_name)
            };

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
                    .alignment(ratatui::layout::Alignment::Center);

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
        })?;

        Ok(())
    }

    /// Render the help overlay as a centered popup
    fn render_help_overlay(&self, frame: &mut ratatui::Frame) {
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
            Line::from(Span::styled(
                "Press any key to close",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        // Calculate popup size and position (centered)
        let area = frame.area();
        let popup_width = 60.min(area.width.saturating_sub(4));
        let popup_height = (help_text.len() as u16 + 2).min(area.height.saturating_sub(4));

        let popup_x = (area.width.saturating_sub(popup_width)) / 2;
        let popup_y = (area.height.saturating_sub(popup_height)) / 2;

        let popup_area = ratatui::layout::Rect {
            x: popup_x,
            y: popup_y,
            width: popup_width,
            height: popup_height,
        };

        // Clear background with a semi-transparent effect (render blank block first)
        let clear = Block::default()
            .style(Style::default().bg(Color::Black));
        frame.render_widget(clear, popup_area);

        // Render help content
        let help_widget = Paragraph::new(help_text)
            .block(
                Block::default()
                    .title("Help")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .alignment(Alignment::Left);

        frame.render_widget(help_widget, popup_area);
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
        // Handle refresh if requested (before render to show updated state)
        app.handle_refresh_if_needed();

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
