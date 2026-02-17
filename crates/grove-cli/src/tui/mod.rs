//! Terminal UI implementation using ratatui.

mod app;
mod command_exec;
mod hint_bar;
mod overlays;
mod render;
mod repo_list;
mod status_bar;
mod tab_changes;
mod tab_commands;
mod tab_state;
mod tabs;

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

// Re-export public items
pub use command_exec::CommandEvent;
pub use tabs::DetailTab;

// Re-export for integration tests
#[allow(unused_imports)]
pub use command_exec::spawn_command;
pub(crate) use repo_list::format_file_change_indicator;
#[cfg(test)]
pub(crate) use repo_list::{compact_path, extract_basename, format_repo_line};
#[cfg(test)]
pub(crate) use status_bar::MessageType;
pub(crate) use status_bar::StatusMessage;

/// Default number of recent commits to show in the detail pane.
const DEFAULT_MAX_COMMITS: usize = 10;

/// Maximum lines of command output to buffer (~1MB at 100 chars/line)
const MAX_OUTPUT_LINES: usize = 10_000;

/// Number of lines to drop when buffer is full (reduces churn)
const LINES_TO_DROP: usize = 1_000;

/// Check if the terminal supports Unicode characters.
///
/// Returns false for terminals known to have poor Unicode support.
fn supports_unicode() -> bool {
    std::env::var("TERM")
        .map(|term| !term.contains("linux") && !term.contains("ascii") && !term.contains("vt100"))
        .unwrap_or(true) // Default to Unicode support
}

/// Which pane currently has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePane {
    RepoList,
    Detail,
    Help,
    ArgumentInput,
    CommandOutput,
}

/// State for argument input dialog.
#[derive(Debug, Clone)]
struct ArgumentInputState {
    buffer: String,
    cursor_pos: usize, // Character position (not byte position)
    command_name: String,
}

/// Main TUI application state.
#[allow(clippy::struct_excessive_bools)]
pub struct App<R, D> {
    registry: R,
    detail_provider: D,
    list_state: ListState,
    should_quit: bool,
    active_pane: ActivePane,
    active_tab: DetailTab,
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
    argument_input: Option<ArgumentInputState>,
    output_lines: Vec<String>,
    output_scroll: usize,
    output_truncated_start: bool,
    command_state: CommandState,
    command_name: Option<String>,
    graft_loader: GraftYamlConfigLoader,
    command_event_rx: Option<Receiver<CommandEvent>>,
    running_command_pid: Option<u32>,
    show_stop_confirmation: bool,

    // State query panel
    state_queries: Vec<crate::state::StateQuery>,
    state_results: Vec<Option<crate::state::StateResult>>,
    state_panel_list_state: ListState,
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
            app.render(&mut terminal)?; // Show "Refreshing..." message
            app.handle_refresh_if_needed(); // Do the refresh
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
mod tests;
