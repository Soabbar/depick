use ratatui::style::{Color, Modifier, Style};
use the_other_tui_markdown::{Renderer, RendererBuilder, Theme};

// ── Brand ─────────────────────────────────────────────────────────────────────
pub const PRIMARY: Color = Color::Rgb(0x21, 0x51, 0xBF);
#[allow(dead_code)]
pub const PRIMARY_DIM: Color = Color::Rgb(0x18, 0x3A, 0x8A);

// ── Neutrals ──────────────────────────────────────────────────────────────────
pub const TEXT: Color = Color::Rgb(0xEA, 0xEC, 0xF1);
pub const MUTED: Color = Color::Rgb(0x7B, 0x81, 0x8C);
pub const BORDER: Color = Color::Rgb(0x3A, 0x40, 0x4D);

// ── Semantic ──────────────────────────────────────────────────────────────────
pub const SUCCESS: Color = Color::Rgb(0x2E, 0xA0, 0x43);
pub const WARNING: Color = Color::Rgb(0xD9, 0x8E, 0x04);
pub const DANGER: Color = Color::Rgb(0xC2, 0x3B, 0x22);
pub const ZERO_MINOR: Color = Color::Rgb(0xD9, 0x6E, 0x00);

// ── Styles ────────────────────────────────────────────────────────────────────

pub fn title_style() -> Style {
    Style::default().fg(PRIMARY).add_modifier(Modifier::BOLD)
}

pub fn muted_style() -> Style {
    Style::default().fg(MUTED)
}

pub fn text_style() -> Style {
    Style::default().fg(TEXT)
}

pub fn focused_row_style() -> Style {
    Style::default().fg(TEXT).add_modifier(Modifier::BOLD)
}

pub fn key_style() -> Style {
    Style::default().fg(PRIMARY).add_modifier(Modifier::BOLD)
}

pub fn selected_dot_style() -> Style {
    Style::default().fg(SUCCESS).add_modifier(Modifier::BOLD)
}

pub fn risk_style(risk: crate::domain::risk::RiskLevel) -> Style {
    use crate::domain::risk::RiskLevel::*;
    match risk {
        Patch => Style::default().fg(SUCCESS),
        Minor => Style::default().fg(WARNING),
        ZeroMinor => Style::default().fg(ZERO_MINOR),
        Major => Style::default().fg(DANGER).add_modifier(Modifier::BOLD),
        Unknown => Style::default().fg(MUTED),
    }
}

pub fn border_style() -> Style {
    Style::default().fg(BORDER)
}

pub fn primary_border_style() -> Style {
    Style::default().fg(PRIMARY)
}

// ── Markdown renderer ─────────────────────────────────────────────────────────

/// Build a `the-other-tui-markdown` `Renderer` styled to match depick's palette.
/// Call once and store the result; `Renderer` is `Send + Sync` so it can live
/// in `AppState` or a `OnceLock`.
pub fn md_renderer() -> Renderer {
    let theme = Theme {
        // Base prose text
        base: Style::default().fg(TEXT),

        // Headings — h1 primary+bold, h2 text+bold, h3+ muted+bold
        h1: Style::default().fg(PRIMARY).add_modifier(Modifier::BOLD),
        h2: Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
        h3: Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
        h4: Style::default().fg(MUTED).add_modifier(Modifier::BOLD),
        h5: Style::default().fg(MUTED).add_modifier(Modifier::BOLD),
        h6: Style::default().fg(MUTED),

        // Inline emphasis
        strong: Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
        emphasis: Style::default().fg(TEXT).add_modifier(Modifier::ITALIC),
        strikethrough: Style::default()
            .fg(MUTED)
            .add_modifier(Modifier::CROSSED_OUT),

        // Code
        inline_code: Style::default().fg(WARNING),
        code_block: Style::default().fg(WARNING),
        code_block_lang: Style::default().fg(MUTED),

        // Links — primary colour so they stand out but are not jarring
        link: Style::default().fg(PRIMARY),
        image: Style::default().fg(MUTED),

        // Block-quote variants (GFM alerts)
        block_quote: Style::default().fg(MUTED),
        block_quote_note: Style::default().fg(PRIMARY),
        block_quote_tip: Style::default().fg(SUCCESS),
        block_quote_warning: Style::default().fg(WARNING),
        block_quote_caution: Style::default().fg(ZERO_MINOR),
        block_quote_important: Style::default().fg(DANGER),

        // List bullets/numbers
        list_marker: Style::default().fg(PRIMARY),

        // Tables
        table_header: Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
        table_cell: Style::default().fg(TEXT),
        table_separator: Style::default().fg(MUTED),

        // Misc
        rule: Style::default().fg(MUTED),
        footnote_ref: Style::default().fg(MUTED).add_modifier(Modifier::DIM),
        footnote_def: Style::default().fg(MUTED).add_modifier(Modifier::DIM),
        math: Style::default().fg(MUTED),
        html: Style::default().fg(MUTED),

        ..Default::default()
    };

    RendererBuilder::new().with_theme(theme).build()
}
