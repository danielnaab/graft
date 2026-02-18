//! Context-sensitive keybinding hint bar.

use super::{ActivePane, App, DetailTab, RepoDetailProvider, RepoRegistry, View};

/// A keybinding hint for the status bar.
pub(super) struct KeyHint {
    pub key: &'static str,
    pub action: &'static str,
}

impl<R: RepoRegistry, D: RepoDetailProvider> App<R, D> {
    /// Return context-sensitive key hints based on current view and tab.
    #[allow(clippy::too_many_lines)]
    pub(super) fn current_hints(&self) -> Vec<KeyHint> {
        // ArgumentInput is an overlay — show its hints regardless of view stack.
        if self.active_pane == ActivePane::ArgumentInput {
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
                    key: "s",
                    action: "state",
                },
                KeyHint {
                    key: "x",
                    action: "commands",
                },
                KeyHint {
                    key: "r",
                    action: "refresh",
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
            View::RepoDetail(_) => match self.active_tab {
                DetailTab::Changes => vec![
                    KeyHint {
                        key: "j/k",
                        action: "scroll",
                    },
                    KeyHint {
                        key: "1-3",
                        action: "tab",
                    },
                    KeyHint {
                        key: "s",
                        action: "state",
                    },
                    KeyHint {
                        key: "x",
                        action: "commands",
                    },
                    KeyHint {
                        key: "Tab",
                        action: "repos",
                    },
                    KeyHint {
                        key: "q",
                        action: "back",
                    },
                ],
                DetailTab::State => vec![
                    KeyHint {
                        key: "j/k",
                        action: "navigate",
                    },
                    KeyHint {
                        key: "r",
                        action: "refresh",
                    },
                    KeyHint {
                        key: "1-3",
                        action: "tab",
                    },
                    KeyHint {
                        key: "Tab",
                        action: "repos",
                    },
                    KeyHint {
                        key: "q",
                        action: "back",
                    },
                ],
                DetailTab::Commands => vec![
                    KeyHint {
                        key: "j/k",
                        action: "navigate",
                    },
                    KeyHint {
                        key: "Enter",
                        action: "run",
                    },
                    KeyHint {
                        key: "1-3",
                        action: "tab",
                    },
                    KeyHint {
                        key: "Tab",
                        action: "repos",
                    },
                    KeyHint {
                        key: "q",
                        action: "back",
                    },
                ],
            },
            View::Help => vec![KeyHint {
                key: "any key",
                action: "close",
            }],
            View::CommandOutput => vec![
                KeyHint {
                    key: "j/k",
                    action: "scroll",
                },
                KeyHint {
                    key: "q",
                    action: "close",
                },
            ],
        }
    }
}
