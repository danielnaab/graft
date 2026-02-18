//! Changes tab: file changes and recent commits.

use super::{
    format_file_change_indicator, App, Color, KeyCode, Line, Modifier, Paragraph, Rect,
    RepoDetailProvider, RepoRegistry, Span, Style,
};

impl<R: RepoRegistry, D: RepoDetailProvider> App<R, D> {
    /// Handle keys when the Changes tab is active.
    pub(super) fn handle_key_changes_tab(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.detail_scroll = self.detail_scroll.saturating_add(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.detail_scroll = self.detail_scroll.saturating_sub(1);
            }
            KeyCode::Enter => {
                self.pop_view();
            }
            _ => {}
        }
    }

    /// Render the Changes tab content in the given area.
    pub(super) fn render_changes_tab(&self, frame: &mut ratatui::Frame, area: Rect) {
        let detail_lines = self.build_detail_lines();

        // Clamp scroll to content height
        let inner_height = area.height as usize;
        let max_scroll = detail_lines.len().saturating_sub(inner_height);
        let clamped_scroll = self.detail_scroll.min(max_scroll);

        let detail_widget = Paragraph::new(detail_lines)
            .scroll((u16::try_from(clamped_scroll).unwrap_or(u16::MAX), 0));

        frame.render_widget(detail_widget, area);
    }

    /// Build the lines for the detail pane based on cached detail.
    pub(super) fn build_detail_lines(&self) -> Vec<Line<'static>> {
        let Some(detail) = &self.cached_detail else {
            return vec![Line::from(Span::styled(
                "No repository selected",
                Style::default().fg(Color::Gray),
            ))];
        };

        let mut lines: Vec<Line<'static>> = Vec::new();

        // Show error as warning if present (but continue rendering partial data)
        if let Some(error) = &detail.error {
            lines.push(Line::from(Span::styled(
                format!("Error: {error}"),
                Style::default().fg(Color::Red),
            )));
            lines.push(Line::from(""));
        }

        // Branch/status header from registry
        if let Some(index) = self.cached_detail_index {
            let repos = self.registry.list_repos();
            if let Some(repo_path) = repos.get(index) {
                if let Some(status) = self.registry.get_status(repo_path) {
                    let mut header_spans = Vec::new();

                    let branch = status
                        .branch
                        .as_ref()
                        .map_or_else(|| "[detached]".to_string(), Clone::clone);
                    header_spans.push(Span::styled(branch, Style::default().fg(Color::Cyan)));

                    let dirty_indicator = if status.is_dirty { " ●" } else { " ○" };
                    let dirty_color = if status.is_dirty {
                        Color::Yellow
                    } else {
                        Color::Green
                    };
                    header_spans.push(Span::styled(
                        dirty_indicator,
                        Style::default().fg(dirty_color),
                    ));

                    if let Some(ahead) = status.ahead.filter(|&n| n > 0) {
                        header_spans.push(Span::styled(
                            format!(" ↑{ahead}"),
                            Style::default().fg(Color::Green),
                        ));
                    }
                    if let Some(behind) = status.behind.filter(|&n| n > 0) {
                        header_spans.push(Span::styled(
                            format!(" ↓{behind}"),
                            Style::default().fg(Color::Red),
                        ));
                    }

                    lines.push(Line::from(header_spans));
                    lines.push(Line::from(""));
                }
            }
        }

        // Changed files section
        if detail.changed_files.is_empty() {
            lines.push(Line::from(Span::styled(
                "No uncommitted changes",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                format!("Changed Files ({})", detail.changed_files.len()),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));

            for change in &detail.changed_files {
                let (indicator, color) = format_file_change_indicator(&change.status);
                lines.push(Line::from(vec![
                    Span::styled(format!("  {indicator} "), Style::default().fg(color)),
                    Span::styled(change.path.clone(), Style::default().fg(Color::White)),
                ]));
            }
        }

        // Separator
        lines.push(Line::from(""));

        // Commits section
        if detail.commits.is_empty() {
            lines.push(Line::from(Span::styled(
                "No commits",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                format!("Recent Commits ({})", detail.commits.len()),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )));

            for commit in &detail.commits {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {} ", commit.hash),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::styled(commit.subject.clone(), Style::default().fg(Color::White)),
                ]));
                lines.push(Line::from(Span::styled(
                    format!("       {} - {}", commit.author, commit.relative_date),
                    Style::default().fg(Color::DarkGray),
                )));
            }
        }

        lines
    }
}
