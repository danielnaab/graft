//! Sticky header rendering for the transcript TUI.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use super::formatting::compact_path;

/// Data needed to render the header.
pub(super) struct HeaderData<'a> {
    pub workspace_name: &'a str,
    pub repo_path: Option<&'a str>,
    pub branch: Option<&'a str>,
    pub is_dirty: Option<bool>,
    pub ahead: Option<usize>,
    pub behind: Option<usize>,
}

/// Render the 2-line sticky header.
///
/// Line 1: "grove · `workspace_name`"
/// Line 2: repo path, branch, dirty indicator, ahead/behind (or "no repo selected")
pub(super) fn render_header(frame: &mut ratatui::Frame, area: Rect, data: &HeaderData) {
    let mut lines = Vec::new();

    // Line 1: grove title
    lines.push(Line::from(vec![
        Span::styled(
            "grove",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" \u{00b7} ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            data.workspace_name.to_string(),
            Style::default().fg(Color::White),
        ),
    ]));

    // Line 2: repo context
    if let Some(path) = data.repo_path {
        let max_path_width = (area.width as usize).saturating_sub(30);
        let display_path = compact_path(path, max_path_width);

        let mut spans = vec![Span::styled(
            display_path,
            Style::default().fg(Color::White),
        )];

        if let Some(b) = data.branch {
            spans.push(Span::styled(" ", Style::default()));
            spans.push(Span::styled(
                format!("({b})"),
                Style::default().fg(Color::Cyan),
            ));
        }

        if let Some(dirty) = data.is_dirty {
            spans.push(Span::raw(" "));
            if dirty {
                spans.push(Span::styled("\u{25cf}", Style::default().fg(Color::Yellow)));
            } else {
                spans.push(Span::styled("\u{25cb}", Style::default().fg(Color::Green)));
            }
        }

        if let Some(n) = data.ahead {
            if n > 0 {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(
                    format!("\u{2191}{n}"),
                    Style::default().fg(Color::Green),
                ));
            }
        }

        if let Some(n) = data.behind {
            if n > 0 {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(
                    format!("\u{2193}{n}"),
                    Style::default().fg(Color::Red),
                ));
            }
        }

        lines.push(Line::from(spans));
    } else {
        lines.push(Line::from(Span::styled(
            "no repo selected",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let widget = Paragraph::new(lines).style(Style::default().bg(Color::Rgb(20, 20, 30)));
    frame.render_widget(widget, area);
}
