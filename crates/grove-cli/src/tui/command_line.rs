//! Vim-style `:` command line input handling.

use super::{App, KeyCode, RepoDetailProvider, RepoRegistry, StatusMessage};

impl<R: RepoRegistry, D: RepoDetailProvider> App<R, D> {
    /// Handle a key press when the command line is active.
    ///
    /// The command line intercepts all keys before view dispatch. `Escape`
    /// cancels; `Enter` submits (parsing handled in Task 8); other keys edit
    /// the buffer.
    pub(super) fn handle_key_command_line(&mut self, code: KeyCode) {
        let Some(state) = &mut self.command_line else {
            return;
        };

        match code {
            KeyCode::Esc => {
                // Cancel command line — dismiss without executing.
                self.command_line = None;
            }
            KeyCode::Enter => {
                let buffer = state.buffer.clone();
                self.command_line = None;

                if !buffer.is_empty() {
                    // Command parsing and execution handled in Task 8.
                    // For now, show an informational message.
                    self.status_message = Some(StatusMessage::info(format!(
                        "Command line: :{buffer} (execution in Task 8)"
                    )));
                }
                // Empty Enter dismisses command line silently.
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
}
