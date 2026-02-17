//! Status bar types and rendering.

use super::{
    supports_unicode, App, Color, Duration, Instant, Line, Paragraph, Rect, RepoDetailProvider,
    RepoRegistry, Span, Style, UnicodeWidthStr,
};

/// Message types for status bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MessageType {
    Error,
    Warning,
    Info,
    Success,
}

impl MessageType {
    /// Get the symbol for this message type.
    pub(crate) fn symbol(&self, unicode: bool) -> &'static str {
        match (self, unicode) {
            (MessageType::Error, true) => "✗",
            (MessageType::Error, false) => "X",
            (MessageType::Warning, true) => "⚠",
            (MessageType::Warning, false) => "!",
            (MessageType::Info, true) => "ℹ",
            (MessageType::Info, false) => "i",
            (MessageType::Success, true) => "✓",
            (MessageType::Success, false) => "*",
        }
    }

    /// Get the foreground color for this message type.
    pub(crate) fn fg_color(&self) -> Color {
        match self {
            MessageType::Error | MessageType::Info => Color::White,
            MessageType::Warning | MessageType::Success => Color::Black,
        }
    }

    /// Get the background color for this message type.
    pub(crate) fn bg_color(&self) -> Color {
        match self {
            MessageType::Error => Color::Red,
            MessageType::Warning => Color::Yellow,
            MessageType::Info => Color::Blue,
            MessageType::Success => Color::Green,
        }
    }
}

/// A status bar message with metadata.
#[derive(Debug, Clone)]
pub(crate) struct StatusMessage {
    pub(super) text: String,
    pub(super) msg_type: MessageType,
    pub(super) shown_at: Instant,
}

impl StatusMessage {
    /// Create a new status message.
    pub(crate) fn new(text: impl Into<String>, msg_type: MessageType) -> Self {
        Self {
            text: text.into(),
            msg_type,
            shown_at: Instant::now(),
        }
    }

    /// Create an error message.
    pub(crate) fn error(text: impl Into<String>) -> Self {
        Self::new(text, MessageType::Error)
    }

    /// Create a warning message.
    pub(crate) fn warning(text: impl Into<String>) -> Self {
        Self::new(text, MessageType::Warning)
    }

    /// Create an info message.
    pub(crate) fn info(text: impl Into<String>) -> Self {
        Self::new(text, MessageType::Info)
    }

    /// Create a success message.
    pub(crate) fn success(text: impl Into<String>) -> Self {
        Self::new(text, MessageType::Success)
    }

    /// Check if this message has expired (older than 3 seconds).
    pub(crate) fn is_expired(&self) -> bool {
        self.shown_at.elapsed() > Duration::from_secs(3)
    }
}

impl<R: RepoRegistry, D: RepoDetailProvider> App<R, D> {
    /// Render the status bar at the bottom of the screen.
    ///
    /// When a status message is active, shows it with colored background.
    /// Otherwise, shows a context-sensitive keybinding hint bar.
    pub(super) fn render_status_bar(&self, frame: &mut ratatui::Frame, area: Rect) {
        let unicode = supports_unicode();

        if let Some(msg) = &self.status_message {
            let symbol = msg.msg_type.symbol(unicode);
            let fg = msg.msg_type.fg_color();
            let bg = msg.msg_type.bg_color();
            let mut text = format!(" {symbol} {}", msg.text);

            // Truncate with ellipsis if message is too long
            let max_width = area.width as usize;
            if text.width() > max_width {
                let target_width = max_width.saturating_sub(3);
                let mut truncated = String::new();
                let mut current_width = 0;

                for ch in text.chars() {
                    let ch_width = UnicodeWidthStr::width(ch.to_string().as_str());
                    if current_width + ch_width > target_width {
                        break;
                    }
                    truncated.push(ch);
                    current_width += ch_width;
                }

                truncated.push_str("...");
                text = truncated;
            }

            let status_bar = Paragraph::new(text).style(Style::default().fg(fg).bg(bg));
            frame.render_widget(status_bar, area);
        } else {
            // Render context-sensitive hint bar
            let hints = self.current_hints();
            let max_width = area.width as usize;

            let mut spans: Vec<Span> = Vec::new();
            let mut current_width = 1; // Start with 1 for leading space

            for (i, hint) in hints.iter().enumerate() {
                // Calculate width of this hint: "key:action" + separator
                let hint_width = hint.key.width() + 1 + hint.action.width();
                let separator_width = if i > 0 { 2 } else { 0 }; // "  " between hints

                if current_width + separator_width + hint_width > max_width {
                    break; // Graceful truncation from the right
                }

                if i > 0 {
                    spans.push(Span::raw("  "));
                    current_width += 2;
                }

                spans.push(Span::styled(
                    hint.key.to_string(),
                    Style::default().fg(Color::Cyan),
                ));
                spans.push(Span::styled(
                    format!(":{}", hint.action),
                    Style::default().fg(Color::Gray),
                ));
                current_width += hint_width;
            }

            // Add leading space
            spans.insert(0, Span::raw(" "));

            let line = Line::from(spans);
            let hint_bar = Paragraph::new(line).style(Style::default().bg(Color::DarkGray));
            frame.render_widget(hint_bar, area);
        }
    }
}
