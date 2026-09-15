#![allow(dead_code)]

use gpui::*;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ThemePreference {
    #[default]
    Auto,
    Dark,
    Light,
}

impl ThemePreference {
    pub fn next(&self) -> Self {
        match self {
            ThemePreference::Auto => ThemePreference::Light,
            ThemePreference::Light => ThemePreference::Dark,
            ThemePreference::Dark => ThemePreference::Auto,
        }
    }

    pub fn label(&self, active_mode: ThemeMode) -> String {
        match self {
            ThemePreference::Auto => {
                if active_mode.is_dark() {
                    "🌓 Auto (Dark)".to_string()
                } else {
                    "🌓 Auto (Light)".to_string()
                }
            }
            ThemePreference::Dark => "🌙 Dark".to_string(),
            ThemePreference::Light => "☀️ Light".to_string(),
        }
    }
}

pub fn detect_system_theme() -> ThemeMode {
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("defaults")
            .args(["read", "-g", "AppleInterfaceStyle"])
            .output()
        {
            if output.status.success() {
                let s = String::from_utf8_lossy(&output.stdout);
                if s.trim().eq_ignore_ascii_case("dark") {
                    return ThemeMode::Dark;
                }
            }
        }
        ThemeMode::Light
    }
    #[cfg(not(target_os = "macos"))]
    {
        ThemeMode::Dark
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ThemeMode {
    #[default]
    Dark = 0,
    Light = 1,
}

static CURRENT_THEME_MODE: AtomicU8 = AtomicU8::new(0); // 0 = Dark, 1 = Light

impl ThemeMode {
    pub fn is_dark(&self) -> bool {
        matches!(self, ThemeMode::Dark)
    }

    pub fn is_light(&self) -> bool {
        matches!(self, ThemeMode::Light)
    }

    pub fn toggled(&self) -> Self {
        match self {
            ThemeMode::Dark => ThemeMode::Light,
            ThemeMode::Light => ThemeMode::Dark,
        }
    }
}

#[inline]
fn hex(v: u32) -> Hsla {
    rgb(v).into()
}

#[inline]
fn hexa(v: u32, a: f32) -> Hsla {
    let mut c: Hsla = rgb(v).into();
    c.a = a;
    c
}

pub struct Theme;

impl Theme {
    /// Get the current active theme mode
    pub fn mode() -> ThemeMode {
        match CURRENT_THEME_MODE.load(Ordering::Relaxed) {
            1 => ThemeMode::Light,
            _ => ThemeMode::Dark,
        }
    }

    /// Set the active theme mode
    pub fn set_mode(mode: ThemeMode) {
        let val = match mode {
            ThemeMode::Dark => 0,
            ThemeMode::Light => 1,
        };
        CURRENT_THEME_MODE.store(val, Ordering::Relaxed);
    }

    /// Toggle between Dark and Light modes
    pub fn toggle_mode() -> ThemeMode {
        let next = Self::mode().toggled();
        Self::set_mode(next);
        next
    }

    // Helper to pick color depending on active mode
    #[inline]
    fn pick(dark_color: Hsla, light_color: Hsla) -> Hsla {
        if Self::mode().is_dark() {
            dark_color
        } else {
            light_color
        }
    }

    // ==========================================
    // Catppuccin Core Palette Primitives
    // ==========================================

    // Catppuccin Mocha (Dark)
    pub const MOCHA_CRUST: u32 = 0x11111b;
    pub const MOCHA_MANTLE: u32 = 0x181825;
    pub const MOCHA_BASE: u32 = 0x1e1e2e;
    pub const MOCHA_SURFACE0: u32 = 0x313244;
    pub const MOCHA_SURFACE1: u32 = 0x45475a;
    pub const MOCHA_SURFACE2: u32 = 0x585b70;
    pub const MOCHA_OVERLAY0: u32 = 0x6c7086;
    pub const MOCHA_OVERLAY1: u32 = 0x7f849c;
    pub const MOCHA_OVERLAY2: u32 = 0x9399b2;
    pub const MOCHA_SUBTEXT0: u32 = 0xa6adc8;
    pub const MOCHA_SUBTEXT1: u32 = 0xbac2de;
    pub const MOCHA_TEXT: u32 = 0xcdd6f4;
    pub const MOCHA_BLUE: u32 = 0x89b4fa;
    pub const MOCHA_LAVENDER: u32 = 0xb4befe;
    pub const MOCHA_SAPPHIRE: u32 = 0x74c7ec;
    pub const MOCHA_SKY: u32 = 0x89dceb;
    pub const MOCHA_TEAL: u32 = 0x94e2d5;
    pub const MOCHA_GREEN: u32 = 0xa6e3a1;
    pub const MOCHA_YELLOW: u32 = 0xf9e2af;
    pub const MOCHA_PEACH: u32 = 0xfab387;
    pub const MOCHA_MAROON: u32 = 0xeba0ac;
    pub const MOCHA_RED: u32 = 0xf38ba8;
    pub const MOCHA_MAUVE: u32 = 0xcba6f7;
    pub const MOCHA_PINK: u32 = 0xf5c2e7;
    pub const MOCHA_FLAMINGO: u32 = 0xf2cdcd;
    pub const MOCHA_ROSEWATER: u32 = 0xf5e0dc;

    // Catppuccin Latte (Light)
    pub const LATTE_CRUST: u32 = 0xdce0e8;
    pub const LATTE_MANTLE: u32 = 0xe6e9ef;
    pub const LATTE_BASE: u32 = 0xeff1f5;
    pub const LATTE_SURFACE0: u32 = 0xccd0da;
    pub const LATTE_SURFACE1: u32 = 0xbcc0cc;
    pub const LATTE_SURFACE2: u32 = 0xacb0be;
    pub const LATTE_OVERLAY0: u32 = 0x9ca0b0;
    pub const LATTE_OVERLAY1: u32 = 0x8c8fa1;
    pub const LATTE_OVERLAY2: u32 = 0x7c7f93;
    pub const LATTE_SUBTEXT0: u32 = 0x6c6f85;
    pub const LATTE_SUBTEXT1: u32 = 0x5c5f77;
    pub const LATTE_TEXT: u32 = 0x4c4f69;
    pub const LATTE_BLUE: u32 = 0x1e66f5;
    pub const LATTE_LAVENDER: u32 = 0x7287fd;
    pub const LATTE_SAPPHIRE: u32 = 0x209fb5;
    pub const LATTE_SKY: u32 = 0x04a5e5;
    pub const LATTE_TEAL: u32 = 0x179299;
    pub const LATTE_GREEN: u32 = 0x40a02b;
    pub const LATTE_YELLOW: u32 = 0xdf8e1d;
    pub const LATTE_PEACH: u32 = 0xfe640b;
    pub const LATTE_MAROON: u32 = 0xe64553;
    pub const LATTE_RED: u32 = 0xd20f39;
    pub const LATTE_MAUVE: u32 = 0x8839ef;
    pub const LATTE_PINK: u32 = 0xea76cb;
    pub const LATTE_FLAMINGO: u32 = 0xdd7878;
    pub const LATTE_ROSEWATER: u32 = 0xdc8a78;

    // ==========================================
    // 1. App Surfaces & Elevation (Catppuccin Hierarchy)
    // ==========================================

    /// Outer window & status bar canvas: Mocha Crust #11111B, Latte Crust #DCE0E8
    pub fn bg_app() -> Hsla {
        Self::pick(hex(Self::MOCHA_CRUST), hex(Self::LATTE_CRUST))
    }

    /// Titlebar / Header: Mocha Mantle #181825, Latte Mantle #E6E9EF
    pub fn bg_titlebar() -> Hsla {
        Self::pick(hex(Self::MOCHA_MANTLE), hex(Self::LATTE_MANTLE))
    }

    /// Activity ribbon: Mocha Mantle #181825, Latte Mantle #E6E9EF
    pub fn bg_ribbon() -> Hsla {
        Self::pick(hex(Self::MOCHA_MANTLE), hex(Self::LATTE_MANTLE))
    }

    /// Left sidebar panel: Mocha Mantle #181825, Latte Mantle #E6E9EF
    pub fn bg_sidebar() -> Hsla {
        Self::pick(hex(Self::MOCHA_MANTLE), hex(Self::LATTE_MANTLE))
    }

    /// Editor writing surface: Mocha Base #1E1E2E, Latte Base #EFF1F5
    pub fn bg_editor() -> Hsla {
        Self::pick(hex(Self::MOCHA_BASE), hex(Self::LATTE_BASE))
    }

    /// Line numbers gutter: Mocha Mantle #181825, Latte Mantle #E6E9EF
    pub fn bg_gutter() -> Hsla {
        Self::pick(hex(Self::MOCHA_MANTLE), hex(Self::LATTE_MANTLE))
    }

    /// Current line highlight: Subtle Surface0 tint rgba(49, 50, 68, 0.5) / Latte rgba(204, 208, 218, 0.4)
    pub fn bg_current_line() -> Hsla {
        Self::pick(
            hexa(Self::MOCHA_SURFACE0, 0.50),
            hexa(Self::LATTE_SURFACE0, 0.40),
        )
    }

    /// Tab bar background: Mocha Mantle #181825, Latte Mantle #E6E9EF
    pub fn bg_tab_bar() -> Hsla {
        Self::pick(hex(Self::MOCHA_MANTLE), hex(Self::LATTE_MANTLE))
    }

    /// Active tab surface: Mocha Base #1E1E2E, Latte Base #EFF1F5
    pub fn bg_tab_active() -> Hsla {
        Self::pick(hex(Self::MOCHA_BASE), hex(Self::LATTE_BASE))
    }

    /// Inactive tab surface: Transparent or Mantle
    pub fn bg_tab_inactive() -> Hsla {
        Self::pick(hex(Self::MOCHA_MANTLE), hex(Self::LATTE_MANTLE))
    }

    /// Cards & Panels: Mocha Surface0 #313244 or Base #1E1E2E
    pub fn bg_card() -> Hsla {
        Self::pick(hex(Self::MOCHA_SURFACE0), hex(Self::LATTE_SURFACE0))
    }

    pub fn bg_panel() -> Hsla {
        Self::pick(hex(Self::MOCHA_SURFACE0), hex(Self::LATTE_SURFACE0))
    }

    /// Modal containers: Mocha Base #1E1E2E, Latte Base #EFF1F5
    pub fn bg_modal() -> Hsla {
        Self::pick(hex(Self::MOCHA_BASE), hex(Self::LATTE_BASE))
    }

    /// Modal backdrop dimming
    pub fn modal_backdrop() -> Hsla {
        Self::pick(
            hsla(0.0, 0.0, 0.0, 0.70),
            hsla(0.0, 0.0, 0.0, 0.30),
        )
    }

    /// Hover state: Surface0/40 or Surface1/40
    pub fn bg_hover() -> Hsla {
        Self::pick(
            hexa(Self::MOCHA_SURFACE0, 0.60),
            hexa(Self::LATTE_SURFACE1, 0.40),
        )
    }

    /// Active / Pressed state: Surface1
    pub fn bg_active() -> Hsla {
        Self::pick(
            hex(Self::MOCHA_SURFACE1),
            hex(Self::LATTE_SURFACE1),
        )
    }

    // ==========================================
    // 2. Typography & Foreground
    // ==========================================

    /// Primary body text: Mocha Text #CDD6F4, Latte Text #4C4F69
    pub fn text_primary() -> Hsla {
        Self::pick(hex(Self::MOCHA_TEXT), hex(Self::LATTE_TEXT))
    }

    /// Bright headings / Active titles: Mocha Text #CDD6F4, Latte Text #4C4F69
    pub fn text_bright() -> Hsla {
        Self::pick(hex(Self::MOCHA_TEXT), hex(Self::LATTE_TEXT))
    }

    /// Subtext / Secondary: Mocha Subtext1 #BAC2DE, Latte Subtext1 #5C5F77
    pub fn text_secondary() -> Hsla {
        Self::pick(hex(Self::MOCHA_SUBTEXT1), hex(Self::LATTE_SUBTEXT1))
    }

    /// Muted labels: Mocha Subtext0 #A6ADC8, Latte Subtext0 #6C6F85
    pub fn text_muted() -> Hsla {
        Self::pick(hex(Self::MOCHA_SUBTEXT0), hex(Self::LATTE_SUBTEXT0))
    }

    /// Dim text / line numbers / comments: Mocha Overlay0 #6C7086, Latte Overlay0 #9CA0B0
    pub fn text_dim() -> Hsla {
        Self::pick(hex(Self::MOCHA_OVERLAY0), hex(Self::LATTE_OVERLAY0))
    }

    /// Inverted button text (on blue/accent background): Mocha Crust #11111B, Latte Crust #EFF1F5
    pub fn text_inverted() -> Hsla {
        Self::pick(hex(Self::MOCHA_CRUST), hex(0xFFFFFF))
    }

    // ==========================================
    // 3. Authentic Catppuccin LaTeX Syntax Tokens
    // ==========================================

    /// Commands (\section, \title, \author, \textbf): Catppuccin Mauve
    pub fn syn_command() -> Hsla {
        Self::pick(hex(Self::MOCHA_MAUVE), hex(Self::LATTE_MAUVE))
    }

    /// Environments (\begin{document}, \end{abstract}): Catppuccin Green
    pub fn syn_environment() -> Hsla {
        Self::pick(hex(Self::MOCHA_GREEN), hex(Self::LATTE_GREEN))
    }

    /// Math mode ($...$, \[...\]): Catppuccin Yellow
    pub fn syn_math() -> Hsla {
        Self::pick(hex(Self::MOCHA_YELLOW), hex(Self::LATTE_YELLOW))
    }

    /// Comments (% ...): Catppuccin Overlay0
    pub fn syn_comment() -> Hsla {
        Self::pick(hex(Self::MOCHA_OVERLAY0), hex(Self::LATTE_OVERLAY0))
    }

    /// Optional args ([12pt], [h!], \href): Catppuccin Teal
    pub fn syn_optional_arg() -> Hsla {
        Self::pick(hex(Self::MOCHA_TEAL), hex(Self::LATTE_TEAL))
    }

    /// Brackets ({ } [ ] ( )): Catppuccin Red
    pub fn syn_bracket() -> Hsla {
        Self::pick(hex(Self::MOCHA_RED), hex(Self::LATTE_RED))
    }

    /// Special characters / Macros (& _ ^ # ~ \\): Catppuccin Peach
    pub fn syn_special() -> Hsla {
        Self::pick(hex(Self::MOCHA_PEACH), hex(Self::LATTE_PEACH))
    }

    /// Citations, Labels, References: Catppuccin Sapphire
    pub fn syn_reference() -> Hsla {
        Self::pick(hex(Self::MOCHA_SAPPHIRE), hex(Self::LATTE_SAPPHIRE))
    }

    // ==========================================
    // 4. Editor Interaction & Caret
    // ==========================================

    /// Cursor / Caret: Catppuccin Blue
    pub fn caret() -> Hsla {
        Self::pick(hex(Self::MOCHA_BLUE), hex(Self::LATTE_BLUE))
    }

    /// Text Selection Fill: Catppuccin Surface1 with alpha
    pub fn selection() -> Hsla {
        Self::pick(
            hexa(Self::MOCHA_SURFACE1, 0.70),
            hexa(Self::LATTE_SURFACE1, 0.50),
        )
    }

    /// Find match - active match background
    pub fn find_match_active_bg() -> Hsla {
        Self::pick(
            hexa(Self::MOCHA_YELLOW, 0.40),
            hexa(Self::LATTE_YELLOW, 0.40),
        )
    }

    /// Find match - active match border
    pub fn find_match_active_border() -> Hsla {
        Self::pick(hex(Self::MOCHA_YELLOW), hex(Self::LATTE_YELLOW))
    }

    /// Find match - other occurrences background
    pub fn find_match_other_bg() -> Hsla {
        Self::pick(
            hexa(Self::MOCHA_YELLOW, 0.18),
            hexa(Self::LATTE_YELLOW, 0.18),
        )
    }

    /// Gutter line number - active: Catppuccin Blue
    pub fn line_num_active() -> Hsla {
        Self::pick(hex(Self::MOCHA_BLUE), hex(Self::LATTE_BLUE))
    }

    /// Gutter line number - inactive: Catppuccin Overlay0
    pub fn line_num_inactive() -> Hsla {
        Self::pick(hex(Self::MOCHA_OVERLAY0), hex(Self::LATTE_OVERLAY0))
    }

    // ==========================================
    // 5. Borders, Dividers & Focus
    // ==========================================

    /// Subtle borders / dividers: Catppuccin Surface0
    pub fn border_subtle() -> Hsla {
        Self::pick(hex(Self::MOCHA_SURFACE0), hex(Self::LATTE_SURFACE0))
    }

    /// Focus outline: Catppuccin Blue
    pub fn border_focus() -> Hsla {
        Self::pick(hex(Self::MOCHA_BLUE), hex(Self::LATTE_BLUE))
    }

    /// Active tab top border: Catppuccin Blue
    pub fn tab_accent() -> Hsla {
        Self::pick(hex(Self::MOCHA_BLUE), hex(Self::LATTE_BLUE))
    }

    // ==========================================
    // 6. Semantic Accents & Status (Catppuccin Palette)
    // ==========================================

    /// Blue: Primary accent, links, buttons, active highlights
    pub fn accent_blue() -> Hsla {
        Self::pick(hex(Self::MOCHA_BLUE), hex(Self::LATTE_BLUE))
    }

    /// Green: Build success, saved, ready status
    pub fn accent_green() -> Hsla {
        Self::pick(hex(Self::MOCHA_GREEN), hex(Self::LATTE_GREEN))
    }

    /// Yellow: Building in-progress, warnings, math
    pub fn accent_yellow() -> Hsla {
        Self::pick(hex(Self::MOCHA_YELLOW), hex(Self::LATTE_YELLOW))
    }

    /// Peach / Orange: TODO chips, warnings, bibTeX
    pub fn accent_peach() -> Hsla {
        Self::pick(hex(Self::MOCHA_PEACH), hex(Self::LATTE_PEACH))
    }

    pub fn accent_orange() -> Hsla {
        Self::accent_peach()
    }

    /// Red / Maroon: Errors, brackets, deletions
    pub fn accent_red() -> Hsla {
        Self::pick(hex(Self::MOCHA_RED), hex(Self::LATTE_RED))
    }

    /// Mauve / Purple: Commands, tags, structure
    pub fn accent_mauve() -> Hsla {
        Self::pick(hex(Self::MOCHA_MAUVE), hex(Self::LATTE_MAUVE))
    }

    pub fn accent_purple() -> Hsla {
        Self::accent_mauve()
    }

    /// Teal / Mint: Tables, macro links, NOTE pills
    pub fn accent_teal() -> Hsla {
        Self::pick(hex(Self::MOCHA_TEAL), hex(Self::LATTE_TEAL))
    }

    pub fn accent_cyan() -> Hsla {
        Self::accent_teal()
    }

    /// Sapphire: Clean secondary blue
    pub fn accent_sapphire() -> Hsla {
        Self::pick(hex(Self::MOCHA_SAPPHIRE), hex(Self::LATTE_SAPPHIRE))
    }

    /// Lavender
    pub fn accent_lavender() -> Hsla {
        Self::pick(hex(Self::MOCHA_LAVENDER), hex(Self::LATTE_LAVENDER))
    }

    // ==========================================
    // 7. Component Overlays & Custom Badges
    // ==========================================

    /// Autocomplete popup background: Catppuccin Surface0
    pub fn completion_popup_bg() -> Hsla {
        Self::pick(hex(Self::MOCHA_SURFACE0), hex(Self::LATTE_SURFACE0))
    }

    /// Autocomplete selected item: Surface1
    pub fn completion_selected_item_bg() -> Hsla {
        Self::pick(
            hex(Self::MOCHA_SURFACE1),
            hex(Self::LATTE_SURFACE1),
        )
    }

    /// PDF backing canvas (behind paper sheet): Catppuccin Mantle #181825
    pub fn pdf_backing_canvas() -> Hsla {
        Self::pick(hex(Self::MOCHA_MANTLE), hex(Self::LATTE_MANTLE))
    }

    /// PDF page border: Catppuccin Surface0
    pub fn pdf_page_border() -> Hsla {
        Self::pick(hex(Self::MOCHA_SURFACE0), hex(Self::LATTE_SURFACE0))
    }

    /// PDF SyncTeX highlight overlay: Catppuccin Blue glow rgba(137, 180, 250, 0.22)
    pub fn pdf_synctex_highlight() -> Hsla {
        Self::pick(
            hexa(Self::MOCHA_BLUE, 0.22),
            hexa(Self::LATTE_BLUE, 0.18),
        )
    }

    /// PDF text search highlight: Catppuccin Yellow with alpha
    pub fn pdf_search_highlight() -> Hsla {
        Self::pick(
            hexa(Self::MOCHA_YELLOW, 0.35),
            hexa(Self::LATTE_YELLOW, 0.30),
        )
    }

    /// Project selector card hover glow: Catppuccin Blue
    pub fn card_hover_glow() -> Hsla {
        Self::pick(
            hexa(Self::MOCHA_BLUE, 0.25),
            hexa(Self::LATTE_BLUE, 0.15),
        )
    }

    /// Table Editor code preview box background: Catppuccin Crust #11111B
    pub fn table_code_preview_bg() -> Hsla {
        Self::pick(hex(Self::MOCHA_CRUST), hex(Self::LATTE_CRUST))
    }
}
