//! Prompt state and rendering for the transcript TUI.
//!
//! The prompt sits at the bottom of the screen and handles command input,
//! palette navigation, history, and argument hints.

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
};

use super::command_line::{filtered_palette, parse_command, CliCommand};
use super::text_buffer::TextBuffer;

// ===== Picker overlay component =====

/// The outcome of a key event handled by [`PickerState`].
#[derive(Debug, PartialEq)]
#[allow(dead_code)]
pub(super) enum PickerOutcome {
    /// The user selected an item; execute this command.
    Select(CliCommand),
    /// The user dismissed the picker (Esc).
    Dismiss,
    /// No selection yet; continue showing the picker.
    Nothing,
}

/// A single item in a picker overlay.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(super) struct PickerItem {
    /// Primary text shown in the left column.
    pub(super) label: String,
    /// Secondary text shown in the right column.
    pub(super) description: String,
    /// Command to execute when this item is selected.
    pub(super) action: CliCommand,
}

/// A filterable, navigable picker overlay rendered above the prompt line.
///
/// Renders identically to the command palette: a bordered `List` widget with
/// cyan highlight on the selected row. Used both as the backing store for the
/// command palette and (in future steps) as a standalone overlay for table
/// blocks.
#[derive(Debug)]
pub(super) struct PickerState {
    /// All available items (unfiltered).
    pub(super) items: Vec<PickerItem>,
    /// Current filter text (case-insensitive substring match on label).
    pub(super) filter: String,
    /// Currently highlighted row index (into the filtered item list).
    pub(super) selected: usize,
}

#[allow(dead_code)]
impl PickerState {
    pub(super) fn new(items: Vec<PickerItem>) -> Self {
        Self {
            items,
            filter: String::new(),
            selected: 0,
        }
    }

    /// Return the subset of items whose label contains `self.filter`
    /// (case-insensitive substring match).
    pub(super) fn filtered_items(&self) -> Vec<&PickerItem> {
        if self.filter.is_empty() {
            self.items.iter().collect()
        } else {
            let f = self.filter.to_ascii_lowercase();
            self.items
                .iter()
                .filter(|i| i.label.to_ascii_lowercase().contains(&f))
                .collect()
        }
    }

    /// Handle a key event and update internal state.
    ///
    /// - j / Down  → move selection down (wraps)
    /// - k / Up    → move selection up (wraps)
    /// - Char       → append to filter, reset selection
    /// - Backspace  → remove last filter char, reset selection
    /// - Enter      → return `Select(action)` for the highlighted item
    /// - Esc        → return `Dismiss`
    pub(super) fn handle_key(&mut self, code: KeyCode, _modifiers: KeyModifiers) -> PickerOutcome {
        match code {
            KeyCode::Esc => PickerOutcome::Dismiss,
            KeyCode::Enter => {
                let items = self.filtered_items();
                if items.is_empty() {
                    PickerOutcome::Nothing
                } else {
                    let idx = self.selected.min(items.len() - 1);
                    PickerOutcome::Select(items[idx].action.clone())
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let count = self.filtered_items().len();
                if count > 0 {
                    self.selected = if self.selected + 1 >= count {
                        0
                    } else {
                        self.selected + 1
                    };
                }
                PickerOutcome::Nothing
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let count = self.filtered_items().len();
                if count > 0 {
                    self.selected = if self.selected == 0 {
                        count - 1
                    } else {
                        self.selected - 1
                    };
                }
                PickerOutcome::Nothing
            }
            KeyCode::Char(c) => {
                self.filter.push(c);
                self.selected = 0;
                PickerOutcome::Nothing
            }
            KeyCode::Backspace => {
                self.filter.pop();
                self.selected = 0;
                PickerOutcome::Nothing
            }
            _ => PickerOutcome::Nothing,
        }
    }

    /// Render the picker overlay floating above `above_area`.
    ///
    /// The popup is anchored to the bottom-left of `above_area`, grows upward,
    /// and is styled identically to the command palette (bordered List, cyan
    /// highlight, black background).
    pub(super) fn render(&self, frame: &mut ratatui::Frame, above_area: Rect, title: &str) {
        let items = self.filtered_items();
        if items.is_empty() {
            return;
        }

        let count = items.len();
        let max_content_width = items
            .iter()
            .map(|i| i.label.len() + 2 + i.description.len())
            .max()
            .unwrap_or(20);

        let list_items: Vec<ListItem> = items
            .iter()
            .map(|item| {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{:<10}", item.label),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::styled(
                        format!("  {}", item.description),
                        Style::default().fg(Color::White),
                    ),
                ]))
            })
            .collect();

        let max_height = above_area.height.saturating_sub(1);
        let count_u16 = u16::try_from(count).unwrap_or(u16::MAX);
        let popup_height = count_u16.saturating_add(2).min(max_height);
        if popup_height < 3 {
            return;
        }

        let width_u16 = u16::try_from(max_content_width).unwrap_or(u16::MAX);
        let popup_width = width_u16.saturating_add(4).min(above_area.width);

        let x = above_area.x;
        let y = above_area
            .y
            .saturating_add(above_area.height)
            .saturating_sub(popup_height);

        let popup_area = Rect {
            x,
            y,
            width: popup_width,
            height: popup_height,
        };

        frame.render_widget(Clear, popup_area);

        let list = List::new(list_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(Style::default().fg(Color::Cyan))
                    .style(Style::default().fg(Color::White).bg(Color::Black)),
            )
            .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan));

        let mut list_state = ListState::default();
        list_state.select(Some(self.selected.min(count.saturating_sub(1))));

        frame.render_stateful_widget(list, popup_area, &mut list_state);
    }
}

/// Maximum number of command history entries to keep.
const MAX_HISTORY: usize = 50;

/// A single argument completion entry shown in the completion popup.
#[derive(Debug, Clone)]
pub(super) struct ArgCompletion {
    /// The completion value (e.g., command name, repo name).
    pub(super) value: String,
    /// Short description shown next to the value.
    pub(super) description: String,
    /// Optional group label for sectioned completion popups.
    pub(super) group: Option<String>,
}

/// Bundled completion state passed from the caller to prompt methods.
#[derive(Debug, Default)]
pub(super) struct CompletionState {
    /// Completions to show in the popup menu.
    pub(super) completions: Vec<ArgCompletion>,
    /// Whether more input is required before the command can be submitted.
    /// When true, Enter fills the selected completion and appends a space
    /// instead of submitting, forcing the user to provide all required args.
    pub(super) requires_more_input: bool,
    /// Ghost hint text to show when no popup completions exist (e.g., `<arg_name>`).
    pub(super) arg_hint: Option<String>,
}

/// State for the command line (active when user has pressed `:`)
#[derive(Debug, Clone)]
pub(super) struct CommandLineState {
    pub(super) text: TextBuffer,
    pub(super) palette_selected: usize,
    pub(super) history_index: Option<usize>,
    pub(super) history_draft: String,
}

/// Full prompt state.
#[derive(Debug)]
pub(super) struct PromptState {
    /// Active command line state (Some when `:` prompt is open).
    pub(super) command_line: Option<CommandLineState>,
    /// Command history (most recent last, bounded at `MAX_HISTORY`).
    pub(super) history: Vec<String>,
}

impl PromptState {
    pub(super) fn new() -> Self {
        Self {
            command_line: None,
            history: Vec::new(),
        }
    }

    /// Whether the command line is currently active.
    pub(super) fn is_active(&self) -> bool {
        self.command_line.is_some()
    }

    /// Open the command line prompt.
    pub(super) fn open(&mut self) {
        self.command_line = Some(CommandLineState {
            text: TextBuffer::new(),
            palette_selected: 0,
            history_index: None,
            history_draft: String::new(),
        });
    }

    /// Open the command line prompt pre-populated with text (cursor at end).
    pub(super) fn open_with(&mut self, text: &str) {
        self.command_line = Some(CommandLineState {
            text: TextBuffer::with_content(text, text.len()),
            palette_selected: 0,
            history_index: None,
            history_draft: String::new(),
        });
    }

    /// Close the command line prompt without executing.
    pub(super) fn close(&mut self) {
        self.command_line = None;
    }

    /// Save a command to history.
    fn push_history(&mut self, cmd: &str) {
        if cmd.is_empty() {
            return;
        }
        if self.history.last().is_some_and(|last| last == cmd) {
            return; // Skip consecutive duplicates
        }
        self.history.push(cmd.to_string());
        if self.history.len() > MAX_HISTORY {
            self.history.remove(0);
        }
    }

    /// Handle a key event when the command line is active.
    ///
    /// Returns `Some(CliCommand)` if the user submitted a command, `None` otherwise.
    #[allow(clippy::too_many_lines)]
    pub(super) fn handle_key(
        &mut self,
        code: KeyCode,
        modifiers: KeyModifiers,
        cs: &CompletionState,
    ) -> Option<CliCommand> {
        let state = self.command_line.as_mut()?;

        // Handle Ctrl shortcuts first
        if modifiers.contains(KeyModifiers::CONTROL) {
            match code {
                KeyCode::Char('u') => {
                    state.text.clear();
                    state.palette_selected = 0;
                    return None;
                }
                KeyCode::Char('w') => {
                    state.text.delete_word_backward();
                    state.palette_selected = 0;
                    return None;
                }
                _ => {}
            }
        }

        match code {
            KeyCode::Esc => {
                self.close();
                None
            }
            KeyCode::Enter => {
                let buffer = state.text.buffer.clone();
                let palette_entries = filtered_palette(&buffer);

                // If palette is showing, use the selected entry
                if !palette_entries.is_empty() {
                    let selected = state.palette_selected.min(palette_entries.len() - 1);
                    let entry = palette_entries[selected];
                    if entry.takes_args || cs.requires_more_input {
                        // Fill the buffer with the command name so user can add args
                        state.text.set(&format!("{} ", entry.command));
                        state.palette_selected = 0;
                        return None;
                    }
                    let command = entry.command.to_string();
                    self.push_history(&command);
                    self.close();
                    return Some(parse_command(&command));
                }

                // Empty buffer, no palette — just close
                if buffer.is_empty() {
                    self.close();
                    return None;
                }

                // Arg completion selection
                if !cs.completions.is_empty() {
                    let selected = state
                        .palette_selected
                        .min(cs.completions.len().saturating_sub(1));
                    let completion = &cs.completions[selected];
                    let prefix = extract_command_prefix(&buffer);
                    let full_command = format!("{prefix}{}", completion.value);

                    // If more args are required after this one, stay open
                    if cs.requires_more_input {
                        state.text.set(&format!("{full_command} "));
                        state.palette_selected = 0;
                        return None;
                    }

                    self.push_history(&full_command);
                    self.close();
                    return Some(parse_command(&full_command));
                }

                // If the command still needs more input, don't submit
                if cs.requires_more_input {
                    // Append a space to prompt for the next argument
                    if !buffer.ends_with(' ') {
                        state.text.set(&format!("{buffer} "));
                        state.palette_selected = 0;
                    }
                    return None;
                }

                // Normal: submit buffer as-is
                self.push_history(&buffer);
                self.close();
                Some(parse_command(&buffer))
            }
            // Palette navigation: j/k only when buffer is empty
            KeyCode::Char('j') if state.text.buffer.is_empty() => {
                let entries = filtered_palette(&state.text.buffer);
                if !entries.is_empty() {
                    let next = state.palette_selected + 1;
                    state.palette_selected = if next >= entries.len() { 0 } else { next };
                }
                None
            }
            KeyCode::Char('k') if state.text.buffer.is_empty() => {
                let entries = filtered_palette(&state.text.buffer);
                if !entries.is_empty() {
                    state.palette_selected = if state.palette_selected == 0 {
                        entries.len() - 1
                    } else {
                        state.palette_selected - 1
                    };
                }
                None
            }
            KeyCode::Down => {
                if state.history_index.is_some() {
                    self.history_down();
                } else {
                    let state = self.command_line.as_mut().unwrap();
                    let entries = filtered_palette(&state.text.buffer);
                    if !entries.is_empty() {
                        let next = state.palette_selected + 1;
                        state.palette_selected = if next >= entries.len() { 0 } else { next };
                    } else if !cs.completions.is_empty() {
                        let next = state.palette_selected + 1;
                        state.palette_selected = if next >= cs.completions.len() {
                            0
                        } else {
                            next
                        };
                    } else {
                        self.history_down();
                    }
                }
                None
            }
            KeyCode::Up => {
                if state.history_index.is_some() {
                    self.history_up();
                } else {
                    let state = self.command_line.as_mut().unwrap();
                    let entries = filtered_palette(&state.text.buffer);
                    if !entries.is_empty() {
                        state.palette_selected = if state.palette_selected == 0 {
                            entries.len() - 1
                        } else {
                            state.palette_selected - 1
                        };
                    } else if !cs.completions.is_empty() {
                        state.palette_selected = if state.palette_selected == 0 {
                            cs.completions.len() - 1
                        } else {
                            state.palette_selected - 1
                        };
                    } else {
                        self.history_up();
                    }
                }
                None
            }
            KeyCode::Tab => {
                let entries = filtered_palette(&state.text.buffer);
                match entries.len() {
                    1 => {
                        let entry = entries[0];
                        let completed = if entry.takes_args {
                            format!("{} ", entry.command)
                        } else {
                            entry.command.to_string()
                        };
                        state.text.set(&completed);
                        state.palette_selected = 0;
                    }
                    2.. => {
                        let commands: Vec<&str> = entries.iter().map(|e| e.command).collect();
                        let prefix = longest_common_prefix(&commands);
                        if prefix.len() > state.text.buffer.len() {
                            state.text.set(&prefix);
                            state.palette_selected = 0;
                        }
                    }
                    _ => {
                        // Fill from selected arg completion
                        if !cs.completions.is_empty() {
                            let selected = state
                                .palette_selected
                                .min(cs.completions.len().saturating_sub(1));
                            let completion = &cs.completions[selected];
                            let prefix = extract_command_prefix(&state.text.buffer);
                            state.text.set(&format!("{prefix}{}", completion.value));
                        }
                    }
                }
                None
            }
            KeyCode::Left => {
                state.text.move_left();
                None
            }
            KeyCode::Right => {
                state.text.move_right();
                None
            }
            KeyCode::Home => {
                state.text.move_home();
                None
            }
            KeyCode::End => {
                state.text.move_end();
                None
            }
            KeyCode::Delete => {
                state.text.delete_forward();
                state.palette_selected = 0;
                None
            }
            KeyCode::Char(c) => {
                state.text.insert_char(c);
                state.palette_selected = 0;
                state.history_index = None;
                None
            }
            KeyCode::Backspace => {
                state.text.backspace();
                state.palette_selected = 0;
                state.history_index = None;
                None
            }
            _ => None,
        }
    }

    /// Navigate command history upward (toward older entries).
    fn history_up(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let Some(state) = &mut self.command_line else {
            return;
        };

        match state.history_index {
            None => {
                state.history_draft = state.text.buffer.clone();
                let idx = self.history.len() - 1;
                state.history_index = Some(idx);
                state.text.set(&self.history[idx]);
            }
            Some(idx) if idx > 0 => {
                let new_idx = idx - 1;
                state.history_index = Some(new_idx);
                state.text.set(&self.history[new_idx]);
            }
            _ => {}
        }
    }

    /// Navigate command history downward (toward newer entries / back to draft).
    fn history_down(&mut self) {
        let Some(state) = &mut self.command_line else {
            return;
        };

        let Some(idx) = state.history_index else {
            return;
        };

        if idx + 1 < self.history.len() {
            let new_idx = idx + 1;
            state.history_index = Some(new_idx);
            state.text.set(&self.history[new_idx]);
        } else {
            state.history_index = None;
            let draft = state.history_draft.clone();
            state.text.set(&draft);
        }
    }

    /// Compute completions and metadata for the current buffer.
    ///
    /// Returns a `CompletionState` with:
    /// - `completions`: filtered popup entries for the current argument position
    /// - `requires_more_input`: true when required args are still needed
    /// - `arg_hint`: ghost text for String-type args (e.g., `<name>`)
    #[allow(clippy::too_many_lines)]
    pub(super) fn compute_completions(
        &self,
        available_commands: &[(String, graft_common::CommandDef)],
        repo_names: &[String],
        state_query_names: &[String],
        focus_entity_opts: &std::collections::HashMap<String, Vec<String>>,
        scion_completions: &[ArgCompletion],
    ) -> CompletionState {
        let Some(state) = self.command_line.as_ref() else {
            return CompletionState::default();
        };
        let buffer = &state.text.buffer;

        // Only complete when cursor is at the end
        if state.text.cursor_pos != buffer.chars().count() {
            return CompletionState::default();
        }

        // Only complete arguments when the user has typed a space after the command name
        if !buffer.contains(char::is_whitespace) {
            return CompletionState::default();
        }

        let mut parts = buffer.splitn(2, char::is_whitespace);
        let cmd = match parts.next() {
            Some(c) => c.to_ascii_lowercase(),
            None => return CompletionState::default(),
        };
        let rest = parts.next().unwrap_or("").trim_start();

        match cmd.as_str() {
            "run" => compute_run_completions(rest, available_commands),
            "repo" => {
                if rest.contains(char::is_whitespace) {
                    return CompletionState::default();
                }
                let partial = rest.to_ascii_lowercase();
                CompletionState {
                    completions: repo_names
                        .iter()
                        .filter(|name| name.to_ascii_lowercase().starts_with(&partial))
                        .map(|name| ArgCompletion {
                            value: name.clone(),
                            description: String::new(),
                            group: None,
                        })
                        .collect(),
                    ..CompletionState::default()
                }
            }
            "state" | "invalidate" | "inv" => {
                if rest.contains(char::is_whitespace) {
                    return CompletionState::default();
                }
                let partial = rest.to_ascii_lowercase();
                CompletionState {
                    completions: state_query_names
                        .iter()
                        .filter(|name| name.to_ascii_lowercase().starts_with(&partial))
                        .map(|name| ArgCompletion {
                            value: name.clone(),
                            description: String::new(),
                            group: None,
                        })
                        .collect(),
                    ..CompletionState::default()
                }
            }
            "catalog" | "cat" => {
                if rest.contains(char::is_whitespace) {
                    return CompletionState::default();
                }
                let partial = rest.to_ascii_lowercase();
                let categories = [
                    ("core", "Core commands"),
                    ("diagnostic", "Diagnostic tools"),
                    ("optional", "Optional commands"),
                    ("advanced", "Advanced commands"),
                    ("uncategorized", "Other"),
                ];
                CompletionState {
                    completions: categories
                        .iter()
                        .filter(|(name, _)| name.starts_with(partial.as_str()))
                        .map(|(name, desc)| ArgCompletion {
                            value: (*name).to_string(),
                            description: (*desc).to_string(),
                            group: None,
                        })
                        .collect(),
                    ..CompletionState::default()
                }
            }
            "focus" | "f" => {
                if rest.contains(char::is_whitespace) {
                    // Second arg: complete entity values for the named query
                    let mut parts = rest.splitn(2, char::is_whitespace);
                    let query_name = parts.next().unwrap_or("");
                    let partial = parts.next().unwrap_or("").trim_start().to_ascii_lowercase();
                    let opts = focus_entity_opts
                        .get(query_name)
                        .map_or(&[][..], Vec::as_slice);
                    CompletionState {
                        completions: opts
                            .iter()
                            .filter(|v| v.to_ascii_lowercase().starts_with(&partial))
                            .map(|v| ArgCompletion {
                                value: v.clone(),
                                description: String::new(),
                                group: None,
                            })
                            .collect(),
                        ..CompletionState::default()
                    }
                } else {
                    // First arg: complete query names
                    let partial = rest.to_ascii_lowercase();
                    CompletionState {
                        completions: state_query_names
                            .iter()
                            .filter(|n| n.to_ascii_lowercase().starts_with(&partial))
                            .map(|n| ArgCompletion {
                                value: n.clone(),
                                description: String::new(),
                                group: None,
                            })
                            .collect(),
                        ..CompletionState::default()
                    }
                }
            }
            "scion" | "sc" => {
                // Parse subcommand and name
                let sub_parts: Vec<&str> = rest.splitn(2, char::is_whitespace).collect();
                let sub = sub_parts.first().copied().unwrap_or("");
                let sub_lower = sub.to_ascii_lowercase();

                // Only match committed subcommands (with trailing space, i.e.
                // sub_parts.len() > 1). Without a space after the subcommand,
                // fall through to the subcommand-completion arm below.
                match (sub_lower.as_str(), sub_parts.len() > 1) {
                    // list doesn't need a name
                    ("list" | "ls", _) => CompletionState::default(),
                    // create needs a name but no completion (new name)
                    ("create", true) => {
                        let has_name = sub_parts.get(1).is_some_and(|s| !s.trim().is_empty());
                        CompletionState {
                            completions: vec![],
                            requires_more_input: !has_name,
                            arg_hint: if has_name {
                                None
                            } else {
                                Some("<name>".to_string())
                            },
                        }
                    }
                    ("start" | "stop" | "prune" | "fuse" | "run", true) => {
                        // Complete scion name
                        let name_partial = sub_parts.get(1).unwrap_or(&"").trim();
                        if name_partial.contains(char::is_whitespace) {
                            return CompletionState::default();
                        }
                        filter_scion_completions(name_partial, scion_completions)
                    }
                    _ => {
                        // Complete subcommand name
                        // (name, description, takes_args)
                        let subs: &[(&str, &str, bool)] = &[
                            ("list", "List all scions", false),
                            ("create", "Create a new scion", true),
                            ("start", "Start runtime session", true),
                            ("stop", "Stop runtime session", true),
                            ("prune", "Remove a scion", true),
                            ("fuse", "Fuse into main", true),
                            ("run", "Create & start", true),
                        ];
                        let partial = sub.to_ascii_lowercase();
                        let filtered: Vec<_> = subs
                            .iter()
                            .filter(|(name, _, _)| name.starts_with(partial.as_str()))
                            .collect();
                        // requires_more_input is false only when the sole match takes no args
                        let needs_more = filtered.len() != 1 || filtered[0].2;
                        CompletionState {
                            completions: filtered
                                .iter()
                                .map(|(name, desc, _)| ArgCompletion {
                                    value: (*name).to_string(),
                                    description: (*desc).to_string(),
                                    group: None,
                                })
                                .collect(),
                            requires_more_input: needs_more,
                            arg_hint: None,
                        }
                    }
                }
            }
            "attach" | "review" => {
                if rest.contains(char::is_whitespace) {
                    return CompletionState::default();
                }
                filter_scion_completions(rest, scion_completions)
            }
            _ => CompletionState::default(),
        }
    }

    /// Render the command palette or argument completion popup above the prompt.
    pub(super) fn render_palette(
        &self,
        frame: &mut ratatui::Frame,
        above_area: Rect,
        cs: &CompletionState,
    ) {
        let Some(state) = &self.command_line else {
            return;
        };

        let palette_entries = filtered_palette(&state.text.buffer);

        // Command palette: delegate rendering to PickerState.
        if !palette_entries.is_empty() {
            let picker_items: Vec<PickerItem> = palette_entries
                .iter()
                .map(|e| PickerItem {
                    label: e.command.to_string(),
                    description: e.description.to_string(),
                    action: parse_command(e.command),
                })
                .collect();
            let picker = PickerState {
                items: picker_items,
                filter: String::new(),
                selected: state
                    .palette_selected
                    .min(palette_entries.len().saturating_sub(1)),
            };
            picker.render(frame, above_area, " Commands ");
            return;
        }

        // Argument completion popup (different styling — value only, DarkGray description).
        if !cs.completions.is_empty() {
            render_completion_popup(frame, above_area, cs, state.palette_selected);
        }
    }

    /// Render the command line prompt in the given area.
    ///
    /// Ghost hint text is derived from the selected arg completion or the
    /// `arg_hint` field for String-type args.
    pub(super) fn render_prompt(
        &self,
        frame: &mut ratatui::Frame,
        area: Rect,
        cs: &CompletionState,
    ) {
        let Some(state) = &self.command_line else {
            // When prompt is not active, render a dim hint
            let widget = Paragraph::new(Line::from(Span::styled(
                " : to open command palette",
                Style::default().fg(Color::DarkGray),
            )));
            frame.render_widget(widget, area);
            return;
        };

        // Derive ghost hint: from selected completion, or from arg_hint for String-type args
        let ghost_hint = if filtered_palette(&state.text.buffer).is_empty() {
            if cs.completions.is_empty() {
                cs.arg_hint.clone()
            } else {
                let selected = state
                    .palette_selected
                    .min(cs.completions.len().saturating_sub(1));
                ghost_hint_suffix(&state.text.buffer, &cs.completions[selected].value)
            }
        } else {
            None
        };

        let chars: Vec<char> = state.text.buffer.chars().collect();
        let before_cursor: String = chars[..state.text.cursor_pos].iter().collect();
        let after_cursor: String = chars[state.text.cursor_pos..].iter().collect();

        let mut spans = vec![
            Span::styled(":", Style::default().fg(Color::Cyan)),
            Span::styled(before_cursor, Style::default().fg(Color::White)),
        ];

        if after_cursor.is_empty() {
            spans.push(Span::styled("_", Style::default().fg(Color::White)));
            if let Some(hint) = &ghost_hint {
                spans.push(Span::styled(
                    hint.clone(),
                    Style::default().fg(Color::DarkGray),
                ));
            }
        } else {
            let mut after_chars = after_cursor.chars();
            let cursor_char = after_chars.next().unwrap_or(' ');
            let rest: String = after_chars.collect();
            spans.push(Span::styled(
                cursor_char.to_string(),
                Style::default().fg(Color::Black).bg(Color::White),
            ));
            if !rest.is_empty() {
                spans.push(Span::styled(rest, Style::default().fg(Color::White)));
            }
        }

        let line = Line::from(spans);
        let widget = Paragraph::new(line);
        frame.render_widget(widget, area);
    }
}

// ===== Run-command multi-arg completions =====

/// Compute completions for `:run` — handles command name and per-argument completion.
pub(super) fn compute_run_completions(
    rest: &str,
    available_commands: &[(String, graft_common::CommandDef)],
) -> CompletionState {
    let words: Vec<&str> = rest.split_whitespace().collect();
    let trailing_space = rest.ends_with(' ');

    // Phase 1: completing the graft command name
    if words.is_empty() || (words.len() == 1 && !trailing_space) {
        let partial = words.first().copied().unwrap_or("");
        let partial_lower = partial.to_ascii_lowercase();
        return CompletionState {
            completions: available_commands
                .iter()
                .filter(|(name, _)| name.to_ascii_lowercase().starts_with(&partial_lower))
                .map(|(name, def)| ArgCompletion {
                    value: name.clone(),
                    description: def.description.clone().unwrap_or_default(),
                    group: None,
                })
                .collect(),
            // requires_more_input: true if every matching command has required args
            // (but we don't block here — the block happens after the name is chosen)
            requires_more_input: false,
            arg_hint: None,
        };
    }

    // Phase 2+: command name is known, completing arguments
    let cmd_name = words[0];
    let Some((_, cmd_def)) = available_commands.iter().find(|(n, _)| n == cmd_name) else {
        return CompletionState::default();
    };
    let Some(arg_defs) = &cmd_def.args else {
        return CompletionState::default();
    };

    // How many complete args have been provided?
    let words_after_cmd = words.len() - 1;
    let (arg_index, partial) = if trailing_space {
        (words_after_cmd, "")
    } else {
        (
            words_after_cmd.saturating_sub(1),
            words.last().copied().unwrap_or(""),
        )
    };

    // Count how many required args (no default) remain after what's been provided
    let complete_args = if trailing_space {
        words_after_cmd
    } else {
        words_after_cmd.saturating_sub(1)
    };
    let required_count = arg_defs
        .iter()
        .filter(|a| a.required && a.default.is_none())
        .count();
    let requires_more = complete_args < required_count;

    let Some(arg_def) = arg_defs.get(arg_index) else {
        // Past the defined args — no completions but check required
        return CompletionState {
            requires_more_input: requires_more,
            ..CompletionState::default()
        };
    };

    let partial_lower = partial.to_ascii_lowercase();

    match arg_def.arg_type {
        graft_common::ArgType::Choice => {
            let completions = if let Some(options) = &arg_def.options {
                options
                    .iter()
                    .filter(|o| o.to_ascii_lowercase().starts_with(&partial_lower))
                    .map(|o| ArgCompletion {
                        value: o.clone(),
                        description: arg_def.description.clone().unwrap_or_default(),
                        group: None,
                    })
                    .collect()
            } else {
                Vec::new()
            };
            CompletionState {
                completions,
                requires_more_input: requires_more,
                arg_hint: None,
            }
        }
        graft_common::ArgType::Flag => {
            let options = ["true", "false"];
            CompletionState {
                completions: options
                    .iter()
                    .filter(|o| o.starts_with(&partial_lower))
                    .map(|o| ArgCompletion {
                        value: (*o).to_string(),
                        description: arg_def.description.clone().unwrap_or_default(),
                        group: None,
                    })
                    .collect(),
                requires_more_input: requires_more,
                arg_hint: None,
            }
        }
        graft_common::ArgType::String => {
            // No popup for free-form strings, but show arg name as ghost hint
            CompletionState {
                completions: Vec::new(),
                requires_more_input: requires_more,
                arg_hint: Some(format!("<{}>", arg_def.name)),
            }
        }
    }
}

// ===== Completion popup rendering =====

/// Render the argument completion popup, with optional group headers.
fn render_completion_popup(
    frame: &mut ratatui::Frame,
    above_area: Rect,
    cs: &CompletionState,
    palette_selected: usize,
) {
    // Show group headers only when there are at least two distinct groups.
    // A single group produces a header that wastes space without aiding navigation.
    let has_groups = {
        let mut distinct = cs.completions.iter().filter_map(|c| c.group.as_deref());
        match distinct.next() {
            None => false,
            Some(first) => distinct.any(|g| g != first),
        }
    };

    // Build list items and determine the selected display row.
    let (items, selected_display_row, max_content_width) = if has_groups {
        build_grouped_items(cs, palette_selected)
    } else {
        build_flat_items(cs, palette_selected)
    };

    let total_rows = items.len();
    let max_height = above_area.height.saturating_sub(1);
    let count_u16 = u16::try_from(total_rows).unwrap_or(u16::MAX);
    let popup_height = count_u16.saturating_add(2).min(max_height);
    if popup_height < 3 {
        return;
    }

    let width_u16 = u16::try_from(max_content_width).unwrap_or(u16::MAX);
    let popup_width = width_u16.saturating_add(4).min(above_area.width);

    let x = above_area.x;
    let y = above_area
        .y
        .saturating_add(above_area.height)
        .saturating_sub(popup_height);

    let popup_area = Rect {
        x,
        y,
        width: popup_width,
        height: popup_height,
    };

    frame.render_widget(Clear, popup_area);

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Completions ")
                .border_style(Style::default().fg(Color::Cyan))
                .style(Style::default().fg(Color::White).bg(Color::Black)),
        )
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan));

    let mut list_state = ListState::default();
    list_state.select(Some(selected_display_row));

    frame.render_stateful_widget(list, popup_area, &mut list_state);
}

/// Build list items with group section headers interleaved.
pub(super) fn build_grouped_items(
    cs: &CompletionState,
    palette_selected: usize,
) -> (Vec<ListItem<'_>>, usize, usize) {
    let mut items: Vec<ListItem> = Vec::new();
    let mut selectable_to_display: Vec<usize> = Vec::new();
    let mut current_group: Option<&str> = None;
    let mut first = true;
    let mut max_w = 0usize;

    for c in &cs.completions {
        let group_label = c.group.as_deref();
        if first || group_label != current_group {
            first = false;
            let label = group_label.unwrap_or("other");
            items.push(ListItem::new(Line::from(Span::styled(
                format!("── {label} ──"),
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ))));
            let hw = label.len() + 6;
            if hw > max_w {
                max_w = hw;
            }
            current_group = group_label;
        }
        let (item, w) = completion_list_item(c);
        if w > max_w {
            max_w = w;
        }
        items.push(item);
        selectable_to_display.push(items.len() - 1);
    }

    let sel = palette_selected.min(cs.completions.len().saturating_sub(1));
    let display_row = selectable_to_display.get(sel).copied().unwrap_or(0);
    (items, display_row, max_w.max(20))
}

/// Build a flat list of completion items (no group headers).
pub(super) fn build_flat_items(
    cs: &CompletionState,
    palette_selected: usize,
) -> (Vec<ListItem<'_>>, usize, usize) {
    let mut max_w = 0usize;
    let items: Vec<ListItem> = cs
        .completions
        .iter()
        .map(|c| {
            let (item, w) = completion_list_item(c);
            if w > max_w {
                max_w = w;
            }
            item
        })
        .collect();
    let sel = palette_selected.min(cs.completions.len().saturating_sub(1));
    (items, sel, max_w.max(20))
}

/// Create a single completion `ListItem` from an `ArgCompletion`.
fn completion_list_item(c: &ArgCompletion) -> (ListItem<'_>, usize) {
    let mut spans = vec![Span::styled(
        c.value.clone(),
        Style::default().fg(Color::Cyan),
    )];
    let mut w = c.value.len();
    if !c.description.is_empty() {
        spans.push(Span::styled(
            format!("  {}", c.description),
            Style::default().fg(Color::DarkGray),
        ));
        w += 2 + c.description.len();
    }
    (ListItem::new(Line::from(spans)), w)
}

// ===== Scion name completion =====

/// Filter pre-computed scion completions by a partial name prefix.
fn filter_scion_completions(partial: &str, completions: &[ArgCompletion]) -> CompletionState {
    let partial_lower = partial.to_ascii_lowercase();
    let filtered: Vec<ArgCompletion> = completions
        .iter()
        .filter(|c| c.value.to_ascii_lowercase().starts_with(&partial_lower))
        .cloned()
        .collect();
    // A name is required: show hint when completions are empty and no text typed yet
    let needs_more = partial.is_empty() && filtered.is_empty();
    CompletionState {
        completions: filtered,
        requires_more_input: needs_more,
        arg_hint: if needs_more {
            Some("<name>".to_string())
        } else {
            None
        },
    }
}

// ===== Helpers =====

/// Extract the command prefix from a buffer (everything up to and including the last space).
///
/// For `"run build"` returns `"run "`, for `"run "` returns `"run "`.
pub(super) fn extract_command_prefix(buffer: &str) -> &str {
    if let Some(pos) = buffer.rfind(' ') {
        &buffer[..=pos]
    } else {
        buffer
    }
}

/// Compute the ghost hint suffix for an arg completion.
///
/// Given buffer `"run bu"` and completion value `"build"`, returns `Some("ild")`.
pub(super) fn ghost_hint_suffix(buffer: &str, completion_value: &str) -> Option<String> {
    let rest = if let Some(pos) = buffer.rfind(' ') {
        buffer[pos + 1..].trim_start()
    } else {
        return None;
    };

    if completion_value
        .to_ascii_lowercase()
        .starts_with(&rest.to_ascii_lowercase())
        && completion_value.len() > rest.len()
    {
        Some(completion_value[rest.len()..].to_string())
    } else {
        None
    }
}

/// Return the longest common prefix of a set of strings.
fn longest_common_prefix(strings: &[&str]) -> String {
    let Some(first) = strings.first() else {
        return String::new();
    };

    let mut prefix_len = first.len();
    for s in &strings[1..] {
        prefix_len = first
            .chars()
            .zip(s.chars())
            .take(prefix_len)
            .take_while(|(a, b)| a == b)
            .count();
        if prefix_len == 0 {
            break;
        }
    }

    first.chars().take(prefix_len).collect()
}
