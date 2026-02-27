//! Path formatting and display utilities extracted from the repo list renderer.
#![allow(dead_code)]

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use unicode_width::UnicodeWidthStr;

use grove_core::{FileChangeStatus, RepoStatus};

/// Extract the basename (final component) from a path.
///
/// # Examples
/// - `/home/user/src/graft` -> `graft`
/// - `~/projects/repo` -> `repo`
/// - `/tmp` -> `tmp`
pub(crate) fn extract_basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Compact a path to fit within a maximum width using abbreviation strategies.
///
/// Applies transformations in order:
/// 1. Home directory shown as "~" (e.g., `/home/user` -> `~`)
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

/// Tree connector prefix for nested graft deps (depth > 0).
const INDENT_PREFIX: &str = "  \u{251c}\u{2500} ";
/// Display columns consumed by `INDENT_PREFIX` (2 spaces + box-draw + dash + 1 space = 5 cols).
const INDENT_WIDTH: u16 = 5;

/// Format a repository status line for display in the TUI.
///
/// `depth` and `ahead_of_lock` come from `RepoRegistry::get_display_meta()` rather
/// than from `RepoStatus`, keeping git-status data separate from display metadata.
#[allow(
    clippy::too_many_lines,
    clippy::single_match_else,
    clippy::needless_pass_by_value
)]
pub(crate) fn format_repo_line(
    path: String,
    status: Option<&RepoStatus>,
    pane_width: u16,
    depth: usize,
    ahead_of_lock: Option<usize>,
) -> Line<'static> {
    // Verify constant is in sync with the prefix string.
    debug_assert_eq!(
        INDENT_PREFIX.width(),
        INDENT_WIDTH as usize,
        "INDENT_WIDTH is out of sync with INDENT_PREFIX display width"
    );

    let effective_width = if depth > 0 {
        pane_width.saturating_sub(INDENT_WIDTH)
    } else {
        pane_width
    };

    let mut line = format_repo_line_inner(&path, status, effective_width, ahead_of_lock);

    if depth > 0 {
        line.spans.insert(
            0,
            Span::styled(INDENT_PREFIX, Style::default().add_modifier(Modifier::DIM)),
        );
    }

    line
}

/// Inner formatting logic, operating on the effective (post-indent) pane width.
#[allow(clippy::too_many_lines, clippy::single_match_else)]
fn format_repo_line_inner(
    path: &str,
    status: Option<&RepoStatus>,
    pane_width: u16,
    ahead_of_lock: Option<usize>,
) -> Line<'static> {
    match status {
        Some(status) => {
            if let Some(error_msg) = &status.error {
                let error_text = format!("[error: {error_msg}]");
                let overhead = 2 + 1 + error_text.width() + 3;
                let max_path_width = (pane_width as usize).saturating_sub(overhead);
                let compacted_path = compact_path(path, max_path_width);

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
                let branch = status
                    .branch
                    .as_ref()
                    .map_or_else(|| "[detached]".to_string(), |b| format!("[{b}]"));

                let dirty_indicator = if status.is_dirty {
                    "\u{25cf}"
                } else {
                    "\u{25cb}"
                };
                let dirty_color = if status.is_dirty {
                    Color::Yellow
                } else {
                    Color::Green
                };

                let ahead_text = status
                    .ahead
                    .filter(|&n| n > 0)
                    .map(|n| format!("\u{2191}{n}"))
                    .unwrap_or_default();

                let behind_text = status
                    .behind
                    .filter(|&n| n > 0)
                    .map(|n| format!("\u{2193}{n}"))
                    .unwrap_or_default();

                let lock_text = match ahead_of_lock {
                    Some(n) if n > 0 => format!("\u{229b}+{n}"),
                    _ => String::new(),
                };

                let mut minimal_status_width = 1 + 1;
                if !ahead_text.is_empty() {
                    minimal_status_width += 1 + ahead_text.width();
                }
                if !behind_text.is_empty() {
                    minimal_status_width += 1 + behind_text.width();
                }
                if !lock_text.is_empty() {
                    minimal_status_width += 1 + lock_text.width();
                }

                if pane_width < 15 {
                    let basename = extract_basename(path);
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
                    if !lock_text.is_empty() {
                        spans.push(Span::raw(" "));
                        spans.push(Span::styled(lock_text, Style::default().fg(Color::Yellow)));
                    }

                    Line::from(spans)
                } else {
                    let full_status_width = 1 + branch.width() + minimal_status_width;
                    let overhead_with_branch = 2 + full_status_width + 3;
                    let max_path_width_with_branch =
                        (pane_width as usize).saturating_sub(overhead_with_branch);
                    let compacted_path_with_branch = compact_path(path, max_path_width_with_branch);

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
                        if !lock_text.is_empty() {
                            spans.push(Span::raw(" "));
                            spans.push(Span::styled(lock_text, Style::default().fg(Color::Yellow)));
                        }

                        Line::from(spans)
                    } else {
                        let overhead_without_branch = 2 + minimal_status_width + 3;
                        let max_path_width =
                            (pane_width as usize).saturating_sub(overhead_without_branch);
                        let compacted_path = compact_path(path, max_path_width);

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
                        if !lock_text.is_empty() {
                            spans.push(Span::raw(" "));
                            spans.push(Span::styled(lock_text, Style::default().fg(Color::Yellow)));
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
            let compacted_path = compact_path(path, max_path_width);

            Line::from(vec![
                Span::styled(compacted_path, Style::default().fg(Color::White)),
                Span::raw(" "),
                Span::styled(loading_text, Style::default().fg(Color::Gray)),
            ])
        }
    }
}
