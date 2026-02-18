//! Context-sensitive keybinding hint bar.

use super::{App, RepoDetailProvider, RepoRegistry, View};

/// A keybinding hint for the status bar.
pub(super) struct KeyHint {
    pub key: &'static str,
    pub action: &'static str,
}

impl<R: RepoRegistry, D: RepoDetailProvider> App<R, D> {
    /// Return context-sensitive key hints based on current view.
    pub(super) fn current_hints(&self) -> Vec<KeyHint> {
        // ArgumentInput is an overlay — show its hints regardless of view stack.
        if self.argument_input.is_some() {
            return vec![
                KeyHint {
                    key: "Enter",
                    action: "run",
                },
                KeyHint {
                    key: "Esc",
                    action: "cancel",
                },
            ];
        }

        match self.current_view() {
            View::Dashboard => vec![
                KeyHint {
                    key: "j/k",
                    action: "navigate",
                },
                KeyHint {
                    key: "Enter",
                    action: "details",
                },
                KeyHint {
                    key: "r",
                    action: "refresh",
                },
                KeyHint {
                    key: ":",
                    action: "command",
                },
                KeyHint {
                    key: "?",
                    action: "help",
                },
                KeyHint {
                    key: "q",
                    action: "quit",
                },
            ],
            View::RepoDetail(_) => vec![
                KeyHint {
                    key: "j/k",
                    action: "scroll",
                },
                KeyHint {
                    key: "Enter",
                    action: "run command",
                },
                KeyHint {
                    key: "r",
                    action: "refresh state",
                },
                KeyHint {
                    key: ":",
                    action: "command",
                },
                KeyHint {
                    key: "?",
                    action: "help",
                },
                KeyHint {
                    key: "q",
                    action: "back",
                },
            ],
            View::Help => vec![
                KeyHint {
                    key: "q",
                    action: "close",
                },
                KeyHint {
                    key: "Esc",
                    action: "home",
                },
            ],
            View::CommandOutput => vec![
                KeyHint {
                    key: "j/k",
                    action: "scroll",
                },
                KeyHint {
                    key: "q",
                    action: "close",
                },
                KeyHint {
                    key: "Esc",
                    action: "home",
                },
                KeyHint {
                    key: ":",
                    action: "command",
                },
            ],
        }
    }
}
