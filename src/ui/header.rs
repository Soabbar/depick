use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::app::AppState;
use crate::ui::theme::*;

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let pm = state.package_manager.label();
    let target = state.target_mode.label();
    let write = state.write_mode.label();
    let outdated = state.items.len();
    let selected = state.selected_count();

    let info_line = Line::from(vec![
        Span::styled("PM: ", muted_style()),
        Span::styled(pm, text_style()),
        Span::raw("   "),
        Span::styled("Target: ", muted_style()),
        Span::styled(target, text_style()),
        Span::raw("   "),
        Span::styled("Write: ", muted_style()),
        Span::styled(write, text_style()),
        Span::raw("   "),
        Span::styled("Outdated: ", muted_style()),
        Span::styled(outdated.to_string(), text_style()),
        Span::raw("   "),
        Span::styled("Selected: ", muted_style()),
        Span::styled(
            selected.to_string(),
            if selected > 0 {
                selected_dot_style()
            } else {
                muted_style()
            },
        ),
    ]);

    let project_line = Line::from(vec![
        Span::styled("Project: ", muted_style()),
        Span::styled(&state.project_name, text_style()),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(primary_border_style())
        .title(Line::from(vec![
            Span::styled(" depick ", title_style()),
            Span::styled("── selective dependency updates ", muted_style()),
        ]));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Render two lines inside the block
    let project = Paragraph::new(project_line).style(text_style());
    let info = Paragraph::new(info_line).style(text_style());

    if inner.height >= 2 {
        f.render_widget(project, Rect { height: 1, ..inner });
        f.render_widget(
            info,
            Rect {
                y: inner.y + 1,
                height: 1,
                ..inner
            },
        );
    }
}
