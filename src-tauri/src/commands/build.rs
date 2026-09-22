//! Building the main document.

use crate::events::{BuildFinished, BuildStarted};
use crate::state::{blocking, CmdResult, Shared};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, State};
use tauri_specta::Event;
use vortex_core::compiler::{self, BuildResult};
use vortex_core::project::{self, Engine};

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MainDocument {
    pub path: String,
    pub engine: Engine,
    /// Where the PDF goes (it may not exist yet).
    pub pdf_path: String,
}

fn resolve(state: &Shared, active: Option<&str>) -> Option<PathBuf> {
    let active = active.map(Path::new);
    let project = state.project_or_parent(active)?;
    project::main_document(active, &project)
}

/// The document a build of `active` compiles: honors `%!TEX root`, else the project's main file.
#[tauri::command]
#[specta::specta]
pub async fn main_document(state: State<'_, Shared>, active: Option<String>) -> CmdResult<Option<MainDocument>> {
    let state = state.inner().clone();
    blocking(move || {
        Ok(resolve(&state, active.as_deref()).map(|main| MainDocument {
            engine: project::detect_engine_for(&main),
            pdf_path: project::pdf_for(&main).to_string_lossy().to_string(),
            path: main.to_string_lossy().to_string(),
        }))
    })
    .await
}

async fn run_build(app: AppHandle, state: Shared, active: Option<String>, clean_first: bool) -> CmdResult<BuildResult> {
    let main = resolve(&state, active.as_deref()).ok_or("No .tex document to build")?;
    if state.building.swap(true, Ordering::SeqCst) {
        return Err("A build is already running".to_string());
    }
    let main_file = main.to_string_lossy().to_string();
    let _ = BuildStarted { main_file: main_file.clone() }.emit(&app);
    let result = blocking(move || {
        if clean_first {
            compiler::clean(&main_file)?;
        }
        Ok(compiler::build_latex_project(&main_file))
    })
    .await;
    state.building.store(false, Ordering::SeqCst);
    let result = result?;
    let _ = BuildFinished(result.clone()).emit(&app);
    Ok(result)
}

/// Compiles the main document for `active`. The UI saves dirty buffers first.
/// Emits `build-started` and `build-finished`.
#[tauri::command]
#[specta::specta]
pub async fn build(app: AppHandle, state: State<'_, Shared>, active: Option<String>) -> CmdResult<BuildResult> {
    run_build(app, state.inner().clone(), active, false).await
}

/// Removes auxiliary files, then builds from scratch.
#[tauri::command]
#[specta::specta]
pub async fn clean_build(app: AppHandle, state: State<'_, Shared>, active: Option<String>) -> CmdResult<BuildResult> {
    run_build(app, state.inner().clone(), active, true).await
}
