//! Context-sensitive keybinding hint bar.

use super::{ActivePane, App, DetailTab, RepoDetailProvider, RepoRegistry};

/// A keybinding hint for the status bar.
pub(super) struct KeyHint {
    pub key: &'static str,
    pub action: &'static str,
}

impl<R: RepoRegistry, D: RepoDetailProvider> App<R, D> {
    /// Return context-sensitive key hints based on current pane and tab.
    #[allow(clippy::too_many_lines)]
    pub(super) fn current_hints(&self) -> Vec<KeyHint> {
        match self.active_pane {
            ActivePane::RepoList => vec![
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
            ActivePane::Detail => match self.active_tab {
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
            ActivePane::Help => vec![KeyHint {
                key: "any key",
                action: "close",
            }],
            ActivePane::ArgumentInput => vec![
                KeyHint {
                    key: "Enter",
                    action: "run",
                },
                KeyHint {
                    key: "Esc",
                    action: "cancel",
                },
            ],
            ActivePane::CommandOutput => vec![
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
