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
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};
use std::io;

/// Default number of recent commits to show in the detail pane.
const DEFAULT_MAX_COMMITS: usize = 10;

/// Which pane currently has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePane {
    RepoList,
    Detail,
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
}

impl<R: RepoRegistry, D: RepoDetailProvider> App<R, D> {
    fn new(registry: R, detail_provider: D) -> Self {
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
        }
    }

    fn handle_key(&mut self, code: KeyCode) {
        match self.active_pane {
            ActivePane::RepoList => self.handle_key_repo_list(code),
            ActivePane::Detail => self.handle_key_detail(code),
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
            let items: Vec<ListItem> = repos
                .iter()
                .map(|repo_path| {
                    let status = self.registry.get_status(repo_path);
                    let line = format_repo_line(repo_path.as_path().display().to_string(), status);
                    ListItem::new(line)
                })
                .collect();

            let list_border_color = if self.active_pane == ActivePane::RepoList {
                Color::Cyan
            } else {
                Color::DarkGray
            };

            let list = List::new(items)
                .block(
                    Block::default()
                        .title("Repositories (j/k navigate, Enter/Tab detail)")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(list_border_color)),
                )
                .highlight_style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("▶ ");

            frame.render_stateful_widget(list, chunks[0], &mut self.list_state);

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
        })?;

        Ok(())
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
fn format_repo_line(path: String, status: Option<&RepoStatus>) -> Line<'static> {
    match status {
        Some(status) => {
            // Check for error first - safe pattern matching without unwrap
            if let Some(error_msg) = &status.error {
                // Use yellow for timeout errors, red for other errors
                let error_color = if error_msg.contains("timed out") {
                    Color::Yellow
                } else {
                    Color::Red
                };

                Line::from(vec![
                    Span::styled(path, Style::default().fg(Color::White)),
                    Span::raw(" "),
                    Span::styled(
                        format!("[error: {error_msg}]"),
                        Style::default().fg(error_color),
                    ),
                ])
            } else {
                // Normal status rendering
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

                let mut spans = vec![
                    Span::styled(path, Style::default().fg(Color::White)),
                    Span::raw(" "),
                    Span::styled(branch, Style::default().fg(Color::Cyan)),
                    Span::raw(" "),
                    Span::styled(dirty_indicator, Style::default().fg(dirty_color)),
                ];

                if let Some(ahead) = status.ahead {
                    if ahead > 0 {
                        spans.push(Span::raw(" "));
                        spans.push(Span::styled(
                            format!("↑{ahead}"),
                            Style::default().fg(Color::Green),
                        ));
                    }
                }

                if let Some(behind) = status.behind {
                    if behind > 0 {
                        spans.push(Span::raw(" "));
                        spans.push(Span::styled(
                            format!("↓{behind}"),
                            Style::default().fg(Color::Red),
                        ));
                    }
                }

                Line::from(spans)
            }
        }
        None => Line::from(vec![
            Span::styled(path, Style::default().fg(Color::White)),
            Span::raw(" "),
            Span::styled("[loading...]", Style::default().fg(Color::Gray)),
        ]),
    }
}

pub fn run<R: RepoRegistry, D: RepoDetailProvider>(registry: R, detail_provider: D) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new(registry, detail_provider);

    // Main event loop
    let result = loop {
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
