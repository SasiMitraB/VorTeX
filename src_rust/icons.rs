//! Embedded SVG icon set (Lucide, ISC license — see `assets/icons/LICENSE`).
//!
//! Icons are monochrome and take their color from `text_color`, e.g.
//! `icon(IconName::Folder).size(px(14.0)).text_color(Theme::accent_yellow())`.

use gpui::{px, svg, AssetSource, Result, SharedString, Styled, Svg};
use std::borrow::Cow;

macro_rules! icons {
    ($($variant:ident => $file:literal,)*) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum IconName {
            $($variant,)*
        }

        impl IconName {
            pub fn path(self) -> &'static str {
                match self {
                    $(Self::$variant => concat!("icons/", $file, ".svg"),)*
                }
            }
        }

        const ICON_FILES: &[(&str, &[u8])] = &[
            $((
                concat!("icons/", $file, ".svg"),
                include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/icons/", $file, ".svg")),
            ),)*
        ];
    };
}

icons! {
    ArrowRightLeft => "arrow-right-left",
    BookMarked => "book-marked",
    Check => "check",
    ChevronDown => "chevron-down",
    ChevronRight => "chevron-right",
    CircleCheck => "circle-check",
    Code => "code",
    Columns2 => "columns-2",
    ExternalLink => "external-link",
    File => "file",
    FileCode => "file-code",
    FileCog => "file-cog",
    FileImage => "file-image",
    FileJson => "file-json",
    FilePlus => "file-plus",
    FileText => "file-text",
    Files => "files",
    Folder => "folder",
    FolderOpen => "folder-open",
    FolderSearch => "folder-search",
    GitBranch => "git-branch",
    GitCommitHorizontal => "git-commit-horizontal",
    GitCompare => "git-compare",
    Hash => "hash",
    Heading => "heading",
    History => "history",
    Image => "image",
    Layers => "layers",
    Link => "link",
    ListTodo => "list-todo",
    ListTree => "list-tree",
    LoaderCircle => "loader-circle",
    Minus => "minus",
    PanelLeft => "panel-left",
    PanelLeftClose => "panel-left-close",
    Play => "play",
    Plus => "plus",
    Quote => "quote",
    RotateCw => "rotate-cw",
    Sigma => "sigma",
    Sparkles => "sparkles",
    Table => "table",
    Table2 => "table-2",
    Tag => "tag",
    X => "x",
}

/// Asset source serving the embedded icons to GPUI's SVG renderer.
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(ICON_FILES
            .iter()
            .find(|(p, _)| *p == path)
            .map(|(_, bytes)| Cow::Borrowed(*bytes)))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(ICON_FILES
            .iter()
            .filter(|(p, _)| p.starts_with(path))
            .map(|(p, _)| SharedString::from(*p))
            .collect())
    }
}

/// A 14px icon; override with `.size(..)` and color with `.text_color(..)`.
pub fn icon(name: IconName) -> Svg {
    svg().path(name.path()).flex_none().size(px(14.0))
}

/// Icon and accent color for a file in the explorer / tab bar, keyed by extension.
pub fn file_icon(file_name: &str) -> (IconName, gpui::Hsla) {
    use crate::theme::Theme;
    let ext = file_name.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "tex" | "ltx" => (IconName::FileCode, Theme::accent_sapphire()),
        "bib" => (IconName::BookMarked, Theme::accent_peach()),
        "pdf" => (IconName::FileText, Theme::accent_red()),
        "md" | "txt" => (IconName::FileText, Theme::accent_cyan()),
        "json" => (IconName::FileJson, Theme::accent_orange()),
        "py" | "js" | "ts" => (IconName::FileCode, Theme::accent_yellow()),
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "eps" | "ps" => (IconName::FileImage, Theme::accent_teal()),
        "cls" | "sty" | "bst" => (IconName::FileCog, Theme::accent_mauve()),
        _ => (IconName::File, Theme::text_dim()),
    }
}
