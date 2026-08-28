#![allow(dead_code)]

use gpui::*;

pub struct Theme;

impl Theme {
    // Backgrounds (Atom One Dark inspired)
    pub fn bg_app() -> Hsla {
        hsla(220.0 / 360.0, 0.13, 0.14, 1.0) // #21252b
    }

    pub fn bg_titlebar() -> Hsla {
        hsla(220.0 / 360.0, 0.13, 0.12, 1.0) // #1d2026
    }

    pub fn bg_sidebar() -> Hsla {
        hsla(220.0 / 360.0, 0.13, 0.15, 1.0) // #23272e
    }

    pub fn bg_panel() -> Hsla {
        hsla(220.0 / 360.0, 0.13, 0.18, 1.0) // #282c34
    }

    pub fn bg_editor() -> Hsla {
        hsla(220.0 / 360.0, 0.13, 0.18, 1.0) // #282c34
    }

    pub fn bg_gutter() -> Hsla {
        hsla(220.0 / 360.0, 0.13, 0.16, 1.0) // #242830
    }

    pub fn bg_tab_bar() -> Hsla {
        hsla(220.0 / 360.0, 0.13, 0.13, 1.0) // #1f2329
    }

    pub fn bg_tab_active() -> Hsla {
        hsla(220.0 / 360.0, 0.13, 0.18, 1.0) // #282c34
    }

    pub fn bg_tab_inactive() -> Hsla {
        hsla(220.0 / 360.0, 0.13, 0.13, 1.0) // #1f2329
    }

    pub fn bg_card() -> Hsla {
        hsla(220.0 / 360.0, 0.13, 0.20, 1.0) // #2e333d
    }

    pub fn bg_modal() -> Hsla {
        hsla(220.0 / 360.0, 0.14, 0.17, 1.0) // #252932
    }

    pub fn bg_hover() -> Hsla {
        hsla(220.0 / 360.0, 0.13, 0.23, 1.0) // #353b45
    }

    pub fn bg_active() -> Hsla {
        hsla(220.0 / 360.0, 0.20, 0.28, 1.0) // #3a424e
    }

    pub fn bg_current_line() -> Hsla {
        hsla(220.0 / 360.0, 0.13, 0.21, 1.0) // #30353f
    }

    // Text & Foreground
    pub fn text_primary() -> Hsla {
        hsla(220.0 / 360.0, 0.14, 0.71, 1.0) // #abb2bf
    }

    pub fn text_bright() -> Hsla {
        hsla(0.0, 0.0, 0.95, 1.0) // #f2f2f2
    }

    pub fn text_muted() -> Hsla {
        hsla(220.0 / 360.0, 0.09, 0.55, 1.0) // #7f848e
    }

    pub fn text_dim() -> Hsla {
        hsla(220.0 / 360.0, 0.08, 0.40, 1.0) // #5c6370
    }

    // Accent Colors
    pub fn accent_blue() -> Hsla {
        hsla(207.0 / 360.0, 0.82, 0.66, 1.0) // #61afef
    }

    pub fn accent_green() -> Hsla {
        hsla(95.0 / 360.0, 0.38, 0.62, 1.0) // #98c379
    }

    pub fn accent_purple() -> Hsla {
        hsla(286.0 / 360.0, 0.60, 0.67, 1.0) // #c678dd
    }

    pub fn accent_yellow() -> Hsla {
        hsla(39.0 / 360.0, 0.67, 0.69, 1.0) // #e5c07b
    }

    pub fn accent_orange() -> Hsla {
        hsla(29.0 / 360.0, 0.54, 0.61, 1.0) // #d19a66
    }

    pub fn accent_red() -> Hsla {
        hsla(355.0 / 360.0, 0.65, 0.65, 1.0) // #e06c75
    }

    pub fn accent_cyan() -> Hsla {
        hsla(187.0 / 360.0, 0.47, 0.55, 1.0) // #56b6c2
    }

    // Borders
    pub fn border_subtle() -> Hsla {
        hsla(220.0 / 360.0, 0.13, 0.22, 1.0) // #333842
    }

    pub fn border_focus() -> Hsla {
        hsla(207.0 / 360.0, 0.82, 0.66, 1.0) // #61afef
    }

    pub fn selection() -> Hsla {
        hsla(210.0 / 360.0, 0.70, 0.50, 0.35) // Rich translucent blue highlight
    }

    // Syntax Highlighting Tokens (LaTeX specific)
    pub fn syn_command() -> Hsla {
        Self::accent_blue()
    }

    pub fn syn_environment() -> Hsla {
        Self::accent_yellow()
    }

    pub fn syn_math() -> Hsla {
        Self::accent_green()
    }

    pub fn syn_comment() -> Hsla {
        Self::text_dim()
    }

    pub fn syn_optional_arg() -> Hsla {
        Self::accent_cyan()
    }

    pub fn syn_bracket() -> Hsla {
        Self::accent_purple()
    }

    pub fn syn_special() -> Hsla {
        Self::accent_orange()
    }
}
