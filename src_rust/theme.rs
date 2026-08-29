#![allow(dead_code)]

use gpui::*;

pub struct Theme;

impl Theme {
    // Material-inspired dark surfaces. Keep elevation visible through small,
    // consistent changes in lightness instead of heavy borders.
    pub fn bg_app() -> Hsla {
        hsla(225.0 / 360.0, 0.18, 0.09, 1.0) // #12141a
    }

    pub fn bg_titlebar() -> Hsla {
        hsla(225.0 / 360.0, 0.17, 0.11, 1.0) // #171920
    }

    pub fn bg_sidebar() -> Hsla {
        hsla(225.0 / 360.0, 0.16, 0.13, 1.0) // #1b1e25
    }

    pub fn bg_panel() -> Hsla {
        hsla(225.0 / 360.0, 0.15, 0.16, 1.0) // #22262e
    }

    pub fn bg_editor() -> Hsla {
        hsla(225.0 / 360.0, 0.15, 0.16, 1.0) // #22262e
    }

    pub fn bg_gutter() -> Hsla {
        hsla(225.0 / 360.0, 0.15, 0.14, 1.0) // #1e2229
    }

    pub fn bg_tab_bar() -> Hsla {
        hsla(225.0 / 360.0, 0.17, 0.10, 1.0) // #15181e
    }

    pub fn bg_tab_active() -> Hsla {
        hsla(225.0 / 360.0, 0.15, 0.16, 1.0) // #22262e
    }

    pub fn bg_tab_inactive() -> Hsla {
        hsla(225.0 / 360.0, 0.17, 0.10, 1.0) // #15181e
    }

    pub fn bg_card() -> Hsla {
        hsla(225.0 / 360.0, 0.15, 0.20, 1.0) // #2a2f39
    }

    pub fn bg_modal() -> Hsla {
        hsla(225.0 / 360.0, 0.16, 0.18, 1.0) // #262a33
    }

    pub fn bg_hover() -> Hsla {
        hsla(225.0 / 360.0, 0.16, 0.25, 1.0) // #353c49
    }

    pub fn bg_active() -> Hsla {
        hsla(225.0 / 360.0, 0.22, 0.31, 1.0) // #414b5e
    }

    pub fn bg_current_line() -> Hsla {
        hsla(225.0 / 360.0, 0.15, 0.19, 1.0) // #292e38
    }

    // Text & Foreground
    pub fn text_primary() -> Hsla {
        hsla(220.0 / 360.0, 0.12, 0.78, 1.0) // #c4c9d4
    }

    pub fn text_bright() -> Hsla {
        hsla(220.0 / 360.0, 0.18, 0.97, 1.0) // #f2f4fa
    }

    pub fn text_muted() -> Hsla {
        hsla(220.0 / 360.0, 0.10, 0.60, 1.0) // #9299a8
    }

    pub fn text_dim() -> Hsla {
        hsla(220.0 / 360.0, 0.09, 0.46, 1.0) // #6d7482
    }

    // Accent Colors
    pub fn accent_blue() -> Hsla {
        hsla(222.0 / 360.0, 0.90, 0.76, 1.0) // #9ab5ff
    }

    pub fn accent_green() -> Hsla {
        hsla(145.0 / 360.0, 0.55, 0.68, 1.0) // #7fe0a7
    }

    pub fn accent_purple() -> Hsla {
        hsla(265.0 / 360.0, 0.78, 0.78, 1.0) // #c5a9ff
    }

    pub fn accent_yellow() -> Hsla {
        hsla(42.0 / 360.0, 0.84, 0.76, 1.0) // #f4c978
    }

    pub fn accent_orange() -> Hsla {
        hsla(26.0 / 360.0, 0.76, 0.70, 1.0) // #ecaa78
    }

    pub fn accent_red() -> Hsla {
        hsla(355.0 / 360.0, 0.76, 0.72, 1.0) // #f18b97
    }

    pub fn accent_cyan() -> Hsla {
        hsla(190.0 / 360.0, 0.70, 0.70, 1.0) // #72d7e3
    }

    // Borders
    pub fn border_subtle() -> Hsla {
        hsla(225.0 / 360.0, 0.14, 0.22, 1.0) // #30353f
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
