//! Terminal UI implementation using ratatui.

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use grove_core::{RepoRegistry, RepoStatus};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Terminal,
};
use std::io;

/// Main TUI application state.
pub struct App<R> {
    registry: R,
    list_state: ListState,
    should_quit: bool,
}

impl<R: RepoRegistry> App<R> {
    fn new(registry: R) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            registry,
            list_state,
            should_quit: false,
        }
    }

    fn handle_key(&mut self, code: KeyCode) {
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

    fn render(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
        terminal.draw(|frame| {
            let chunks = Layout::default()
                .constraints([Constraint::Min(0)])
                .split(frame.area());

            let repos = self.registry.list_repos();
            let items: Vec<ListItem> = repos
                .iter()
                .map(|repo_path| {
                    let status = self.registry.get_status(repo_path);
                    let line = format_repo_line(repo_path.as_path().display().to_string(), status);
                    ListItem::new(line)
                })
                .collect();

            let list = List::new(items)
                .block(
                    Block::default()
                        .title("Grove - Repository Status (j/k to navigate, q to quit)")
                        .borders(Borders::ALL),
                )
                .highlight_style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("▶ ");

            frame.render_stateful_widget(list, chunks[0], &mut self.list_state);
        })?;

        Ok(())
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

pub fn run<R: RepoRegistry>(registry: R) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new(registry);

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
