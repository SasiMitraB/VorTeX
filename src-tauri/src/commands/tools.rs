//! latexdiff, grammar checking, math previews and the table editor.

use crate::state::{blocking, CmdResult, Shared};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::Path;
use tauri::State;
use vortex_core::git::{self, Commit};
use vortex_core::grammar_checker::{run_grammar_check, GrammarDialect};
use vortex_core::math_preview::{self, MathRenderRequest, RenderedMath};
use vortex_core::settings::Settings;
use vortex_core::table_editor::{self, TableOp};
use vortex_core::table_parser::{generate_latex_table, TableModel};
use vortex_core::text::{char_col_to_utf16, utf16_to_char_col};
use vortex_core::{latexdiff, project};

// ── latexdiff ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LatexDiffOptions {
    pub repo_root: String,
    /// Repo-relative project folder that each version is taken from ("" = whole repo).
    pub scope: String,
    /// Repo-relative main document, if one was found.
    pub main_rel: Option<String>,
    /// Commits touching the project, newest first.
    pub commits: Vec<Commit>,
}

struct DiffTarget {
    root: std::path::PathBuf,
    scope: String,
    main_rel: Option<String>,
}

fn diff_target(state: &Shared, active: Option<&str>) -> CmdResult<DiffTarget> {
    let project = state.require_project()?;
    let root = git::repo_root(&project).ok_or("The project is not in a git repository")?;
    let scope = git::relative_path(&root, &project).unwrap_or_default();
    let main_rel = project::main_document(active.map(Path::new), &project).and_then(|m| git::relative_path(&root, &m));
    Ok(DiffTarget { root, scope, main_rel })
}

/// What the "Compare versions" dialog offers. Fails when the project isn't under git.
#[tauri::command]
#[specta::specta]
pub async fn latexdiff_options(state: State<'_, Shared>, active: Option<String>) -> CmdResult<LatexDiffOptions> {
    let state = state.inner().clone();
    blocking(move || {
        let t = diff_target(&state, active.as_deref())?;
        Ok(LatexDiffOptions {
            commits: git::log(&t.root, &t.scope, 200),
            repo_root: t.root.to_string_lossy().to_string(),
            scope: t.scope,
            main_rel: t.main_rel,
        })
    })
    .await
}

/// A commit hash or `HEAD`; anything else could reach git as an option.
fn check_rev(rev: &str) -> CmdResult<()> {
    let is_hash = (4..=64).contains(&rev.len()) && rev.chars().all(|c| c.is_ascii_hexdigit());
    if is_hash || rev == "HEAD" {
        Ok(())
    } else {
        Err(format!("Not a commit: {rev}"))
    }
}

/// Builds a change-tracking PDF of `old_rev` → `new_rev` (`null` = working copy, saved to disk)
/// next to the main document, and returns its path.
#[tauri::command]
#[specta::specta]
pub async fn latexdiff(
    state: State<'_, Shared>,
    active: Option<String>,
    old_rev: String,
    new_rev: Option<String>,
) -> CmdResult<String> {
    check_rev(&old_rev)?;
    if let Some(rev) = &new_rev {
        check_rev(rev)?;
    }
    let state = state.inner().clone();
    blocking(move || {
        let t = diff_target(&state, active.as_deref())?;
        let main_rel = t.main_rel.ok_or("No main document found")?;
        let name = latexdiff::output_name(&main_rel, &old_rev, new_rev.as_deref());
        let main_dir = t.root.join(&main_rel).parent().map(Path::to_path_buf).unwrap_or_else(|| t.root.clone());
        let req = latexdiff::LatexDiffRequest {
            repo_root: t.root,
            scope: t.scope,
            main_rel,
            old_rev,
            new_rev,
            output_pdf: main_dir.join(name),
        };
        latexdiff::generate(&req).map(|p| p.to_string_lossy().to_string())
    })
    .await
}

// ── Grammar ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct GrammarIssue {
    /// 0-based line.
    pub line: usize,
    /// UTF-16 columns on that line.
    pub from: u32,
    pub to: u32,
    pub message: String,
}

/// Grammar and spelling issues in `text` (a whole `.tex` buffer). `dialect` defaults to the setting.
/// Citations are read as author–year text using the project's bibliography.
#[tauri::command]
#[specta::specta]
pub async fn grammar_check(
    state: State<'_, Shared>,
    text: String,
    dialect: Option<GrammarDialect>,
) -> CmdResult<Vec<GrammarIssue>> {
    let backend = state.backend.clone();
    blocking(move || {
        let dialect = dialect.unwrap_or_else(|| Settings::load().grammar_dialect);
        let bib = backend.get_all_bib_entries();
        let lines: Vec<&str> = text.lines().collect();
        Ok(run_grammar_check(&text, &bib, dialect)
            .into_iter()
            .map(|d| {
                let line = lines.get(d.row).copied().unwrap_or("");
                GrammarIssue {
                    line: d.row,
                    from: char_col_to_utf16(line, d.col_start),
                    to: char_col_to_utf16(line, d.col_end),
                    message: d.message,
                }
            })
            .collect())
    })
    .await
}

// ── Math preview ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MathAt {
    /// 0-based lines and UTF-16 columns; `to` is exclusive.
    pub from_line: usize,
    pub from_col: u32,
    pub to_line: usize,
    pub to_col: u32,
    /// "inline math", "display math" or the environment name.
    pub label: String,
    /// Standalone LaTeX for `math_render`.
    pub snippet: String,
}

/// The math span (`$…$`, `\[…\]`, `equation`, `align`, …) containing the position, if any.
#[tauri::command]
#[specta::specta]
pub async fn math_at(text: String, line: usize, col: u32) -> CmdResult<Option<MathAt>> {
    blocking(move || {
        let lines: Vec<String> = text.lines().map(str::to_string).collect();
        let Some(cur) = lines.get(line) else {
            return Ok(None);
        };
        let spans = math_preview::find_math_spans(&lines);
        let Some(span) = math_preview::math_span_at(&spans, line, utf16_to_char_col(cur, col)) else {
            return Ok(None);
        };
        let at = |row: usize, c: usize| char_col_to_utf16(lines.get(row).map(String::as_str).unwrap_or(""), c);
        Ok(Some(MathAt {
            from_line: span.start.0,
            from_col: at(span.start.0, span.start.1),
            to_line: span.end.0,
            to_col: at(span.end.0, span.end.1 + 1),
            label: span.label(),
            snippet: span.snippet(),
        }))
    })
    .await
}

/// Typesets `snippet` (from `math_at`) with the macros and math packages of `text`,
/// in `color` (`RRGGBB`). Returns a PNG under `~/.vortex-editor/math_cache`; results are cached.
#[tauri::command]
#[specta::specta]
pub async fn math_render(text: String, file: Option<String>, snippet: String, color: String) -> CmdResult<RenderedMath> {
    if color.len() != 6 || !color.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("Not an RRGGBB color: {color}"));
    }
    blocking(move || {
        let preamble = math_preview::build_preamble(&text, file.as_deref());
        math_preview::render_math(&MathRenderRequest { snippet, preamble, color_hex: color.to_uppercase() })
    })
    .await
}

// ── Table editor ─────────────────────────────────────────────────────────────

/// The table in `source` (a `table` or `tabular` environment), or a starter table.
#[tauri::command]
#[specta::specta]
pub fn table_load(source: String) -> TableModel {
    table_editor::load(&source)
}

#[tauri::command]
#[specta::specta]
pub fn table_apply(mut model: TableModel, op: TableOp) -> TableModel {
    table_editor::apply(&mut model, op);
    model
}

#[tauri::command]
#[specta::specta]
pub fn table_latex(model: TableModel) -> String {
    generate_latex_table(&model)
}
