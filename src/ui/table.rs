use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Row, Table, TableState},
    Frame,
};

use crate::app::AppState;
use crate::ui::theme::*;

pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
    let header_cells = [
        "   ", "Package", "Current", "Target", "Latest", "Kind", "Risk",
    ]
    .iter()
    .map(|h| Cell::from(*h).style(muted_style()));

    let header = Row::new(header_cells)
        .style(Style::default())
        .height(1)
        .bottom_margin(0);

    let rows: Vec<Row> = state
        .items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_focused = i == state.selected_row;

            let dot = if item.checked { "●" } else { "○" };
            let dot_style = if item.checked {
                selected_dot_style()
            } else {
                muted_style()
            };

            let focus_marker = if is_focused { "›" } else { " " };
            let focus_style = if is_focused {
                Style::default().fg(PRIMARY).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let sel_cell = Cell::from(Line::from(vec![
                Span::styled(focus_marker, focus_style),
                Span::raw(" "),
                Span::styled(dot, dot_style),
            ]));

            let name_style = if is_focused {
                focused_row_style()
            } else {
                text_style()
            };

            let name_cell = Cell::from(item.name.as_str()).style(name_style);

            let current_cell = Cell::from(item.current.as_str()).style(if is_focused {
                text_style()
            } else {
                muted_style()
            });

            let target_cell = Cell::from(item.target.as_str()).style(if is_focused {
                Style::default().fg(PRIMARY).add_modifier(Modifier::BOLD)
            } else {
                text_style()
            });

            let latest_cell = Cell::from(item.latest.as_str()).style(muted_style());

            let kind_cell = Cell::from(item.kind.label()).style(muted_style());

            let risk_cell = Cell::from(item.risk.label()).style(risk_style(item.risk));

            let row_style = if is_focused {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            Row::new(vec![
                sel_cell,
                name_cell,
                current_cell,
                target_cell,
                latest_cell,
                kind_cell,
                risk_cell,
            ])
            .style(row_style)
            .height(1)
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style());

    let table = Table::new(
        rows,
        [
            Constraint::Length(4),  // sel
            Constraint::Min(22),    // package
            Constraint::Length(10), // current
            Constraint::Length(10), // target
            Constraint::Length(10), // latest
            Constraint::Length(5),  // kind
            Constraint::Length(10), // risk
        ],
    )
    .header(header)
    .block(block)
    .row_highlight_style(Style::default());

    // Keep the focused row visible. ratatui's TableState manages the scroll
    // offset when we pass the selected index — it will clamp and scroll
    // automatically to ensure the selected row stays in view.
    let mut ts = TableState::default().with_selected(state.selected_row);
    f.render_stateful_widget(table, area, &mut ts);
}
