//! Repository list rendering and path formatting utilities.

use super::{
    ActivePane, Alignment, App, Block, Borders, Color, FileChangeStatus, Line, List, ListItem,
    Modifier, Paragraph, Rect, RepoDetailProvider, RepoRegistry, RepoStatus, Span, Style,
    UnicodeWidthStr,
};

/// Extract the basename (final component) from a path.
///
/// # Examples
/// - `/home/user/src/graft` → `graft`
/// - `~/projects/repo` → `repo`
/// - `/tmp` → `tmp`
pub(crate) fn extract_basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Compact a path to fit within a maximum width using abbreviation strategies.
///
/// Applies transformations in order:
/// 1. Home directory shown as "~" (e.g., `/home/user` → `~`)
/// 2. Parent directory components abbreviated to first character (preserves last 2 components)
/// 3. Fallback to prefix truncation with "[..]" if still too wide
pub(crate) fn compact_path(path: &str, max_width: usize) -> String {
    // First, collapse home directory to tilde
    let tilde_path = if let Ok(home) = std::env::var("HOME") {
        if path.starts_with(&home) {
            path.replacen(&home, "~", 1)
        } else {
            path.to_string()
        }
    } else {
        path.to_string()
    };

    let current_width = tilde_path.width();

    // If it fits already, we're done
    if current_width <= max_width {
        return tilde_path.clone();
    }

    // Split into components
    let parts: Vec<&str> = tilde_path.split('/').collect();

    // If we have fewer than 3 components, just truncate with prefix
    if parts.len() < 3 {
        return prefix_truncate(&tilde_path, max_width);
    }

    // Fish-style abbreviation: abbreviate all but last 2 components
    let preserve_count = 2;
    let mut abbreviated = String::new();

    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            abbreviated.push('/');
        }

        // Preserve last N components and empty parts (for leading /)
        if i >= parts.len() - preserve_count || part.is_empty() {
            abbreviated.push_str(part);
        } else {
            // Abbreviate to first character (or empty if component is empty)
            if let Some(first_char) = part.chars().next() {
                abbreviated.push(first_char);
            }
        }
    }

    // Check if abbreviated version fits
    if abbreviated.width() <= max_width {
        return abbreviated;
    }

    // Last resort: prefix truncation
    prefix_truncate(&abbreviated, max_width)
}

/// Truncate a string from the start with "[..]" prefix.
fn prefix_truncate(s: &str, max_width: usize) -> String {
    const PREFIX: &str = "[..]";
    const PREFIX_WIDTH: usize = 4; // "[..]".width()

    if s.width() <= max_width {
        return s.to_string();
    }

    if max_width <= PREFIX_WIDTH {
        // Not enough room for prefix, just take what we can
        return s.chars().take(max_width).collect();
    }

    let target_width = max_width - PREFIX_WIDTH;
    let mut truncated = String::from(PREFIX);
    let mut current_width = 0;

    // Take characters from the end that fit within target_width
    for ch in s.chars().rev() {
        let ch_width = UnicodeWidthStr::width(ch.to_string().as_str());
        if current_width + ch_width > target_width {
            break;
        }
        current_width += ch_width;
        truncated.insert(PREFIX_WIDTH, ch);
    }

    truncated
}

/// Map a `FileChangeStatus` to an indicator character and color.
pub(crate) fn format_file_change_indicator(status: &FileChangeStatus) -> (&'static str, Color) {
    match status {
        FileChangeStatus::Modified => ("M", Color::Yellow),
        FileChangeStatus::Added => ("A", Color::Green),
        FileChangeStatus::Deleted => ("D", Color::Red),
        FileChangeStatus::Renamed => ("R", Color::Cyan),
        FileChangeStatus::Copied => ("C", Color::Cyan),
        FileChangeStatus::Unknown => ("?", Color::Gray),
    }
}

/// Format a repository status line for display in the TUI.
///
/// Returns `Line<'static>` because all data is owned (no borrowing from input parameters).
/// The 'static lifetime indicates the Line owns its data, not that it's statically allocated.
#[allow(
    clippy::too_many_lines,
    clippy::single_match_else,
    clippy::needless_pass_by_value
)]
pub(crate) fn format_repo_line(
    path: String,
    status: Option<&RepoStatus>,
    pane_width: u16,
) -> Line<'static> {
    match status {
        Some(status) => {
            // Check for error first
            if let Some(error_msg) = &status.error {
                let error_text = format!("[error: {error_msg}]");
                let overhead = 2 + 1 + error_text.width() + 3;
                let max_path_width = (pane_width as usize).saturating_sub(overhead);
                let compacted_path = compact_path(&path, max_path_width);

                let error_color = if error_msg.contains("timed out") {
                    Color::Yellow
                } else {
                    Color::Red
                };

                Line::from(vec![
                    Span::styled(compacted_path, Style::default().fg(Color::White)),
                    Span::raw(" "),
                    Span::styled(error_text, Style::default().fg(error_color)),
                ])
            } else {
                // Build status indicators first to calculate their width
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

                let ahead_text = status
                    .ahead
                    .filter(|&n| n > 0)
                    .map(|n| format!("↑{n}"))
                    .unwrap_or_default();

                let behind_text = status
                    .behind
                    .filter(|&n| n > 0)
                    .map(|n| format!("↓{n}"))
                    .unwrap_or_default();

                // Calculate status width WITHOUT branch (for tight space fallback)
                let mut minimal_status_width = 1 + 1; // space + dirty
                if !ahead_text.is_empty() {
                    minimal_status_width += 1 + ahead_text.width(); // space + ahead
                }
                if !behind_text.is_empty() {
                    minimal_status_width += 1 + behind_text.width(); // space + behind
                }

                if pane_width < 15 {
                    // Very tight: show just basename
                    let basename = extract_basename(&path);
                    let overhead = 2 + minimal_status_width + 3;
                    let max_basename_width = (pane_width as usize).saturating_sub(overhead);

                    let display_name = if basename.width() > max_basename_width {
                        &basename[..max_basename_width.min(basename.len())]
                    } else {
                        basename
                    };

                    let mut spans = vec![
                        Span::styled(display_name.to_string(), Style::default().fg(Color::White)),
                        Span::raw(" "),
                        Span::styled(dirty_indicator, Style::default().fg(dirty_color)),
                    ];

                    if !ahead_text.is_empty() {
                        spans.push(Span::raw(" "));
                        spans.push(Span::styled(ahead_text, Style::default().fg(Color::Green)));
                    }

                    if !behind_text.is_empty() {
                        spans.push(Span::raw(" "));
                        spans.push(Span::styled(behind_text, Style::default().fg(Color::Red)));
                    }

                    Line::from(spans)
                } else {
                    // Calculate status width WITH branch
                    let full_status_width = 1 + branch.width() + minimal_status_width;

                    let overhead_with_branch = 2 + full_status_width + 3;
                    let max_path_width_with_branch =
                        (pane_width as usize).saturating_sub(overhead_with_branch);
                    let compacted_path_with_branch =
                        compact_path(&path, max_path_width_with_branch);

                    let use_branch = !compacted_path_with_branch.starts_with("[..]")
                        && compacted_path_with_branch.width() >= 8;

                    if use_branch {
                        let mut spans = vec![
                            Span::styled(
                                compacted_path_with_branch,
                                Style::default().fg(Color::White),
                            ),
                            Span::raw(" "),
                            Span::styled(branch, Style::default().fg(Color::Cyan)),
                            Span::raw(" "),
                            Span::styled(dirty_indicator, Style::default().fg(dirty_color)),
                        ];

                        if !ahead_text.is_empty() {
                            spans.push(Span::raw(" "));
                            spans.push(Span::styled(ahead_text, Style::default().fg(Color::Green)));
                        }

                        if !behind_text.is_empty() {
                            spans.push(Span::raw(" "));
                            spans.push(Span::styled(behind_text, Style::default().fg(Color::Red)));
                        }

                        Line::from(spans)
                    } else {
                        let overhead_without_branch = 2 + minimal_status_width + 3;
                        let max_path_width =
                            (pane_width as usize).saturating_sub(overhead_without_branch);
                        let compacted_path = compact_path(&path, max_path_width);

                        let mut spans = vec![
                            Span::styled(compacted_path, Style::default().fg(Color::White)),
                            Span::raw(" "),
                            Span::styled(dirty_indicator, Style::default().fg(dirty_color)),
                        ];

                        if !ahead_text.is_empty() {
                            spans.push(Span::raw(" "));
                            spans.push(Span::styled(ahead_text, Style::default().fg(Color::Green)));
                        }

                        if !behind_text.is_empty() {
                            spans.push(Span::raw(" "));
                            spans.push(Span::styled(behind_text, Style::default().fg(Color::Red)));
                        }

                        Line::from(spans)
                    }
                }
            }
        }
        None => {
            let loading_text = "[loading...]";
            let overhead = 2 + 1 + loading_text.width() + 3;
            let max_path_width = (pane_width as usize).saturating_sub(overhead);
            let compacted_path = compact_path(&path, max_path_width);

            Line::from(vec![
                Span::styled(compacted_path, Style::default().fg(Color::White)),
                Span::raw(" "),
                Span::styled(loading_text, Style::default().fg(Color::Gray)),
            ])
        }
    }
}

impl<R: RepoRegistry, D: RepoDetailProvider> App<R, D> {
    /// Render the repository list in the left pane.
    pub(super) fn render_repo_list(&mut self, frame: &mut ratatui::Frame, area: Rect) {
        let repos = self.registry.list_repos();
        let pane_width = area.width;

        let list_border_color = if self.active_pane == ActivePane::RepoList {
            Color::Cyan
        } else {
            Color::DarkGray
        };

        let title = format!("Grove: {}", self.workspace_name);

        if repos.is_empty() {
            let empty_message = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "No repositories configured",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Edit your workspace config to add repositories:",
                    Style::default().fg(Color::Gray),
                )),
                Line::from(Span::styled(
                    "  ~/.config/grove/workspace.yaml",
                    Style::default().fg(Color::Cyan),
                )),
                Line::from(""),
                Line::from(Span::styled("Example:", Style::default().fg(Color::Gray))),
                Line::from(Span::styled(
                    "  repositories:",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(Span::styled(
                    "    - path: ~/src/my-project",
                    Style::default().fg(Color::DarkGray),
                )),
            ];

            let empty_widget = Paragraph::new(empty_message)
                .block(
                    Block::default()
                        .title(title)
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(list_border_color)),
                )
                .alignment(Alignment::Center);

            frame.render_widget(empty_widget, area);
        } else {
            let items: Vec<ListItem> = repos
                .iter()
                .map(|repo_path| {
                    let status = self.registry.get_status(repo_path);
                    let line = format_repo_line(
                        repo_path.as_path().display().to_string(),
                        status,
                        pane_width,
                    );
                    ListItem::new(line)
                })
                .collect();

            let list = List::new(items)
                .block(
                    Block::default()
                        .title(title)
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(list_border_color)),
                )
                .highlight_style(
                    Style::default()
                        .bg(Color::Rgb(40, 40, 50))
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("▶ ");

            frame.render_stateful_widget(list, area, &mut self.list_state);
        }
    }
}
