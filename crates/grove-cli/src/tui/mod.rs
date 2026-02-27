//! Terminal UI implementation using ratatui.
//!
//! Transcript-paradigm TUI: scrolling content area with a command prompt.

mod command_exec;
mod command_line;
mod formatting;
mod header;
mod prompt;
mod scroll_buffer;
mod status_bar;
mod text_buffer;
mod transcript;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use grove_core::{RepoDetailProvider, RepoRegistry};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

// Re-export public items
#[allow(unused_imports)]
pub use command_exec::CommandEvent;

// Re-export for integration tests
#[allow(unused_imports)]
pub use command_exec::spawn_command;

/// Check if the terminal supports Unicode characters.
///
/// Returns false for terminals known to have poor Unicode support.
fn supports_unicode() -> bool {
    std::env::var("TERM")
        .map(|term| !term.contains("linux") && !term.contains("ascii") && !term.contains("vt100"))
        .unwrap_or(true)
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
    let mut app = transcript::TranscriptApp::new(registry, detail_provider, workspace_name);

    // Main event loop
    let result = loop {
        // If refresh requested, render "Refreshing..." first, then refresh
        if app.needs_refresh {
            app.render(&mut terminal)?;
            app.handle_refresh_if_needed();
        }

        app.render(&mut terminal)?;

        if app.should_quit {
            break Ok(());
        }

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    app.handle_key(key.code, key.modifiers);
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
