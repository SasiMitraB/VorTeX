//! SyncTeX: source line ↔ PDF position. PDF coordinates are points from the page's top-left.

use crate::state::{blocking, CmdResult, Shared};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::Path;
use tauri::State;
use vortex_core::project;
use vortex_core::synctex::{synctex_forward_search, synctex_inverse_search, SynctexRect};

/// Where `line` (1-based) of `file` appears in the PDF. `pdf` defaults to the main
/// document's PDF. `null` when there is no SyncTeX data (build first).
#[tauri::command]
#[specta::specta]
pub async fn synctex_forward(
    state: State<'_, Shared>,
    file: String,
    line: u32,
    pdf: Option<String>,
) -> CmdResult<Option<SynctexRect>> {
    let state = state.inner().clone();
    blocking(move || {
        let pdf = match pdf {
            Some(pdf) => pdf,
            None => {
                let active = Path::new(&file);
                let dir = state.project_or_parent(Some(active)).ok_or("No project is open")?;
                let main = project::main_document(Some(active), &dir).ok_or("No main document found")?;
                project::pdf_for(&main).to_string_lossy().to_string()
            }
        };
        Ok(synctex_forward_search(&file, line as usize, 0, &pdf, 0).as_ref().map(SynctexRect::from))
    })
    .await
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct SourceLocation {
    /// Absolute, normalized path of the source file.
    pub file: String,
    /// 1-based.
    pub line: usize,
    /// 0-based; 0 when SyncTeX doesn't know the column.
    pub column: usize,
}

/// The source location under (`x`, `y`) on 1-based `page` of `pdf`.
#[tauri::command]
#[specta::specta]
pub async fn synctex_inverse(pdf: String, page: u32, x: f32, y: f32) -> CmdResult<Option<SourceLocation>> {
    blocking(move || {
        Ok(synctex_inverse_search(&pdf, page as usize, x, y).map(|r| SourceLocation {
            file: project::normalize_path(Path::new(&r.input)).to_string_lossy().to_string(),
            line: r.line,
            column: r.column,
        }))
    })
    .await
}
