//! Read-only views of the semantic index, and autocomplete.

use crate::state::{blocking, CmdResult, Shared};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::State;
use vortex_core::completion::{self, Completions};
use vortex_core::latex_parser::{label_kind, LabelKind, SectionItem, TableItem, TodoItem};
use vortex_core::semantic_index::IndexStats;

#[tauri::command]
#[specta::specta]
pub fn index_stats(state: State<'_, Shared>) -> CmdResult<IndexStats> {
    state.backend.get_stats().map_err(|e| e.to_string())
}

/// Sections of every indexed file, in file then line order.
#[tauri::command]
#[specta::specta]
pub fn outline(state: State<'_, Shared>) -> CmdResult<Vec<SectionItem>> {
    state.backend.get_all_sections().map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct LabelEntry {
    pub key: String,
    pub file: String,
    pub filename: String,
    pub line: usize,
    pub kind: LabelKind,
}

#[tauri::command]
#[specta::specta]
pub fn labels(state: State<'_, Shared>) -> CmdResult<Vec<LabelEntry>> {
    let mut labels: Vec<LabelEntry> = state
        .backend
        .get_all_labels()
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|l| LabelEntry { kind: label_kind(&l.key), key: l.key, file: l.file, filename: l.filename, line: l.line })
        .collect();
    labels.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    Ok(labels)
}

/// TODO/FIXME comments; `file` limits them to one file.
#[tauri::command]
#[specta::specta]
pub fn todos(state: State<'_, Shared>, file: Option<String>) -> CmdResult<Vec<TodoItem>> {
    state.backend.get_todos(file.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn tables(state: State<'_, Shared>, file: Option<String>) -> CmdResult<Vec<TableItem>> {
    state.backend.get_tables(file.as_deref()).map_err(|e| e.to_string())
}

/// Suggestions at UTF-16 column `cursor` of `line`; `null` when there is nothing to complete.
/// Ranking uses `file` to prefer labels defined nearby.
#[tauri::command]
#[specta::specta]
pub async fn complete(
    state: State<'_, Shared>,
    line: String,
    cursor: u32,
    file: Option<String>,
) -> CmdResult<Option<Completions>> {
    let backend = state.backend.clone();
    blocking(move || Ok(completion::complete(&backend, &line, cursor, file.as_deref()))).await
}
