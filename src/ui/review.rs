use ratatui::style::Style;
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};

use crate::app::AppState;
use crate::domain::risk::RiskLevel;
use crate::ui::theme::*;

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let checked = state.checked_items();

    let mut lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled("Project: ", muted_style()),
            Span::styled(&state.project_name, text_style()),
            Span::raw("   "),
            Span::styled("PM: ", muted_style()),
            Span::styled(state.package_manager.label(), text_style()),
        ]),
        Line::raw(""),
        Line::from(Span::styled(
            "The following changes will be made:",
            muted_style(),
        )),
        Line::raw(""),
    ];

    for (i, item) in checked.iter().enumerate() {
        let cmd = item.install_command(&state.package_manager, &state.write_mode);

        lines.push(Line::from(vec![
            Span::styled(format!("  {}. ", i + 1), muted_style()),
            Span::styled(&item.name, title_style()),
            Span::raw("  "),
            Span::styled(item.risk.label(), risk_style(item.risk)),
        ]));

        lines.push(Line::from(vec![
            Span::raw("     "),
            Span::styled(item.planned_change(), text_style()),
        ]));

        lines.push(Line::from(vec![
            Span::raw("     "),
            Span::styled(cmd.clone(), Style::default().fg(PRIMARY)),
        ]));

        lines.push(Line::raw(""));
    }

    // Summary counts
    let patch = checked
        .iter()
        .filter(|i| i.risk == RiskLevel::Patch)
        .count();
    let minor = checked
        .iter()
        .filter(|i| matches!(i.risk, RiskLevel::Minor | RiskLevel::ZeroMinor))
        .count();
    let major = checked
        .iter()
        .filter(|i| i.risk == RiskLevel::Major)
        .count();

    lines.push(Line::from(Span::styled("Summary", muted_style())));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(format!("Selected: {}  ", checked.len()), text_style()),
        Span::styled(format!("Patch: {}  ", patch), risk_style(RiskLevel::Patch)),
        Span::styled(format!("Minor: {}  ", minor), risk_style(RiskLevel::Minor)),
        Span::styled(format!("Major: {}", major), risk_style(RiskLevel::Major)),
    ]));

    // Risky warning
    if minor > 0 || major > 0 {
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "  ⚠  Review minor/major updates carefully before applying.",
            Style::default().fg(WARNING),
        )));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(primary_border_style())
        .title(Line::from(Span::styled(
            " Apply update plan ",
            title_style(),
        )));

    let para = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });

    f.render_widget(para, area);
}

pub fn render_footer(f: &mut Frame, area: Rect) {
    let line = Line::from(vec![
        Span::styled("[enter]", key_style()),
        Span::styled(" apply   ", muted_style()),
        Span::styled("[b]", key_style()),
        Span::styled(" back   ", muted_style()),
        Span::styled("[q]", key_style()),
        Span::styled(" cancel", muted_style()),
    ]);
    let para = Paragraph::new(line);
    f.render_widget(para, area);
}
