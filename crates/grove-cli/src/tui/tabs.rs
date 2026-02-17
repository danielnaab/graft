//! `DetailTab` enum and tab header rendering.

use super::{App, Color, Line, Modifier, RepoDetailProvider, RepoRegistry, Span, Style};

/// Which tab is active in the detail pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailTab {
    Changes,  // 1 — changed files + commits
    State,    // 2 — state queries
    Commands, // 3 — command picker
}

impl DetailTab {
    /// Display label for the tab.
    pub fn label(self) -> &'static str {
        match self {
            DetailTab::Changes => "Changes",
            DetailTab::State => "State",
            DetailTab::Commands => "Commands",
        }
    }
}

impl<R: RepoRegistry, D: RepoDetailProvider> App<R, D> {
    /// Render the tab header as a title Line for the detail pane block.
    pub(super) fn render_tab_header(&self) -> Line<'static> {
        let tabs = [DetailTab::Changes, DetailTab::State, DetailTab::Commands];
        let mut spans: Vec<Span<'static>> = Vec::new();

        spans.push(Span::raw(" "));

        for (i, tab) in tabs.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(" | ", Style::default().fg(Color::DarkGray)));
            }

            if *tab == self.active_tab {
                spans.push(Span::styled(
                    tab.label(),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ));
            } else {
                spans.push(Span::styled(
                    tab.label(),
                    Style::default().fg(Color::DarkGray),
                ));
            }
        }

        spans.push(Span::raw(" "));

        Line::from(spans)
    }
}
