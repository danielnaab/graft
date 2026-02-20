//! Terminal UI implementation using ratatui.

mod app;
mod command_exec;
mod command_line;
mod hint_bar;
mod overlays;
mod render;
mod repo_detail;
mod repo_list;
mod status_bar;
mod text_buffer;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
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

/// A view in the view stack.
///
/// Each variant represents a full-screen content area. Navigation pushes and
/// pops views; `q` pops, `Escape` resets to Dashboard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum View {
    /// The repo list (home view).
    Dashboard,
    /// Detail view for a specific repository (index into the registry list).
    RepoDetail(usize),
    /// Full-screen command output.
    CommandOutput,
    /// Help / keybindings reference.
    Help,
}

/// State for argument input dialog.
#[derive(Debug, Clone)]
struct ArgumentInputState {
    text: text_buffer::TextBuffer,
    command_name: String,
}

/// Value for a single form field.
#[derive(Debug, Clone)]
enum FieldValue {
    /// Free-text input (for string type).
    Text(text_buffer::TextBuffer),
    /// Selected index into ArgDef.options (for choice type).
    Choice(usize),
    /// Toggle on/off (for flag type).
    Flag(bool),
}

/// A single field in the schema-driven argument form.
#[derive(Debug, Clone)]
struct FormField {
    def: grove_core::ArgDef,
    value: FieldValue,
}

/// State for the schema-driven argument form overlay.
#[derive(Debug, Clone)]
struct FormInputState {
    command_name: String,
    fields: Vec<FormField>,
    focused: usize,
}

impl FormInputState {
    /// Build a form from an arg schema, pre-populating defaults.
    fn from_schema(command_name: String, args: Vec<grove_core::ArgDef>) -> Self {
        let fields = args
            .into_iter()
            .map(|def| {
                let value = match def.arg_type {
                    grove_core::ArgType::String => {
                        let mut buf = text_buffer::TextBuffer::new();
                        if let Some(ref default) = def.default {
                            buf.set(default);
                        }
                        FieldValue::Text(buf)
                    }
                    grove_core::ArgType::Choice => {
                        let idx = def
                            .default
                            .as_ref()
                            .and_then(|d| {
                                def.options
                                    .as_ref()
                                    .and_then(|opts| opts.iter().position(|o| o == d))
                            })
                            .unwrap_or(0);
                        FieldValue::Choice(idx)
                    }
                    grove_core::ArgType::Flag => {
                        let on = def.default.as_ref().is_some_and(|d| d == "true");
                        FieldValue::Flag(on)
                    }
                };
                FormField { def, value }
            })
            .collect();

        Self {
            command_name,
            fields,
            focused: 0,
        }
    }
}

/// State for the vim-style `:` command line.
///
/// When active, the command line renders at the bottom of the screen
/// (replacing the hint bar) and accepts a single-line command input.
/// `Escape` cancels; `Enter` submits.
/// A command palette popup is shown above the command line when the
/// buffer is empty or partially typed; `j`/`k` navigate it.
#[derive(Debug, Clone)]
struct CommandLineState {
    text: text_buffer::TextBuffer,
    palette_selected: usize,      // Index into the filtered palette entries
    history_index: Option<usize>, // Position in command_history when browsing
    history_draft: String,        // Saves the user's in-progress input when browsing history
}

/// Main TUI application state.
#[allow(clippy::struct_excessive_bools)]
pub struct App<R, D> {
    registry: R,
    detail_provider: D,
    list_state: ListState,
    should_quit: bool,
    /// View stack — the top of the stack is the current view.
    /// Invariant: always has at least one element (Dashboard).
    view_stack: Vec<View>,
    /// Active `:` command line state, or `None` when the command line is dismissed.
    command_line: Option<CommandLineState>,
    /// Command history (most recent last, bounded at 50).
    command_history: Vec<String>,
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
    form_input: Option<FormInputState>,
    output_lines: Vec<String>,
    output_scroll: usize,
    output_truncated_start: bool,
    command_state: CommandState,
    command_name: Option<String>,
    graft_loader: GraftYamlConfigLoader,
    command_event_rx: Option<Receiver<CommandEvent>>,
    running_command_pid: Option<u32>,
    show_stop_confirmation: bool,

    // State queries (loaded lazily for the current repo in RepoDetail view)
    state_queries: Vec<crate::state::StateQuery>,
    state_results: Vec<Option<crate::state::StateResult>>,
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
                    app.handle_key_event(key);
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
