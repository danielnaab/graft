//! Context-sensitive keybinding hint bar.

use super::{App, DetailItem, RepoDetailProvider, RepoRegistry, View};

/// A keybinding hint for the status bar.
pub(super) struct KeyHint {
    pub key: &'static str,
    pub action: &'static str,
}

impl<R: RepoRegistry, D: RepoDetailProvider> App<R, D> {
    /// Return context-sensitive key hints based on current view.
    #[allow(clippy::too_many_lines)]
    pub(super) fn current_hints(&self) -> Vec<KeyHint> {
        // Approval overlay — show its hints regardless of view stack.
        if self.approval_overlay.is_some() {
            return vec![
                KeyHint {
                    key: "a",
                    action: "approve",
                },
                KeyHint {
                    key: "r",
                    action: "reject",
                },
                KeyHint {
                    key: "Esc",
                    action: "cancel",
                },
            ];
        }

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
            View::RepoDetail(_) => {
                let mut hints = vec![KeyHint {
                    key: "j/k",
                    action: "navigate",
                }];
                match self.current_detail_item() {
                    Some(DetailItem::Command(_)) => {
                        hints.push(KeyHint {
                            key: "Enter",
                            action: "run",
                        });
                    }
                    Some(DetailItem::StateQuery(_)) => {
                        hints.push(KeyHint {
                            key: "Enter",
                            action: "expand/collapse",
                        });
                    }
                    Some(DetailItem::Run(_)) => {
                        hints.push(KeyHint {
                            key: "Enter",
                            action: "view log",
                        });
                    }
                    Some(DetailItem::RunState(idx)) => {
                        let idx = *idx;
                        if self.is_pending_checkpoint(idx) {
                            hints.push(KeyHint {
                                key: "Enter",
                                action: "review checkpoint",
                            });
                        } else {
                            hints.push(KeyHint {
                                key: "Enter",
                                action: "expand/collapse",
                            });
                        }
                    }
                    _ => {}
                }
                hints.extend([
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
                ]);
                hints
            }
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
