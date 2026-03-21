use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use the_other_tui_markdown::into_text_with_renderer;

use crate::app::AppState;
use crate::domain::release::ReleaseContextState;
use crate::ui::theme::*;

// ── Public entry points ───────────────────────────────────────────────────────

/// Right panel (45% of body height): release notes preview.
pub fn render_notes(f: &mut Frame, area: Rect, state: &AppState) {
    let Some(item) = state.selected_item() else {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style());
        f.render_widget(block, area);
        return;
    };

    let key = crate::domain::release::ReleaseCacheKey::from_item(item);
    let ctx_state = state.release_contexts.entries.get(&key);

    // Package name + version change as the panel title
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled(item.name.as_str(), title_style()),
        Span::raw("  "),
        Span::styled(item.planned_change(), muted_style()),
        Span::raw(" "),
    ]);

    let has_notes = matches!(
        ctx_state,
        Some(ReleaseContextState::Loaded(ctx)) if ctx.release_body_preview.is_some()
    );

    let block = {
        let b = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style())
            .title(title);

        if has_notes {
            b.title_bottom(Line::from(vec![
                Span::raw(" "),
                Span::styled("[m]", key_style()),
                Span::styled(" expand ", muted_style()),
            ]))
        } else {
            b
        }
    };

    let inner = block.inner(area);
    f.render_widget(block, area);

    match ctx_state {
        Some(ReleaseContextState::Loaded(ctx)) => {
            // Build the preview markdown text.
            // If there's a release title we prepend it as a h2 so the
            // markdown renderer gives it the right weight.
            let md_src = match (&ctx.release_title, &ctx.release_body_preview) {
                (Some(t), Some(b)) => format!("## {t}\n\n{b}"),
                (Some(t), None) => format!("## {t}"),
                (None, Some(b)) => b.clone(),
                (None, None) => String::new(),
            };

            if md_src.is_empty() {
                let para = Paragraph::new(Line::from(Span::styled(
                    "No structured release notes found.",
                    muted_style(),
                )));
                f.render_widget(para, inner);
                return;
            }

            let text = into_text_with_renderer(&md_src, &state.md_renderer);

            // Count rendered lines to decide whether to show the overflow hint.
            let rendered_lines = text.lines.len();
            let available = inner.height as usize;

            // If it fits, just render it directly.
            if rendered_lines <= available {
                let para = Paragraph::new(text).wrap(Wrap { trim: false });
                f.render_widget(para, inner);
                return;
            }

            // Clip to (available - 1) lines and append the overflow hint.
            let mut clipped: Vec<Line<'static>> = text
                .lines
                .into_iter()
                .take(available.saturating_sub(1))
                .collect();

            let remaining = rendered_lines - available.saturating_sub(1);
            clipped.push(Line::from(vec![
                Span::styled(format!("  … {remaining} more lines"), muted_style()),
                Span::raw("  "),
                Span::styled("[m]", key_style()),
                Span::styled(" expand", muted_style()),
            ]));

            let para = Paragraph::new(clipped).wrap(Wrap { trim: false });
            f.render_widget(para, inner);
        }

        Some(ReleaseContextState::Failed(err)) => {
            let para = Paragraph::new(Line::from(vec![
                Span::styled("error  ", muted_style()),
                Span::styled(
                    err.as_str(),
                    risk_style(crate::domain::risk::RiskLevel::Major),
                ),
            ]));
            f.render_widget(para, inner);
        }

        // No entry yet — blank panel (all fetches are done at startup,
        // so this is only possible if an item has no cache key, which
        // should never happen in practice).
        None => {}
    }
}

/// Full-width detail bar below the body:
///   line 1 — name · planned change · risk · kind
///   line 2 — command
///   line 3 — release / compare / homepage URL (or error)
pub fn render_detail_bar(f: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style());

    let inner = block.inner(area);
    f.render_widget(block, area);

    let Some(item) = state.selected_item() else {
        return;
    };

    let mut lines: Vec<Line> = vec![];

    // ── Line 1: name · change · risk · kind ──────────────────────────────
    lines.push(Line::from(vec![
        Span::styled(item.name.as_str(), title_style()),
        Span::styled("  ·  ", muted_style()),
        Span::styled(item.planned_change(), text_style()),
        Span::styled("  ·  ", muted_style()),
        Span::styled(
            item.risk.label(),
            risk_style(item.risk).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  ·  ", muted_style()),
        Span::styled(item.kind.label(), muted_style()),
    ]));

    // ── Line 2: install command ───────────────────────────────────────────
    let cmd = item.install_command(&state.package_manager, &state.write_mode);
    lines.push(Line::from(vec![
        Span::styled("command  ", muted_style()),
        Span::styled(cmd, Style::default().fg(PRIMARY)),
    ]));

    // ── Line 3: link or error ─────────────────────────────────────────────
    let key = crate::domain::release::ReleaseCacheKey::from_item(item);
    match state.release_contexts.entries.get(&key) {
        Some(ReleaseContextState::Loaded(ctx)) => {
            // Prefer release URL, then compare, then changelog/homepage
            let (label, url) = if let Some(u) = &ctx.release_url {
                ("releases ", u.as_str())
            } else if let Some(u) = &ctx.compare_url {
                ("compare  ", u.as_str())
            } else if let Some(u) = &ctx.changelog_url {
                ("homepage ", u.as_str())
            } else {
                ("", "")
            };

            if !label.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled(label, muted_style()),
                    Span::styled(url, text_style()),
                ]));
            } else {
                lines.push(Line::raw(""));
            }
        }

        Some(ReleaseContextState::Failed(err)) => {
            lines.push(Line::from(vec![
                Span::styled("error  ", muted_style()),
                Span::styled(
                    err.as_str(),
                    risk_style(crate::domain::risk::RiskLevel::Major),
                ),
            ]));
        }

        None => {
            lines.push(Line::raw(""));
        }
    }

    let para = Paragraph::new(lines).wrap(Wrap { trim: true });
    f.render_widget(para, inner);
}

/// Full-screen modal overlay — complete release notes with scroll.
/// Call after all other renders so it paints on top.
pub fn render_modal(f: &mut Frame, state: &AppState) {
    let area = f.area();

    // Centred box: 90% wide, 85% tall
    let vpad = ((area.height as f32) * 0.075) as u16;
    let hpad = ((area.width as f32) * 0.05) as u16;
    let modal_area = Rect {
        x: area.x + hpad,
        y: area.y + vpad,
        width: area.width.saturating_sub(hpad * 2),
        height: area.height.saturating_sub(vpad * 2),
    };

    f.render_widget(Clear, modal_area);

    let Some(item) = state.selected_item() else {
        return;
    };

    let key = crate::domain::release::ReleaseCacheKey::from_item(item);
    let md_src = match state.release_contexts.entries.get(&key) {
        Some(ReleaseContextState::Loaded(ctx)) => {
            match (&ctx.release_title, &ctx.release_body_full) {
                (Some(t), Some(b)) => format!("## {t}\n\n{b}"),
                (Some(t), None) => format!("## {t}"),
                (None, Some(b)) => b.clone(),
                (None, None) => "No structured release notes found.".to_string(),
            }
        }
        _ => return,
    };

    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled(item.name.as_str(), title_style()),
        Span::raw("  "),
        Span::styled(item.planned_change(), muted_style()),
        Span::raw(" "),
    ]);

    let footer_hint = Line::from(vec![
        Span::raw(" "),
        Span::styled("[↑↓]", key_style()),
        Span::styled(" scroll  ", muted_style()),
        Span::styled("[PgUp/PgDn]", key_style()),
        Span::styled(" page  ", muted_style()),
        Span::styled("[m]", key_style()),
        Span::styled(" close ", muted_style()),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(PRIMARY))
        .title(title)
        .title_bottom(footer_hint);

    let inner = block.inner(modal_area);
    f.render_widget(block, modal_area);

    let text = into_text_with_renderer(&md_src, &state.md_renderer);

    let para = Paragraph::new(text)
        .wrap(Wrap { trim: false })
        .scroll((state.modal_scroll, 0));

    f.render_widget(para, inner);
}
