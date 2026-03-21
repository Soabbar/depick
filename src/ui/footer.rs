use ratatui::style::Style;
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::{AppState, Phase};
use crate::ui::theme::*;

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let line = match state.phase {
        Phase::Ready => {
            let selected = state.selected_count();
            let prompt = if selected == 0 {
                "Select packages to include in the update plan"
            } else {
                "Review selections and press x to apply"
            };
            Line::from(vec![
                Span::styled("❯ ", Style::default().fg(PRIMARY)),
                Span::styled(prompt, text_style()),
            ])
        }

        Phase::Reviewing => Line::from(vec![
            Span::styled("[enter]", key_style()),
            Span::styled(" apply   ", muted_style()),
            Span::styled("[b]", key_style()),
            Span::styled(" back   ", muted_style()),
            Span::styled("[q]", key_style()),
            Span::styled(" quit", muted_style()),
        ]),

        Phase::Applying => Line::from(vec![Span::styled(
            "❯ Updating manifest and lockfile…",
            Style::default().fg(PRIMARY),
        )]),

        Phase::Done => Line::from(vec![
            Span::styled("[q]", key_style()),
            Span::styled(" quit", muted_style()),
        ]),

        Phase::Scanning => Line::from(vec![Span::styled(
            "❯ Scanning dependencies…",
            Style::default().fg(PRIMARY),
        )]),
    };

    f.render_widget(Paragraph::new(line), area);
}

pub fn render_keys(f: &mut Frame, area: Rect, state: &AppState) {
    let mut spans = vec![
        Span::styled("[↑↓]", key_style()),
        Span::styled(" move  ", muted_style()),
        Span::styled("[space]", key_style()),
        Span::styled(" toggle  ", muted_style()),
        Span::styled("[a]", key_style()),
        Span::styled(" all  ", muted_style()),
        Span::styled("[i]", key_style()),
        Span::styled(" invert  ", muted_style()),
        Span::styled("[t]", key_style()),
        Span::styled(" target  ", muted_style()),
        Span::styled("[w]", key_style()),
        Span::styled(" write  ", muted_style()),
        Span::styled("[o]", key_style()),
        Span::styled(" open link  ", muted_style()),
    ];

    // Show [m] expand hint only when notes are actually loaded for this row
    if state.has_loaded_notes() {
        spans.push(Span::styled("[m]", key_style()));
        spans.push(Span::styled(" expand  ", muted_style()));
    }

    spans.extend([
        Span::styled("[x]", key_style()),
        Span::styled(" apply  ", muted_style()),
        Span::styled("[q]", key_style()),
        Span::styled(" quit", muted_style()),
    ]);

    f.render_widget(Paragraph::new(Line::from(spans)), area);
}
