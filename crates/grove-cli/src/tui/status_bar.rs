//! Status bar types.

use std::time::{Duration, Instant};

use ratatui::style::Color;

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
            (MessageType::Error, true) => "\u{2717}",
            (MessageType::Error, false) => "X",
            (MessageType::Warning, true) => "\u{26a0}",
            (MessageType::Warning, false) => "!",
            (MessageType::Info, true) => "\u{2139}",
            (MessageType::Info, false) => "i",
            (MessageType::Success, true) => "\u{2713}",
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
