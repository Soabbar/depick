use ratatui::style::Style;
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};

use crate::app::AppState;
use crate::ui::theme::*;

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let mut lines: Vec<Line> = vec![];

    for log_line in &state.apply_log {
        let style = if log_line.contains("done") {
            Style::default().fg(SUCCESS)
        } else if log_line.contains("failed") || log_line.contains("error") {
            Style::default().fg(DANGER)
        } else if log_line.starts_with('[') {
            Style::default().fg(PRIMARY)
        } else {
            muted_style()
        };
        lines.push(Line::from(Span::styled(log_line.as_str(), style)));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(primary_border_style())
        .title(Line::from(Span::styled(
            " Applying updates ",
            title_style(),
        )));

    let para = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });

    f.render_widget(para, area);
}

pub fn render_done(f: &mut Frame, area: Rect, state: &AppState) {
    let mut lines: Vec<Line> = vec![];

    lines.push(Line::from(Span::styled(
        "Updated successfully",
        Style::default().fg(SUCCESS).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::raw(""));

    for line in &state.apply_log {
        if line.contains("done") {
            lines.push(Line::from(Span::styled(
                format!("  ✓  {}", line),
                Style::default().fg(SUCCESS),
            )));
        } else if line.contains("failed") {
            lines.push(Line::from(Span::styled(
                format!("  ✗  {}", line),
                Style::default().fg(DANGER),
            )));
        }
    }

    let remaining = state.items.iter().filter(|i| !i.checked).count();
    if remaining > 0 {
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            format!("Remaining outdated packages: {}", remaining),
            muted_style(),
        )));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(primary_border_style())
        .title(Line::from(Span::styled(" Done ", title_style())));

    let para = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });

    f.render_widget(para, area);
}

use ratatui::style::Modifier;
