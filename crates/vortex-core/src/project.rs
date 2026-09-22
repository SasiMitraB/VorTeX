//! Project-level decisions shared by building, SyncTeX and latexdiff:
//! which file is the main document, and which TeX engine compiles it.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

fn is_tex(p: &Path) -> bool {
    p.extension().map(|e| e.eq_ignore_ascii_case("tex")).unwrap_or(false)
}

/// The document to compile: the active file if it is a full document (or names one via
/// `%!TEX root`), else `main.tex`, else the first root-level file with `\documentclass`.
///
/// `project` is the project folder; for a lone file pass its parent folder.
pub fn main_document(active: Option<&Path>, project: &Path) -> Option<PathBuf> {
    if let Some(active) = active.filter(|p| is_tex(p)) {
        if let Ok(text) = std::fs::read_to_string(active) {
            if let Some(root) = magic_root(&text) {
                let p = active.parent().unwrap_or(project).join(root);
                if p.is_file() {
                    return Some(p);
                }
            }
            if is_full_document(&text) {
                return Some(active.to_path_buf());
            }
        }
    }
    let main = project.join("main.tex");
    if main.is_file() {
        return Some(main);
    }
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(project)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && is_tex(p))
        .collect();
    candidates.sort();
    candidates
        .into_iter()
        .find(|p| std::fs::read_to_string(p).map(|t| is_full_document(&t)).unwrap_or(false))
}

/// Removes `.` and `..` components without touching the disk
/// (`/p/./ch/../main.tex` → `/p/main.tex`), as TeX and SyncTeX report paths.
pub fn normalize_path(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

/// The PDF that compiling `tex` produces (same folder, same stem).
pub fn pdf_for(tex: &Path) -> PathBuf {
    tex.with_extension("pdf")
}

fn is_full_document(text: &str) -> bool {
    text.lines().any(|l| l.trim_start().starts_with("\\documentclass"))
}

/// `% !TEX root = ../main.tex` (any capitalization of `TEX`) in the first 20 lines.
fn magic_root(text: &str) -> Option<String> {
    text.lines().take(20).find_map(|l| {
        let l = l.trim_start().strip_prefix('%')?.trim_start().strip_prefix('!')?.trim_start();
        let (tex, rest) = l.split_at_checked(3)?;
        if !tex.eq_ignore_ascii_case("tex") {
            return None;
        }
        let rest = rest.trim_start().strip_prefix("root")?.trim_start().strip_prefix('=')?;
        let root = rest.trim();
        (!root.is_empty()).then(|| root.to_string())
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum Engine {
    #[serde(rename = "pdflatex")]
    Pdf,
    #[serde(rename = "xelatex")]
    Xe,
    #[serde(rename = "lualatex")]
    Lua,
}

impl Engine {
    pub fn latexmk_flag(self) -> &'static str {
        match self {
            Engine::Pdf => "-pdf",
            Engine::Xe => "-xelatex",
            Engine::Lua => "-lualatex",
        }
    }

    /// Display name, e.g. for the toolbar.
    pub fn name(self) -> &'static str {
        match self {
            Engine::Pdf => "pdfLaTeX",
            Engine::Xe => "XeLaTeX",
            Engine::Lua => "LuaLaTeX",
        }
    }
}

/// Honors `% !TEX program = …`; otherwise packages that need a Unicode engine pick XeLaTeX.
pub fn detect_engine(source: &str) -> Engine {
    for line in source.lines().take(30) {
        let l = line.to_ascii_lowercase();
        if l.trim_start().starts_with('%') && l.contains("program") && l.contains('=') {
            if l.contains("lualatex") {
                return Engine::Lua;
            }
            if l.contains("xelatex") {
                return Engine::Xe;
            }
            if l.contains("pdflatex") {
                return Engine::Pdf;
            }
        }
    }
    let uncommented = source.lines().map(|l| l.split('%').next().unwrap_or("")).collect::<Vec<_>>().join("\n");
    if ["{fontspec}", "{unicode-math}", "{polyglossia}"].iter().any(|p| uncommented.contains(p)) {
        Engine::Xe
    } else {
        Engine::Pdf
    }
}

/// Engine for the file at `path` (pdfLaTeX if it can't be read).
pub fn detect_engine_for(path: &Path) -> Engine {
    std::fs::read_to_string(path).map(|s| detect_engine(&s)).unwrap_or(Engine::Pdf)
}
