//! Scroll buffer: a vertical list of content blocks with scroll and focus support.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use std::sync::atomic::{AtomicU64, Ordering};

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
    /// A data table with headers and rows.
    Table {
        id: BlockId,
        title: String,
        headers: Vec<String>,
        rows: Vec<Vec<Span<'static>>>,
        collapsed: bool,
    },
    /// A horizontal divider line.
    Divider { id: BlockId },
}

#[allow(dead_code)]
impl ContentBlock {
    pub(super) fn id(&self) -> BlockId {
        match self {
            Self::Text { id, .. } | Self::Table { id, .. } | Self::Divider { id } => *id,
        }
    }

    pub(super) fn is_collapsed(&self) -> bool {
        match self {
            Self::Text { collapsed, .. } | Self::Table { collapsed, .. } => *collapsed,
            Self::Divider { .. } => false,
        }
    }

    pub(super) fn toggle_collapse(&mut self) {
        match self {
            Self::Text { collapsed, .. } | Self::Table { collapsed, .. } => {
                *collapsed = !*collapsed;
            }
            Self::Divider { .. } => {}
        }
    }

    /// Render this block into lines for display.
    #[allow(clippy::too_many_lines)]
    fn render_lines(&self, width: u16) -> Vec<Line<'static>> {
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

    /// Append lines to the last block if it's a Text block.
    /// Returns false if the last block isn't Text or there are no blocks.
    pub(super) fn append_lines_to_last(&mut self, new_lines: Vec<Line<'static>>) -> bool {
        if let Some(ContentBlock::Text { lines, .. }) = self.blocks.last_mut() {
            lines.extend(new_lines);
            self.scroll_to_bottom();
            true
        } else {
            false
        }
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
        let mut total = 0;
        for (i, block) in self.blocks.iter().enumerate() {
            if i > 0 {
                total += 1; // blank line between blocks
            }
            total += block.render_lines(width).len();
        }
        total
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

        // Collect all rendered lines with block association
        let mut all_lines: Vec<(Line<'static>, Option<usize>)> = Vec::new();
        for (block_idx, block) in self.blocks.iter().enumerate() {
            if block_idx > 0 {
                all_lines.push((Line::from(""), None));
            }
            for line in block.render_lines(width) {
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
