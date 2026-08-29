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
    // 1. App Surfaces & Elevation
    // ==========================================

    /// Base canvas: Dark #121417, Light #F7F8FA
    pub fn bg_app() -> Hsla {
        Self::pick(hex(0x121417), hex(0xF7F8FA))
    }

    /// Titlebar / Header: Dark #1A1D23, Light #EEF0F3
    pub fn bg_titlebar() -> Hsla {
        Self::pick(hex(0x1A1D23), hex(0xEEF0F3))
    }

    /// Activity ribbon: Dark #16181D, Light #EEF0F3
    pub fn bg_ribbon() -> Hsla {
        Self::pick(hex(0x16181D), hex(0xEEF0F3))
    }

    /// Left sidebar panel: Dark #16181D, Light #F0F1F4
    pub fn bg_sidebar() -> Hsla {
        Self::pick(hex(0x16181D), hex(0xF0F1F4))
    }

    /// Editor writing surface: Dark #1E2128, Light #FFFFFF
    pub fn bg_editor() -> Hsla {
        Self::pick(hex(0x1E2128), hex(0xFFFFFF))
    }

    /// Line numbers gutter: Dark #191C22, Light #F5F6F8
    pub fn bg_gutter() -> Hsla {
        Self::pick(hex(0x191C22), hex(0xF5F6F8))
    }

    /// Current line highlight: Dark #262A33, Light #EAF2FE
    pub fn bg_current_line() -> Hsla {
        Self::pick(hex(0x262A33), hex(0xEAF2FE))
    }

    /// Tab bar background: Dark #16181D, Light #EEF0F3
    pub fn bg_tab_bar() -> Hsla {
        Self::pick(hex(0x16181D), hex(0xEEF0F3))
    }

    /// Active tab surface: Dark #1E2128, Light #FFFFFF
    pub fn bg_tab_active() -> Hsla {
        Self::pick(hex(0x1E2128), hex(0xFFFFFF))
    }

    /// Inactive tab surface: Dark #1A1D23, Light #E4E6EA
    pub fn bg_tab_inactive() -> Hsla {
        Self::pick(hex(0x1A1D23), hex(0xE4E6EA))
    }

    /// Cards & Panels: Dark #1E2128, Light #FFFFFF
    pub fn bg_card() -> Hsla {
        Self::pick(hex(0x1E2128), hex(0xFFFFFF))
    }

    pub fn bg_panel() -> Hsla {
        Self::pick(hex(0x1E2128), hex(0xFFFFFF))
    }

    /// Modal containers: Dark #1C1F26, Light #FFFFFF
    pub fn bg_modal() -> Hsla {
        Self::pick(hex(0x1C1F26), hex(0xFFFFFF))
    }

    /// Modal backdrop dimming: Dark rgba(0,0,0,0.65), Light rgba(0,0,0,0.25)
    pub fn modal_backdrop() -> Hsla {
        Self::pick(
            hsla(0.0, 0.0, 0.0, 0.65),
            hsla(0.0, 0.0, 0.0, 0.25),
        )
    }

    /// Hover state: Dark rgba(255,255,255,0.06), Light rgba(0,0,0,0.04)
    pub fn bg_hover() -> Hsla {
        Self::pick(
            hsla(0.0, 0.0, 1.0, 0.06),
            hsla(0.0, 0.0, 0.0, 0.04),
        )
    }

    /// Active / Pressed state: Dark rgba(255,255,255,0.10), Light rgba(0,0,0,0.08)
    pub fn bg_active() -> Hsla {
        Self::pick(
            hsla(0.0, 0.0, 1.0, 0.10),
            hsla(0.0, 0.0, 0.0, 0.08),
        )
    }

    // ==========================================
    // 2. Typography & Foreground
    // ==========================================

    /// Primary body text: Dark #E4E6EB, Light #1F2328
    pub fn text_primary() -> Hsla {
        Self::pick(hex(0xE4E6EB), hex(0x1F2328))
    }

    /// Bright headings / Active titles: Dark #FFFFFF, Light #0B0D10
    pub fn text_bright() -> Hsla {
        Self::pick(hex(0xFFFFFF), hex(0x0B0D10))
    }

    /// Muted labels / secondary text: Dark #9DA3AE, Light #5B6270
    pub fn text_muted() -> Hsla {
        Self::pick(hex(0x9DA3AE), hex(0x5B6270))
    }

    /// Dim text / comments / placeholders: Dark #5C6370, Light #8A909C
    pub fn text_dim() -> Hsla {
        Self::pick(hex(0x5C6370), hex(0x8A909C))
    }

    /// Inverted button text (on solid accent backgrounds): Dark #0B0D10, Light #FFFFFF
    pub fn text_inverted() -> Hsla {
        Self::pick(hex(0x0B0D10), hex(0xFFFFFF))
    }

    // ==========================================
    // 3. LaTeX Syntax Tokens
    // ==========================================

    /// Commands (\section, \textbf, \cite, \ref): Dark #62AEEF, Light #1A73E8
    pub fn syn_command() -> Hsla {
        Self::pick(hex(0x62AEEF), hex(0x1A73E8))
    }

    /// Environments (\begin{...}, \end{...}): Dark #E5C07B, Light #B58500
    pub fn syn_environment() -> Hsla {
        Self::pick(hex(0xE5C07B), hex(0xB58500))
    }

    /// Math mode ($...$, \[...\]): Dark #98C379, Light #2E7D32
    pub fn syn_math() -> Hsla {
        Self::pick(hex(0x98C379), hex(0x2E7D32))
    }

    /// Comments (% ...): Dark #5C6370, Light #8A909C
    pub fn syn_comment() -> Hsla {
        Self::pick(hex(0x5C6370), hex(0x8A909C))
    }

    /// Optional args ([12pt], [h!]): Dark #56B6C2, Light #0E8A8A
    pub fn syn_optional_arg() -> Hsla {
        Self::pick(hex(0x56B6C2), hex(0x0E8A8A))
    }

    /// Brackets ({ } [ ] ( )): Dark #C678DD, Light #8B3FA8
    pub fn syn_bracket() -> Hsla {
        Self::pick(hex(0xC678DD), hex(0x8B3FA8))
    }

    /// Special characters (& _ ^ # ~ \\): Dark #D19A66, Light #C1580C
    pub fn syn_special() -> Hsla {
        Self::pick(hex(0xD19A66), hex(0xC1580C))
    }

    // ==========================================
    // 4. Editor Interaction & Caret
    // ==========================================

    /// Cursor / Caret: Dark #FFFFFF, Light #1A73E8
    pub fn caret() -> Hsla {
        Self::pick(hex(0xFFFFFF), hex(0x1A73E8))
    }

    /// Text Selection Fill: Dark rgba(98,174,239,0.25), Light rgba(26,115,232,0.18)
    pub fn selection() -> Hsla {
        Self::pick(
            hexa(0x62AEEF, 0.25),
            hexa(0x1A73E8, 0.18),
        )
    }

    /// Find match - active match background: Dark rgba(229,192,123,0.35), Light rgba(181,133,0,0.35)
    pub fn find_match_active_bg() -> Hsla {
        Self::pick(
            hexa(0xE5C07B, 0.35),
            hexa(0xB58500, 0.35),
        )
    }

    /// Find match - active match border: Dark #E5C07B, Light #B58500
    pub fn find_match_active_border() -> Hsla {
        Self::pick(hex(0xE5C07B), hex(0xB58500))
    }

    /// Find match - other occurrences background: Dark rgba(229,192,123,0.15), Light rgba(181,133,0,0.15)
    pub fn find_match_other_bg() -> Hsla {
        Self::pick(
            hexa(0xE5C07B, 0.15),
            hexa(0xB58500, 0.15),
        )
    }

    /// Gutter line number - active: Dark #E4E6EB, Light #1F2328
    pub fn line_num_active() -> Hsla {
        Self::pick(hex(0xE4E6EB), hex(0x1F2328))
    }

    /// Gutter line number - inactive: Dark #5C6370, Light #8A909C
    pub fn line_num_inactive() -> Hsla {
        Self::pick(hex(0x5C6370), hex(0x8A909C))
    }

    // ==========================================
    // 5. Borders, Dividers & Focus
    // ==========================================

    /// Subtle borders / dividers: Dark #2A2E37, Light #E2E5E9
    pub fn border_subtle() -> Hsla {
        Self::pick(hex(0x2A2E37), hex(0xE2E5E9))
    }

    /// Focus outline: Dark #62AEEF, Light #1A73E8
    pub fn border_focus() -> Hsla {
        Self::pick(hex(0x62AEEF), hex(0x1A73E8))
    }

    /// Active tab indicator: Dark #62AEEF, Light #1A73E8
    pub fn tab_accent() -> Hsla {
        Self::pick(hex(0x62AEEF), hex(0x1A73E8))
    }

    // ==========================================
    // 6. Semantic Accents & Status
    // ==========================================

    /// Blue accent (links, sync, references): Dark #62AEEF, Light #1A73E8
    pub fn accent_blue() -> Hsla {
        Self::pick(hex(0x62AEEF), hex(0x1A73E8))
    }

    /// Green accent (build success, saved, ready): Dark #98C379, Light #2E7D32
    pub fn accent_green() -> Hsla {
        Self::pick(hex(0x98C379), hex(0x2E7D32))
    }

    /// Yellow accent (building/in-progress, warnings): Dark #E5C07B, Light #B58500
    pub fn accent_yellow() -> Hsla {
        Self::pick(hex(0xE5C07B), hex(0xB58500))
    }

    /// Orange accent (snippets, secondary warnings): Dark #D19A66, Light #C1580C
    pub fn accent_orange() -> Hsla {
        Self::pick(hex(0xD19A66), hex(0xC1580C))
    }

    /// Red accent (compiler errors, delete/close): Dark #E06C75, Light #D32F2F
    pub fn accent_red() -> Hsla {
        Self::pick(hex(0xE06C75), hex(0xD32F2F))
    }

    /// Purple accent (BibTeX citations, structure): Dark #C678DD, Light #8B3FA8
    pub fn accent_purple() -> Hsla {
        Self::pick(hex(0xC678DD), hex(0x8B3FA8))
    }

    /// Cyan accent (params, file paths, tooltips): Dark #56B6C2, Light #0E8A8A
    pub fn accent_cyan() -> Hsla {
        Self::pick(hex(0x56B6C2), hex(0x0E8A8A))
    }

    // ==========================================
    // 7. Component Overlays & Custom Badges
    // ==========================================

    /// Autocomplete popup background: Dark #23262E, Light #FFFFFF
    pub fn completion_popup_bg() -> Hsla {
        Self::pick(hex(0x23262E), hex(0xFFFFFF))
    }

    /// Autocomplete selected item: Dark rgba(98,174,239,0.15), Light rgba(26,115,232,0.10)
    pub fn completion_selected_item_bg() -> Hsla {
        Self::pick(
            hexa(0x62AEEF, 0.15),
            hexa(0x1A73E8, 0.10),
        )
    }

    /// PDF backing canvas (behind white pages): Dark #0D0E11, Light #E4E6EA
    pub fn pdf_backing_canvas() -> Hsla {
        Self::pick(hex(0x0D0E11), hex(0xE4E6EA))
    }

    /// PDF page border: Dark #2A2E37, Light #D7DAE0
    pub fn pdf_page_border() -> Hsla {
        Self::pick(hex(0x2A2E37), hex(0xD7DAE0))
    }

    /// PDF SyncTeX highlight overlay: Dark rgba(98,174,239,0.30), Light rgba(26,115,232,0.22)
    pub fn pdf_synctex_highlight() -> Hsla {
        Self::pick(
            hexa(0x62AEEF, 0.30),
            hexa(0x1A73E8, 0.22),
        )
    }

    /// PDF text search highlight: Dark rgba(229,192,123,0.40), Light rgba(181,133,0,0.30)
    pub fn pdf_search_highlight() -> Hsla {
        Self::pick(
            hexa(0xE5C07B, 0.40),
            hexa(0xB58500, 0.30),
        )
    }

    /// Project selector card hover glow: Dark rgba(98,174,239,0.20), Light rgba(26,115,232,0.12)
    pub fn card_hover_glow() -> Hsla {
        Self::pick(
            hexa(0x62AEEF, 0.20),
            hexa(0x1A73E8, 0.12),
        )
    }

    /// Table Editor code preview box background: Dark #16181D, Light #F5F6F8
    pub fn table_code_preview_bg() -> Hsla {
        Self::pick(hex(0x16181D), hex(0xF5F6F8))
    }
}
