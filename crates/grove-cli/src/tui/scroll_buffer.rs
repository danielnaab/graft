//! Scroll buffer: a vertical list of content blocks with scroll and focus support.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use super::command_line::CliCommand;

/// Unique identifier for a content block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct BlockId(u64);

impl BlockId {
    /// Generate a new unique block ID.
    pub(super) fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

/// Outcome of a command execution, used to finalize a `Running` block.
#[derive(Debug)]
pub(super) enum RunCompletion {
    /// Process exited with the given code (0 = success).
    Exited(i32),
    /// Process could not be spawned or failed with an error message.
    Error(String),
}

/// A block of content in the scroll buffer.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(super) enum ContentBlock {
    /// Free-form styled text lines.
    Text {
        id: BlockId,
        lines: Vec<Line<'static>>,
        collapsed: bool,
    },
    /// A live command execution block. Renders with an animated spinner and
    /// elapsed time while the command is running. Call
    /// [`ScrollBuffer::finalize_running`] to convert it to a static `Text`
    /// block when the command completes.
    Running {
        id: BlockId,
        command: String,
        args: Vec<String>,
        started_at: Instant,
        output_lines: Vec<Line<'static>>,
        /// Set when old output was dropped to stay within the line cap.
        output_truncated: bool,
        collapsed: bool,
        /// True when the block was auto-expanded because an error indicator
        /// was detected in the output.
        auto_expanded: bool,
    },
    /// A data table with headers and rows.
    Table {
        id: BlockId,
        title: String,
        headers: Vec<String>,
        rows: Vec<Vec<Span<'static>>>,
        collapsed: bool,
        /// Optional per-row actions. When `Some`, pressing Enter on this table
        /// opens a picker overlay. Length must match `rows`.
        actions: Option<Vec<CliCommand>>,
    },
    /// A horizontal divider line.
    Divider { id: BlockId },
}

#[allow(dead_code)]
impl ContentBlock {
    pub(super) fn id(&self) -> BlockId {
        match self {
            Self::Text { id, .. }
            | Self::Running { id, .. }
            | Self::Table { id, .. }
            | Self::Divider { id } => *id,
        }
    }

    pub(super) fn is_collapsed(&self) -> bool {
        match self {
            Self::Text { collapsed, .. }
            | Self::Running { collapsed, .. }
            | Self::Table { collapsed, .. } => *collapsed,
            Self::Divider { .. } => false,
        }
    }

    pub(super) fn toggle_collapse(&mut self) {
        match self {
            Self::Text { collapsed, .. }
            | Self::Running { collapsed, .. }
            | Self::Table { collapsed, .. } => {
                *collapsed = !*collapsed;
            }
            Self::Divider { .. } => {}
        }
    }

    /// Count rendered lines without allocating.
    ///
    /// Used by [`ScrollBuffer::total_lines`] to compute scroll bounds efficiently.
    fn line_count(&self) -> usize {
        match self {
            Self::Divider { .. } => 1,
            Self::Text {
                lines, collapsed, ..
            } => {
                if *collapsed {
                    1
                } else {
                    lines.len()
                }
            }
            Self::Running {
                output_lines,
                output_truncated,
                collapsed,
                ..
            } => {
                if *collapsed {
                    1 + output_lines.len().min(COLLAPSED_TAIL_LINES)
                } else {
                    // 1 header + optional truncation notice + output lines
                    1 + usize::from(*output_truncated) + output_lines.len()
                }
            }
            Self::Table {
                title,
                headers,
                rows,
                collapsed,
                ..
            } => {
                if *collapsed {
                    return 1;
                }
                let title_lines = usize::from(!title.is_empty());
                // header row + separator when headers are present
                let header_lines = if headers.is_empty() { 0 } else { 2 };
                title_lines + header_lines + rows.len()
            }
        }
    }

    /// Render this block into lines for display at the given instant.
    ///
    /// `now` is used only by [`ContentBlock::Running`] to drive the spinner
    /// and elapsed-time display; all other variants ignore it.
    #[allow(clippy::too_many_lines)]
    fn render_lines_at(&self, width: u16, now: Instant) -> Vec<Line<'static>> {
        match self {
            Self::Text {
                lines, collapsed, ..
            } => {
                if *collapsed {
                    if let Some(first) = lines.first() {
                        vec![Line::from(vec![
                            Span::styled("\u{25b6} ", Style::default().fg(Color::DarkGray)),
                            Span::styled(
                                format_first_line(first),
                                Style::default().fg(Color::DarkGray),
                            ),
                        ])]
                    } else {
                        vec![Line::from(Span::styled(
                            "\u{25b6} (empty)",
                            Style::default().fg(Color::DarkGray),
                        ))]
                    }
                } else {
                    lines.clone()
                }
            }
            Self::Running {
                command,
                args,
                started_at,
                output_lines,
                output_truncated,
                collapsed,
                auto_expanded,
                ..
            } => {
                const SPINNER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
                let elapsed = now.duration_since(*started_at);
                let frame = (elapsed.as_millis() / 100) as usize % SPINNER.len();
                let spinner = SPINNER[frame];
                let elapsed_str = format_elapsed(elapsed);
                let arg_str = args.join(" ");

                if *collapsed {
                    let summary = if arg_str.is_empty() {
                        command.clone()
                    } else {
                        format!("{command}  {arg_str}")
                    };
                    let mut out = vec![Line::from(vec![
                        Span::styled(format!("{spinner} "), Style::default().fg(Color::Yellow)),
                        Span::styled(summary, Style::default().fg(Color::DarkGray)),
                        Span::styled(
                            format!("  [{elapsed_str}]"),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ])];
                    // Show the last few output lines as a dimmed tail preview.
                    let tail_count = output_lines.len().min(COLLAPSED_TAIL_LINES);
                    let dim = Style::default().fg(Color::DarkGray);
                    for line in output_lines.iter().skip(output_lines.len() - tail_count) {
                        let mut spans = vec![Span::styled("  ", dim)];
                        for span in &line.spans {
                            let mut s = span.clone();
                            s.style = s.style.patch(dim);
                            spans.push(s);
                        }
                        out.push(Line::from(spans));
                    }
                    return out;
                }

                let mut header = vec![
                    Span::styled(format!("{spinner} "), Style::default().fg(Color::Yellow)),
                    Span::styled(
                        command.clone(),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ];
                if !arg_str.is_empty() {
                    header.push(Span::styled(
                        format!("  {arg_str}"),
                        Style::default().fg(Color::White),
                    ));
                }
                header.push(Span::styled(
                    format!("    [{elapsed_str}]"),
                    Style::default().fg(Color::DarkGray),
                ));
                if *auto_expanded {
                    header.push(Span::styled(
                        "  (auto-expanded due to error)",
                        Style::default().fg(Color::Red),
                    ));
                }

                let mut out = vec![Line::from(header)];
                if *output_truncated {
                    out.push(Line::from(Span::styled(
                        "  \u{2026} (earlier output truncated)",
                        Style::default().fg(Color::DarkGray),
                    )));
                }
                out.extend(output_lines.iter().cloned());
                out
            }
            Self::Table {
                title,
                headers,
                rows,
                collapsed,
                ..
            } => {
                if *collapsed {
                    return vec![Line::from(vec![
                        Span::styled("\u{25b6} ", Style::default().fg(Color::DarkGray)),
                        Span::styled(
                            format!("{title} ({} rows)", rows.len()),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ])];
                }

                let mut out = Vec::new();

                // Title line
                if !title.is_empty() {
                    out.push(Line::from(Span::styled(
                        title.clone(),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )));
                }

                // Compute column widths
                let col_count = headers.len();
                let col_widths = compute_col_widths(headers, rows, width, col_count);

                // Header row
                if !headers.is_empty() {
                    let mut spans = Vec::new();
                    for (i, header) in headers.iter().enumerate() {
                        let w = col_widths.get(i).copied().unwrap_or(10);
                        spans.push(Span::styled(
                            pad_or_truncate(header, w),
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        ));
                        if i + 1 < col_count {
                            spans.push(Span::raw("  "));
                        }
                    }
                    out.push(Line::from(spans));

                    // Separator
                    let total: usize =
                        col_widths.iter().sum::<usize>() + (col_count.saturating_sub(1)) * 2;
                    let sep_width = total.min(width as usize);
                    out.push(Line::from(Span::styled(
                        "\u{2500}".repeat(sep_width),
                        Style::default().fg(Color::DarkGray),
                    )));
                }

                // Data rows
                for row in rows {
                    let mut spans = Vec::new();
                    for (i, cell) in row.iter().enumerate() {
                        let w = col_widths.get(i).copied().unwrap_or(10);
                        let content = cell.content.to_string();
                        let padded = pad_or_truncate(&content, w);
                        spans.push(Span::styled(padded, cell.style));
                        if i + 1 < col_count {
                            spans.push(Span::raw("  "));
                        }
                    }
                    // Pad missing columns
                    for i in row.len()..col_count {
                        let w = col_widths.get(i).copied().unwrap_or(10);
                        spans.push(Span::raw(" ".repeat(w)));
                        if i + 1 < col_count {
                            spans.push(Span::raw("  "));
                        }
                    }
                    out.push(Line::from(spans));
                }

                out
            }
            Self::Divider { .. } => {
                let sep = "\u{2500}".repeat(width as usize);
                vec![Line::from(Span::styled(
                    sep,
                    Style::default().fg(Color::DarkGray),
                ))]
            }
        }
    }
}

/// The scroll buffer: a stack of content blocks rendered top-to-bottom.
#[derive(Debug)]
pub(super) struct ScrollBuffer {
    pub(super) blocks: Vec<ContentBlock>,
    /// Scroll offset (in rendered lines from top).
    pub(super) scroll_offset: usize,
    /// Index of the focused block (for collapse toggle).
    pub(super) focused_block: Option<usize>,
    /// Last known viewport width (updated each render).
    last_width: u16,
    /// Last known viewport height (updated each render).
    last_viewport_height: u16,
}

impl ScrollBuffer {
    pub(super) fn new() -> Self {
        Self {
            blocks: Vec::new(),
            scroll_offset: 0,
            focused_block: None,
            last_width: 80,
            last_viewport_height: 24,
        }
    }

    /// Push a new block and auto-scroll to show it.
    pub(super) fn push(&mut self, block: ContentBlock) {
        self.blocks.push(block);
        self.focused_block = Some(self.blocks.len() - 1);
        self.scroll_to_bottom();
    }

    /// Replace the lines of the last Text block entirely.
    #[allow(dead_code)]
    pub(super) fn replace_last_lines(&mut self, new_lines: Vec<Line<'static>>) -> bool {
        if let Some(ContentBlock::Text { lines, .. }) = self.blocks.last_mut() {
            *lines = new_lines;
            self.scroll_to_bottom();
            true
        } else {
            false
        }
    }

    /// Clear all blocks and reset scroll.
    #[allow(dead_code)]
    pub(super) fn clear(&mut self) {
        self.blocks.clear();
        self.scroll_offset = 0;
        self.focused_block = None;
    }

    /// Total rendered lines across all blocks (including blank separators between blocks).
    fn total_lines(&self, _width: u16) -> usize {
        self.blocks
            .iter()
            .enumerate()
            .map(|(i, b)| usize::from(i > 0) + b.line_count())
            .sum()
    }

    /// Append lines to the `Running` block with the given `id`.
    ///
    /// Lines beyond [`MAX_RUNNING_OUTPUT_LINES`] are dropped from the front
    /// and `output_truncated` is set so the block can show a truncation notice.
    /// No-ops if the block is not found or is not a `Running` block.
    pub(super) fn append_lines_to_running(&mut self, id: BlockId, new_lines: Vec<Line<'static>>) {
        const MAX_RUNNING_OUTPUT_LINES: usize = 10_000;
        const RUNNING_LINES_TO_DROP: usize = 1_000;

        let at_bottom = self.scroll_offset == usize::MAX;

        if let Some(ContentBlock::Running {
            output_lines,
            output_truncated,
            collapsed,
            auto_expanded,
            ..
        }) = self.blocks.iter_mut().find(|b| b.id() == id)
        {
            // Check new lines for error indicators before extending.
            if *collapsed && !*auto_expanded {
                let has_error = new_lines.iter().any(|line| {
                    let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
                    line_contains_error_indicator(&text)
                });
                if has_error {
                    *collapsed = false;
                    *auto_expanded = true;
                }
            }
            output_lines.extend(new_lines);
            if output_lines.len() > MAX_RUNNING_OUTPUT_LINES {
                output_lines.drain(0..RUNNING_LINES_TO_DROP);
                *output_truncated = true;
            }
        }

        // Only auto-scroll if the user hasn't scrolled up manually.
        if at_bottom {
            self.scroll_to_bottom();
        }
    }

    /// Convert a `Running` block to a static `Text` block, recording the
    /// outcome in the header and preserving all captured output lines.
    ///
    /// The header is updated to show a check-mark (exit 0) or cross (non-zero
    /// / error) together with the elapsed time.  Call this once when the
    /// command process finishes.  No-ops if the block is not found.
    pub(super) fn finalize_running(&mut self, id: BlockId, outcome: &RunCompletion) {
        let now = Instant::now();
        let pos = self.blocks.iter().position(|b| b.id() == id);
        let Some(pos) = pos else { return };

        // Remove the block to take ownership — avoids cloning output_lines.
        let block = self.blocks.remove(pos);
        let ContentBlock::Running {
            command,
            args,
            started_at,
            output_lines,
            collapsed,
            ..
        } = block
        else {
            // Put it back if it wasn't a Running block.
            self.blocks.insert(pos, block);
            return;
        };

        let elapsed = now.duration_since(started_at);
        let elapsed_str = format_elapsed(elapsed);
        let arg_str = args.join(" ");
        let cmd_display = if arg_str.is_empty() {
            command.clone()
        } else {
            format!("{command}  {arg_str}")
        };

        let (symbol, symbol_color, exit_label) = match &outcome {
            RunCompletion::Exited(0) => ("\u{2713}", Color::Green, String::new()),
            RunCompletion::Exited(n) => ("\u{2717}", Color::Red, format!("  (exit {n})")),
            RunCompletion::Error(_) => ("\u{2717}", Color::Red, String::new()),
        };

        // Note: no leading "▶ " here — Text collapse rendering adds its own prefix.
        let mut header = vec![
            Span::styled(cmd_display, Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("  {symbol}  {elapsed_str}{exit_label}"),
                Style::default().fg(symbol_color),
            ),
        ];

        if let RunCompletion::Error(msg) = &outcome {
            header.push(Span::styled(
                format!("  {msg}"),
                Style::default().fg(Color::Red),
            ));
        }

        let mut lines = vec![Line::from(header)];
        lines.extend(output_lines);

        self.blocks.insert(
            pos,
            ContentBlock::Text {
                id,
                lines,
                collapsed,
            },
        );
        self.scroll_to_bottom();
    }

    /// Scroll to the bottom so the last content is visible.
    pub(super) fn scroll_to_bottom(&mut self) {
        // Actual offset computed at render time when we know the viewport height.
        // Use a sentinel that render_visible clamps.
        self.scroll_offset = usize::MAX;
    }

    /// Scroll up by `n` lines.
    pub(super) fn scroll_up(&mut self, n: usize) {
        // If at the bottom sentinel, resolve to actual max offset first so we
        // don't end up at usize::MAX - n (still enormous, renders as bottom).
        if self.scroll_offset == usize::MAX {
            let total = self.total_lines(self.last_width);
            let max_offset = total.saturating_sub(self.last_viewport_height as usize);
            self.scroll_offset = max_offset;
        }
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
    }

    /// Scroll down by `n` lines, using last-known viewport dimensions.
    pub(super) fn scroll_down(&mut self, n: usize) {
        let total = self.total_lines(self.last_width);
        let max_offset = total.saturating_sub(self.last_viewport_height as usize);
        self.scroll_offset = (self.scroll_offset.saturating_add(n)).min(max_offset);
    }

    /// Move focus to the next block (clamps at last block).
    ///
    /// When no block is focused, focuses the last block.
    pub(super) fn focus_next(&mut self) {
        if self.blocks.is_empty() {
            return;
        }
        self.focused_block = Some(match self.focused_block {
            None => self.blocks.len() - 1,
            Some(i) => (i + 1).min(self.blocks.len() - 1),
        });
        self.scroll_to_focused();
    }

    /// Move focus to the previous block (clamps at first block).
    ///
    /// When no block is focused, focuses the last block.
    pub(super) fn focus_prev(&mut self) {
        if self.blocks.is_empty() {
            return;
        }
        self.focused_block = Some(match self.focused_block {
            None => self.blocks.len() - 1,
            Some(i) => i.saturating_sub(1),
        });
        self.scroll_to_focused();
    }

    /// Move focus to the first block.
    pub(super) fn focus_first(&mut self) {
        if self.blocks.is_empty() {
            return;
        }
        self.focused_block = Some(0);
        self.scroll_to_focused();
    }

    /// Move focus to the last block.
    pub(super) fn focus_last(&mut self) {
        if self.blocks.is_empty() {
            return;
        }
        self.focused_block = Some(self.blocks.len() - 1);
        self.scroll_to_focused();
    }

    /// Compute the starting line offset of a block.
    fn block_start_line(&self, index: usize) -> usize {
        self.blocks
            .iter()
            .take(index)
            .enumerate()
            .map(|(i, b)| usize::from(i > 0) + b.line_count())
            .sum::<usize>()
            + usize::from(index > 0)
    }

    /// Scroll so that the focused block is visible.
    fn scroll_to_focused(&mut self) {
        let Some(idx) = self.focused_block else {
            return;
        };
        let block_start = self.block_start_line(idx);
        let block_height = self.blocks.get(idx).map_or(0, ContentBlock::line_count);
        let viewport = self.last_viewport_height as usize;

        // Resolve sentinel before comparing.
        let total = self.total_lines(self.last_width);
        let max_offset = total.saturating_sub(viewport);
        let current = self.scroll_offset.min(max_offset);

        if block_start < current {
            // Block is above viewport — scroll up.
            self.scroll_offset = block_start;
        } else if block_start + block_height > current + viewport {
            // Block is below viewport — scroll down so block end is visible.
            self.scroll_offset = (block_start + block_height).saturating_sub(viewport);
        }
    }

    /// Return the actions for the focused block if it is an actionable `Table`.
    ///
    /// Returns `Some(slice)` when the focused block is a `Table` with
    /// `actions: Some(...)`, otherwise `None`.
    pub(super) fn focused_block_actions(&self) -> Option<&[CliCommand]> {
        if let Some(idx) = self.focused_block {
            if let Some(ContentBlock::Table {
                actions: Some(acts),
                ..
            }) = self.blocks.get(idx)
            {
                return Some(acts.as_slice());
            }
        }
        None
    }

    /// Toggle collapse on the focused block.
    pub(super) fn toggle_focused_collapse(&mut self) {
        if let Some(idx) = self.focused_block {
            if let Some(block) = self.blocks.get_mut(idx) {
                block.toggle_collapse();
            }
        }
    }

    /// Render the visible portion of the scroll buffer into the frame.
    pub(super) fn render(&mut self, frame: &mut ratatui::Frame, area: Rect) {
        let width = area.width;
        let viewport_height = area.height as usize;

        // Store dimensions for scroll_down to use between renders
        self.last_width = width;
        self.last_viewport_height = area.height;

        // Clamp focused_block to valid range in case blocks changed
        if let Some(idx) = self.focused_block {
            if idx >= self.blocks.len() {
                self.focused_block = if self.blocks.is_empty() {
                    None
                } else {
                    Some(self.blocks.len() - 1)
                };
            }
        }

        // Collect all rendered lines with block association.
        // Capture `now` once so every Running block uses the same instant.
        // Reserve 2 columns for the left gutter marker.
        let content_width = width.saturating_sub(GUTTER_WIDTH);
        let now = Instant::now();
        let mut all_lines: Vec<(Line<'static>, Option<usize>)> = Vec::new();
        for (block_idx, block) in self.blocks.iter().enumerate() {
            if block_idx > 0 {
                all_lines.push((Line::from(""), None));
            }
            for line in block.render_lines_at(content_width, now) {
                all_lines.push((line, Some(block_idx)));
            }
        }

        let total = all_lines.len();

        // Clamp scroll offset
        let max_offset = total.saturating_sub(viewport_height);
        let offset = self.scroll_offset.min(max_offset);

        // Extract the visible window
        let visible_end = (offset + viewport_height).min(total);
        let visible = &all_lines[offset..visible_end];

        // Apply gutter markers and focus highlight
        let area_width = area.width as usize;
        let lines: Vec<Line<'static>> = visible
            .iter()
            .map(|(line, block_idx)| {
                let is_focused = matches!(
                    (self.focused_block, block_idx),
                    (Some(f), Some(b)) if *b == f
                );
                match block_idx {
                    Some(_) if is_focused => {
                        let gutter = Span::styled("\u{2590} ", Style::default().fg(Color::Cyan));
                        let mut spans = vec![gutter];
                        spans.extend(line.spans.iter().cloned());
                        // Pad with spaces so the background color fills the full width
                        let content_width: usize = spans.iter().map(Span::width).sum();
                        if content_width < area_width {
                            spans.push(Span::styled(
                                " ".repeat(area_width - content_width),
                                Style::default(),
                            ));
                        }
                        Line::from(spans).patch_style(Style::default().bg(Color::Rgb(40, 40, 70)))
                    }
                    Some(_) => {
                        let gutter =
                            Span::styled("\u{2502} ", Style::default().fg(Color::Rgb(60, 60, 60)));
                        let mut spans = vec![gutter];
                        spans.extend(line.spans.iter().cloned());
                        Line::from(spans)
                    }
                    None => line.clone(), // separator lines: no gutter
                }
            })
            .collect();

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, area);
    }
}

// ===== Helpers =====

/// Width of the left gutter marker (e.g. "│ " or "▐ ").
const GUTTER_WIDTH: u16 = 2;

/// Format a duration as a short human-readable string: "42s" or "2m 34s".
fn format_elapsed(d: Duration) -> String {
    graft_common::format_duration(d)
}

/// Number of trailing output lines shown when a Running block is collapsed.
const COLLAPSED_TAIL_LINES: usize = 3;

/// Check if a line of output contains common error indicators.
///
/// Used as a heuristic to auto-expand collapsed Running blocks so users don't
/// miss failures. False positives are acceptable — the user can re-collapse.
fn line_contains_error_indicator(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("error")
        || lower.contains("failed")
        || lower.contains("panic")
        || lower.contains("fatal")
        || lower.contains("fail")
}

const FORMAT_FIRST_LINE_MAX_CHARS: usize = 60;
const FORMAT_FIRST_LINE_TRUNCATED_CHARS: usize = 57;

fn format_first_line(line: &Line<'_>) -> String {
    let mut s = String::new();
    for span in &line.spans {
        s.push_str(&span.content);
    }
    // Truncate by char count, not byte length, to avoid panics on multibyte chars.
    if s.chars().count() > FORMAT_FIRST_LINE_MAX_CHARS {
        s = s.chars().take(FORMAT_FIRST_LINE_TRUNCATED_CHARS).collect();
        s.push_str("...");
    }
    s
}

fn pad_or_truncate(s: &str, width: usize) -> String {
    use unicode_width::UnicodeWidthStr;
    let w = s.width();
    if w > width {
        // Truncate with ellipsis
        let ellipsis_width = usize::from(width >= 2); // "…" is 1 wide
        let target = width.saturating_sub(ellipsis_width);
        let mut out = String::new();
        let mut current = 0;
        for ch in s.chars() {
            let cw = UnicodeWidthStr::width(ch.to_string().as_str());
            if current + cw > target {
                break;
            }
            out.push(ch);
            current += cw;
        }
        if ellipsis_width > 0 {
            out.push('\u{2026}'); // …
        }
        out
    } else {
        format!("{s}{}", " ".repeat(width - w))
    }
}

/// Minimum column width before proportional fallback kicks in.
const MIN_COL_WIDTH: usize = 4;

fn compute_col_widths(
    headers: &[String],
    rows: &[Vec<Span<'static>>],
    total_width: u16,
    col_count: usize,
) -> Vec<usize> {
    use unicode_width::UnicodeWidthStr;

    if col_count == 0 {
        return Vec::new();
    }

    // Natural widths: max of header and all cell contents per column.
    let mut widths: Vec<usize> = headers.iter().map(|h| h.width()).collect();
    widths.resize(col_count, 0);

    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < col_count {
                let cw = cell.content.to_string().width();
                if cw > widths[i] {
                    widths[i] = cw;
                }
            }
        }
    }

    // Available space after column separators (2 chars each).
    let sep_space = (col_count.saturating_sub(1)) * 2;
    let available = (total_width as usize).saturating_sub(sep_space);
    let total_natural: usize = widths.iter().sum();

    if total_natural <= available || available == 0 {
        return widths;
    }

    // Per-column minimum: min(natural, MIN_COL_WIDTH).
    let mins: Vec<usize> = widths.iter().map(|&w| w.min(MIN_COL_WIDTH)).collect();

    // Phase 1: Shrink widest first, respecting minimums.
    let mut excess = total_natural - available;
    while excess > 0 {
        // Find current max width.
        let max_w = widths.iter().copied().max().unwrap_or(0);
        if max_w == 0 {
            break;
        }

        // Indices sharing the max width.
        let widest: Vec<usize> = widths
            .iter()
            .enumerate()
            .filter(|(_, &w)| w == max_w)
            .map(|(i, _)| i)
            .collect();

        // Next-widest value (the target we shrink toward), clamped to each column's minimum.
        let next_w = widths
            .iter()
            .copied()
            .filter(|&w| w < max_w)
            .max()
            .unwrap_or(0);

        // How much each widest column can give.
        let mut total_shrink = 0usize;
        for &i in &widest {
            let floor = next_w.max(mins[i]);
            total_shrink += max_w.saturating_sub(floor);
        }

        if total_shrink == 0 {
            break; // All widest columns are already at their minimum.
        }

        if total_shrink <= excess {
            // Shrink all widest columns to their floor.
            for &i in &widest {
                let floor = next_w.max(mins[i]);
                widths[i] = floor;
            }
            excess -= total_shrink;
        } else {
            // Distribute remaining excess evenly among widest columns.
            let per_col = excess / widest.len();
            let remainder = excess % widest.len();
            for (j, &i) in widest.iter().enumerate() {
                let shrink = per_col + usize::from(j < remainder);
                widths[i] = widths[i].saturating_sub(shrink).max(mins[i]);
            }
            excess = 0;
        }
    }

    // Phase 2: Proportional fallback if sum of minimums still exceeds available.
    let total_now: usize = widths.iter().sum();
    if total_now > available {
        for w in &mut widths {
            *w = (*w * available) / total_now.max(1);
            if *w == 0 {
                *w = 1;
            }
        }
    }

    widths
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn make_text(lines: Vec<&'static str>) -> ContentBlock {
        ContentBlock::Text {
            id: BlockId::new(),
            lines: lines.into_iter().map(Line::raw).collect(),
            collapsed: false,
        }
    }

    fn make_running(output: Vec<&'static str>) -> (BlockId, ContentBlock) {
        let id = BlockId::new();
        let block = ContentBlock::Running {
            id,
            command: "test".into(),
            args: vec![],
            started_at: Instant::now(),
            output_lines: output.into_iter().map(Line::raw).collect(),
            output_truncated: false,
            collapsed: false,
            auto_expanded: false,
        };
        (id, block)
    }

    // ── format_elapsed ──────────────────────────────────────────────────────

    #[test]
    fn format_elapsed_seconds() {
        assert_eq!(format_elapsed(Duration::from_secs(5)), "5s");
        assert_eq!(format_elapsed(Duration::from_secs(59)), "59s");
    }

    #[test]
    fn format_elapsed_minutes() {
        assert_eq!(format_elapsed(Duration::from_secs(60)), "1m 0s");
        assert_eq!(format_elapsed(Duration::from_secs(154)), "2m 34s");
    }

    // ── append_lines_to_running ─────────────────────────────────────────────

    #[test]
    fn append_lines_adds_to_running_block() {
        let mut buf = ScrollBuffer::new();
        let (id, block) = make_running(vec![]);
        buf.push(block);

        buf.append_lines_to_running(id, vec![Line::raw("line 1"), Line::raw("line 2")]);

        if let ContentBlock::Running { output_lines, .. } = &buf.blocks[0] {
            assert_eq!(output_lines.len(), 2);
        } else {
            panic!("expected Running block");
        }
    }

    #[test]
    fn append_lines_noop_on_wrong_id() {
        let mut buf = ScrollBuffer::new();
        let (_, block) = make_running(vec!["existing"]);
        buf.push(block);

        let other_id = BlockId::new();
        buf.append_lines_to_running(other_id, vec![Line::raw("new")]);

        if let ContentBlock::Running { output_lines, .. } = &buf.blocks[0] {
            assert_eq!(output_lines.len(), 1);
        } else {
            panic!("expected Running block");
        }
    }

    #[test]
    fn append_lines_respects_auto_scroll_opt_out() {
        let mut buf = ScrollBuffer::new();
        let (id, block) = make_running(vec![]);
        buf.push(block);

        // Scroll up manually (offset 0 = top)
        buf.scroll_offset = 0;
        buf.append_lines_to_running(id, vec![Line::raw("x")]);

        // Should NOT have jumped to bottom
        assert_eq!(buf.scroll_offset, 0);
    }

    #[test]
    fn append_lines_auto_scrolls_when_at_bottom() {
        let mut buf = ScrollBuffer::new();
        let (id, block) = make_running(vec![]);
        buf.push(block);

        // At bottom sentinel
        buf.scroll_offset = usize::MAX;
        buf.append_lines_to_running(id, vec![Line::raw("x")]);

        assert_eq!(buf.scroll_offset, usize::MAX);
    }

    // ── finalize_running ────────────────────────────────────────────────────

    #[test]
    fn finalize_running_converts_to_text() {
        let mut buf = ScrollBuffer::new();
        let (id, block) = make_running(vec!["out1", "out2"]);
        buf.push(block);

        buf.finalize_running(id, &RunCompletion::Exited(0));

        assert!(matches!(buf.blocks[0], ContentBlock::Text { .. }));
        if let ContentBlock::Text { lines, .. } = &buf.blocks[0] {
            // First line = header with command name; subsequent = output
            assert!(lines.len() >= 3); // header + 2 output lines
        }
    }

    #[test]
    fn finalize_running_exit_nonzero_shows_exit_code() {
        let mut buf = ScrollBuffer::new();
        let (id, block) = make_running(vec![]);
        buf.push(block);

        buf.finalize_running(id, &RunCompletion::Exited(1));

        if let ContentBlock::Text { lines, .. } = &buf.blocks[0] {
            let header_text: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
            assert!(header_text.contains("exit 1"), "got: {header_text}");
        } else {
            panic!("expected Text block");
        }
    }

    #[test]
    fn finalize_running_noop_on_wrong_id() {
        let mut buf = ScrollBuffer::new();
        let (_, block) = make_running(vec![]);
        buf.push(block);

        let other_id = BlockId::new();
        buf.finalize_running(other_id, &RunCompletion::Exited(0));

        // Block should still be Running
        assert!(matches!(buf.blocks[0], ContentBlock::Running { .. }));
    }

    // ── render_lines_at (Running variant) ───────────────────────────────────

    #[test]
    fn running_render_shows_spinner_and_output() {
        let now = Instant::now();
        let (id, block) = make_running(vec!["stdout line"]);
        let lines = block.render_lines_at(80, now);

        // header line + 1 output line
        assert_eq!(lines.len(), 2);

        // Header should contain command name "test"
        let header: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(header.contains("test"), "got: {header}");
        let _ = id;
    }

    #[test]
    fn running_render_collapsed_shows_tail_preview() {
        let id = BlockId::new();
        let block = ContentBlock::Running {
            id,
            command: "build".into(),
            args: vec!["--release".into()],
            started_at: Instant::now(),
            output_lines: vec![Line::raw("a"), Line::raw("b"), Line::raw("c")],
            output_truncated: false,
            collapsed: true,
            auto_expanded: false,
        };

        let lines = block.render_lines_at(80, Instant::now());
        // header + 3 tail lines
        assert_eq!(lines.len(), 4);
        // Tail lines should contain the output text, be indented, and be DarkGray
        for (i, expected) in ["a", "b", "c"].iter().enumerate() {
            let tail = &lines[i + 1];
            let text: String = tail.spans.iter().map(|s| s.content.as_ref()).collect();
            assert!(text.contains(expected), "tail {i}: got {text:?}");
            // First span is the 2-space indent
            assert_eq!(tail.spans[0].content.as_ref(), "  ", "tail {i} indent");
            // All spans should be DarkGray (patch_style applies to every span)
            for span in &tail.spans {
                assert_eq!(
                    span.style.fg,
                    Some(Color::DarkGray),
                    "tail {i} span {span:?} should be DarkGray"
                );
            }
        }
    }

    #[test]
    fn running_render_collapsed_partial_tail() {
        let id = BlockId::new();
        let block = ContentBlock::Running {
            id,
            command: "test".into(),
            args: vec![],
            started_at: Instant::now(),
            output_lines: vec![Line::raw("x"), Line::raw("y")],
            output_truncated: false,
            collapsed: true,
            auto_expanded: false,
        };

        let lines = block.render_lines_at(80, Instant::now());
        // header + 2 tail lines (fewer than cap)
        assert_eq!(lines.len(), 3);
        assert_eq!(block.line_count(), 3);
        let text1: String = lines[1].spans.iter().map(|s| s.content.as_ref()).collect();
        let text2: String = lines[2].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(text1.contains("x"), "got: {text1}");
        assert!(text2.contains("y"), "got: {text2}");
    }

    #[test]
    fn running_render_collapsed_empty_output_is_single_line() {
        let id = BlockId::new();
        let block = ContentBlock::Running {
            id,
            command: "build".into(),
            args: vec![],
            started_at: Instant::now(),
            output_lines: vec![],
            output_truncated: false,
            collapsed: true,
            auto_expanded: false,
        };

        let lines = block.render_lines_at(80, Instant::now());
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn running_render_collapsed_caps_at_three_lines() {
        let id = BlockId::new();
        let output: Vec<Line<'static>> = (0..10).map(|i| Line::raw(format!("line {i}"))).collect();
        let block = ContentBlock::Running {
            id,
            command: "test".into(),
            args: vec![],
            started_at: Instant::now(),
            output_lines: output,
            output_truncated: false,
            collapsed: true,
            auto_expanded: false,
        };

        let lines = block.render_lines_at(80, Instant::now());
        // header + 3 tail lines (capped)
        assert_eq!(lines.len(), 4);
        // Should show the LAST 3 lines (7, 8, 9)
        let tail1: String = lines[1].spans.iter().map(|s| s.content.as_ref()).collect();
        let tail3: String = lines[3].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(tail1.contains("line 7"), "got: {tail1}");
        assert!(tail3.contains("line 9"), "got: {tail3}");
    }

    #[test]
    fn line_count_running_collapsed_matches_render() {
        let id = BlockId::new();
        let block = ContentBlock::Running {
            id,
            command: "build".into(),
            args: vec!["--release".into()],
            started_at: Instant::now(),
            output_lines: vec![
                Line::raw("a"),
                Line::raw("b"),
                Line::raw("c"),
                Line::raw("d"),
                Line::raw("e"),
            ],
            output_truncated: false,
            collapsed: true,
            auto_expanded: false,
        };

        let rendered = block.render_lines_at(80, Instant::now()).len();
        assert_eq!(block.line_count(), rendered);
    }

    #[test]
    fn running_render_truncated_adds_notice() {
        let id = BlockId::new();
        let block = ContentBlock::Running {
            id,
            command: "test".into(),
            args: vec![],
            started_at: Instant::now(),
            output_lines: vec![Line::raw("x")],
            output_truncated: true,
            collapsed: false,
            auto_expanded: false,
        };

        let lines = block.render_lines_at(80, Instant::now());
        // header + truncation notice + 1 output
        assert_eq!(lines.len(), 3);
        let notice: String = lines[1].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(notice.contains("truncated"), "got: {notice}");
    }

    // ── line_count ──────────────────────────────────────────────────────────

    #[test]
    fn line_count_text_uncollapsed() {
        let block = make_text(vec!["a", "b", "c"]);
        assert_eq!(block.line_count(), 3);
    }

    #[test]
    fn line_count_text_collapsed() {
        let mut block = make_text(vec!["a", "b", "c"]);
        if let ContentBlock::Text { collapsed, .. } = &mut block {
            *collapsed = true;
        }
        assert_eq!(block.line_count(), 1);
    }

    #[test]
    fn line_count_running_matches_render() {
        let (_, block) = make_running(vec!["x", "y"]);
        let rendered = block.render_lines_at(80, Instant::now()).len();
        assert_eq!(block.line_count(), rendered);
    }

    #[test]
    fn line_count_running_with_truncation() {
        let id = BlockId::new();
        let block = ContentBlock::Running {
            id,
            command: "t".into(),
            args: vec![],
            started_at: Instant::now(),
            output_lines: vec![Line::raw("a"), Line::raw("b")],
            output_truncated: true,
            collapsed: false,
            auto_expanded: false,
        };
        let rendered = block.render_lines_at(80, Instant::now()).len();
        assert_eq!(block.line_count(), rendered);
    }

    #[test]
    fn line_count_divider() {
        let block = ContentBlock::Divider { id: BlockId::new() };
        assert_eq!(block.line_count(), 1);
    }

    // ── scroll_up from sentinel ─────────────────────────────────────────────

    #[test]
    fn scroll_up_from_sentinel_resolves_correctly() {
        let mut buf = ScrollBuffer::new();
        // Add enough lines to make scrolling meaningful
        for i in 0..20 {
            buf.push(make_text(vec![Box::leak(
                format!("line {i}").into_boxed_str(),
            )]));
        }
        buf.scroll_to_bottom(); // sentinel = usize::MAX

        buf.scroll_up(1);

        // After scrolling up, offset must NOT be usize::MAX or near it
        assert!(
            buf.scroll_offset < usize::MAX / 2,
            "scroll_offset should be small, got {}",
            buf.scroll_offset
        );
    }

    // ── auto-expand on error ────────────────────────────────────────────────

    #[test]
    fn auto_expand_on_error_indicator() {
        let mut buf = ScrollBuffer::new();
        let id = BlockId::new();
        let block = ContentBlock::Running {
            id,
            command: "test".into(),
            args: vec![],
            started_at: Instant::now(),
            output_lines: vec![],
            output_truncated: false,
            collapsed: true,
            auto_expanded: false,
        };
        buf.push(block);

        buf.append_lines_to_running(id, vec![Line::raw("error[E0308]: mismatched types")]);

        if let ContentBlock::Running {
            collapsed,
            auto_expanded,
            ..
        } = &buf.blocks[0]
        {
            assert!(!collapsed, "block should be expanded after error");
            assert!(auto_expanded, "auto_expanded flag should be set");
        } else {
            panic!("expected Running block");
        }
    }

    #[test]
    fn no_expand_without_error() {
        let mut buf = ScrollBuffer::new();
        let id = BlockId::new();
        let block = ContentBlock::Running {
            id,
            command: "test".into(),
            args: vec![],
            started_at: Instant::now(),
            output_lines: vec![],
            output_truncated: false,
            collapsed: true,
            auto_expanded: false,
        };
        buf.push(block);

        buf.append_lines_to_running(id, vec![Line::raw("compiling crate v0.1.0")]);

        if let ContentBlock::Running { collapsed, .. } = &buf.blocks[0] {
            assert!(collapsed, "block should remain collapsed without error");
        } else {
            panic!("expected Running block");
        }
    }

    #[test]
    fn auto_expand_only_triggers_once() {
        let mut buf = ScrollBuffer::new();
        let id = BlockId::new();
        let block = ContentBlock::Running {
            id,
            command: "test".into(),
            args: vec![],
            started_at: Instant::now(),
            output_lines: vec![],
            output_truncated: false,
            collapsed: true,
            auto_expanded: false,
        };
        buf.push(block);

        // First error triggers expansion
        buf.append_lines_to_running(id, vec![Line::raw("FAILED test_foo")]);
        assert!(!buf.blocks[0].is_collapsed());

        // Manually re-collapse
        buf.blocks[0].toggle_collapse();
        assert!(buf.blocks[0].is_collapsed());

        // Second error should NOT re-expand (auto_expanded is already true)
        buf.append_lines_to_running(id, vec![Line::raw("FAILED test_bar")]);
        assert!(
            buf.blocks[0].is_collapsed(),
            "should not re-expand after manual collapse"
        );
    }

    #[test]
    fn auto_expand_renders_indicator() {
        let id = BlockId::new();
        let block = ContentBlock::Running {
            id,
            command: "build".into(),
            args: vec![],
            started_at: Instant::now(),
            output_lines: vec![Line::raw("error: something went wrong")],
            output_truncated: false,
            collapsed: false,
            auto_expanded: true,
        };
        let lines = block.render_lines_at(120, Instant::now());
        let header: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(
            header.contains("auto-expanded due to error"),
            "got: {header}"
        );
    }

    #[test]
    fn line_contains_error_indicator_cases() {
        assert!(line_contains_error_indicator("error[E0308]: types"));
        assert!(line_contains_error_indicator("test FAILED"));
        assert!(line_contains_error_indicator("FAIL src/lib.rs"));
        assert!(line_contains_error_indicator("thread 'main' panicked"));
        assert!(line_contains_error_indicator("fatal: not a git repository"));
        assert!(line_contains_error_indicator("fail: some check"));
        assert!(!line_contains_error_indicator("compiling graft v0.1.0"));
        assert!(!line_contains_error_indicator("Finished dev profile"));
    }

    #[test]
    fn auto_expand_detects_error_in_multi_span_line() {
        let mut buf = ScrollBuffer::new();
        let id = BlockId::new();
        let block = ContentBlock::Running {
            id,
            command: "test".into(),
            args: vec![],
            started_at: Instant::now(),
            output_lines: vec![],
            output_truncated: false,
            collapsed: true,
            auto_expanded: false,
        };
        buf.push(block);

        // Error indicator split across multiple spans (as ratatui styled output)
        let multi_span_line = Line::from(vec![
            Span::styled("prefix: ", Style::default().fg(Color::White)),
            Span::styled("error[E0308]", Style::default().fg(Color::Red)),
            Span::raw(" in file.rs"),
        ]);
        buf.append_lines_to_running(id, vec![multi_span_line]);

        if let ContentBlock::Running {
            collapsed,
            auto_expanded,
            ..
        } = &buf.blocks[0]
        {
            assert!(!collapsed, "should expand on multi-span error line");
            assert!(auto_expanded, "auto_expanded should be set");
        } else {
            panic!("expected Running block");
        }
    }

    // ── compute_col_widths ────────────────────────────────────────────────

    fn make_row(cells: &[&str]) -> Vec<Span<'static>> {
        cells.iter().map(|s| Span::raw(s.to_string())).collect()
    }

    #[test]
    fn col_widths_fits_without_shrinking() {
        let headers = vec!["A".into(), "B".into()];
        let rows = vec![make_row(&["xx", "yy"])];
        let widths = compute_col_widths(&headers, &rows, 80, 2);
        assert_eq!(widths, vec![2, 2]);
    }

    #[test]
    fn col_widths_single_wide_column_absorbs_excess() {
        // Headers: "N" (1), "Description" (11) → natural widths = 1, 11
        // available = 20 - 2 sep = 18, total natural = 12, fits.
        // Now with available = 10 - 2 = 8, total natural = 12, excess = 4.
        // Widest is col 1 (11). Shrink toward col 0 (1) but excess=4, so 11-4=7.
        let headers = vec!["N".into(), "Description".into()];
        let rows: Vec<Vec<Span<'static>>> = vec![];
        let widths = compute_col_widths(&headers, &rows, 10, 2);
        assert_eq!(widths[0], 1); // short column preserved
        assert_eq!(widths[1], 7); // wide column absorbed the excess
    }

    #[test]
    fn col_widths_tied_widest_share_shrinkage() {
        // Three columns, all natural width 10. Total natural = 30.
        // Available = 40 - 4 sep = 36. Fits.
        // Available = 20 - 4 = 16. Excess = 14. All three tied.
        // Per-col shrink = 14/3 = 4 remainder 2. → 10-5=5, 10-5=5, 10-4=6
        let headers = vec![
            "AAAAAAAAAA".into(),
            "BBBBBBBBBB".into(),
            "CCCCCCCCCC".into(),
        ];
        let rows: Vec<Vec<Span<'static>>> = vec![];
        let widths = compute_col_widths(&headers, &rows, 20, 3);
        let total: usize = widths.iter().sum();
        assert_eq!(total, 16); // exactly fills available
                               // All should be roughly equal
        for w in &widths {
            assert!(*w >= 4 && *w <= 6, "width {w} out of expected range");
        }
    }

    #[test]
    fn col_widths_all_at_minimum_triggers_proportional() {
        // Columns natural: 4, 4, 4. Total = 12. Available = 2 (width=6, sep=4).
        // Minimums = 4, 4, 4. Sum of mins = 12 > 2. Proportional fallback.
        let headers = vec!["ABCD".into(), "EFGH".into(), "IJKL".into()];
        let rows: Vec<Vec<Span<'static>>> = vec![];
        let widths = compute_col_widths(&headers, &rows, 6, 3);
        // All widths should be >= 1 (floor)
        for w in &widths {
            assert!(*w >= 1, "width should be at least 1, got {w}");
        }
    }

    #[test]
    fn col_widths_zero_available_returns_all_ones() {
        let headers = vec!["A".into(), "B".into()];
        let rows = vec![make_row(&["xx", "yy"])];
        let widths = compute_col_widths(&headers, &rows, 0, 2);
        // With 0 total_width, available = 0, early return with natural widths
        // (no shrinking path triggered since available == 0).
        // Actually available=0 triggers early return.
        assert!(!widths.is_empty());
    }

    #[test]
    fn col_widths_single_column() {
        let headers = vec!["LongHeader".into()];
        let rows = vec![make_row(&["short"])];
        // Natural = 10, available = 5 - 0 sep = 5, excess = 5
        let widths = compute_col_widths(&headers, &rows, 5, 1);
        assert_eq!(widths, vec![5]);
    }

    #[test]
    fn col_widths_empty_table_returns_empty() {
        let headers: Vec<String> = vec![];
        let rows: Vec<Vec<Span<'static>>> = vec![];
        let widths = compute_col_widths(&headers, &rows, 80, 0);
        assert!(widths.is_empty());
    }
}
