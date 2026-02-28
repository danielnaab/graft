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
                    return vec![Line::from(vec![
                        Span::styled(format!("{spinner} "), Style::default().fg(Color::Yellow)),
                        Span::styled(summary, Style::default().fg(Color::DarkGray)),
                        Span::styled(
                            format!("  [{elapsed_str}]"),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ])];
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
    fn total_lines(&self, width: u16) -> usize {
        let now = Instant::now();
        let mut total = 0;
        for (i, block) in self.blocks.iter().enumerate() {
            if i > 0 {
                total += 1; // blank line between blocks
            }
            total += block.render_lines_at(width, now).len();
        }
        total
    }

    /// Append lines to the `Running` block with the given `id`.
    ///
    /// Lines beyond [`MAX_RUNNING_OUTPUT_LINES`] are dropped from the front
    /// and `output_truncated` is set so the block can show a truncation notice.
    /// No-ops if the block is not found or is not a `Running` block.
    pub(super) fn append_lines_to_running(&mut self, id: BlockId, new_lines: Vec<Line<'static>>) {
        const MAX_RUNNING_OUTPUT_LINES: usize = 10_000;
        const RUNNING_LINES_TO_DROP: usize = 1_000;

        if let Some(ContentBlock::Running {
            output_lines,
            output_truncated,
            ..
        }) = self.blocks.iter_mut().find(|b| b.id() == id)
        {
            output_lines.extend(new_lines);
            if output_lines.len() > MAX_RUNNING_OUTPUT_LINES {
                output_lines.drain(0..RUNNING_LINES_TO_DROP);
                *output_truncated = true;
            }
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

        let block = &self.blocks[pos];
        let ContentBlock::Running {
            command,
            args,
            started_at,
            output_lines,
            collapsed,
            ..
        } = block
        else {
            return;
        };

        let elapsed = now.duration_since(*started_at);
        let elapsed_str = format_elapsed(elapsed);
        let arg_str = args.join(" ");
        let cmd_display = if arg_str.is_empty() {
            command.clone()
        } else {
            format!("{command}  {arg_str}")
        };
        let collapsed = *collapsed;

        let (symbol, symbol_color, exit_label) = match &outcome {
            RunCompletion::Exited(0) => ("\u{2713}", Color::Green, String::new()),
            RunCompletion::Exited(n) => ("\u{2717}", Color::Red, format!("  (exit {n})")),
            RunCompletion::Error(_) => ("\u{2717}", Color::Red, String::new()),
        };

        let mut header = vec![
            Span::styled("\u{25b6} ", Style::default().fg(Color::DarkGray)),
            Span::styled(cmd_display, Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("    {symbol}  {elapsed_str}{exit_label}"),
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
        // Clone output lines out before the mutable borrow below
        let saved_output = output_lines.clone();
        lines.extend(saved_output);

        self.blocks[pos] = ContentBlock::Text {
            id,
            lines,
            collapsed,
        };
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
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
    }

    /// Scroll down by `n` lines, using last-known viewport dimensions.
    pub(super) fn scroll_down(&mut self, n: usize) {
        let total = self.total_lines(self.last_width);
        let max_offset = total.saturating_sub(self.last_viewport_height as usize);
        self.scroll_offset = (self.scroll_offset.saturating_add(n)).min(max_offset);
    }

    /// Move focus to the next block.
    pub(super) fn focus_next(&mut self) {
        if self.blocks.is_empty() {
            return;
        }
        match self.focused_block {
            None => self.focused_block = Some(0),
            Some(i) => {
                if i + 1 < self.blocks.len() {
                    self.focused_block = Some(i + 1);
                }
            }
        }
    }

    /// Move focus to the previous block.
    pub(super) fn focus_prev(&mut self) {
        match self.focused_block {
            None => {
                if !self.blocks.is_empty() {
                    self.focused_block = Some(self.blocks.len() - 1);
                }
            }
            Some(i) => {
                if i > 0 {
                    self.focused_block = Some(i - 1);
                }
            }
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
        let now = Instant::now();
        let mut all_lines: Vec<(Line<'static>, Option<usize>)> = Vec::new();
        for (block_idx, block) in self.blocks.iter().enumerate() {
            if block_idx > 0 {
                all_lines.push((Line::from(""), None));
            }
            for line in block.render_lines_at(width, now) {
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

        // Apply focus highlight
        let lines: Vec<Line<'static>> = visible
            .iter()
            .map(|(line, block_idx)| {
                if let (Some(focused), Some(bidx)) = (self.focused_block, block_idx) {
                    if *bidx == focused {
                        let highlighted = line
                            .clone()
                            .patch_style(Style::default().bg(Color::Rgb(30, 30, 45)));
                        return highlighted;
                    }
                }
                line.clone()
            })
            .collect();

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, area);
    }
}

// ===== Helpers =====

/// Format a duration as a short human-readable string: "42s" or "2m 34s".
fn format_elapsed(d: Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        format!("{secs}s")
    } else {
        format!("{}m {}s", secs / 60, secs % 60)
    }
}

fn format_first_line(line: &Line<'_>) -> String {
    let mut s = String::new();
    for span in &line.spans {
        s.push_str(&span.content);
    }
    if s.len() > 60 {
        s.truncate(57);
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

    // Start with header widths
    let mut widths: Vec<usize> = headers.iter().map(|h| h.width()).collect();
    widths.resize(col_count, 0);

    // Expand to fit data
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

    // Cap total to available width (minus separators)
    let sep_space = (col_count.saturating_sub(1)) * 2;
    let available = (total_width as usize).saturating_sub(sep_space);
    let total_col: usize = widths.iter().sum();

    if total_col > available && available > 0 {
        // Proportionally shrink
        for w in &mut widths {
            *w = (*w * available) / total_col.max(1);
            if *w == 0 {
                *w = 1;
            }
        }
    }

    widths
}
