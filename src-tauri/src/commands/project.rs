//! Settings, the project list, the open project, and file operations.

use crate::events::IndexUpdated;
use crate::state::{blocking, CmdResult, Shared};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager, State};
use tauri_specta::Event;
use vortex_core::fs_utils::{build_tree, scan_projects, ProjectItem, TreeNode};
use vortex_core::semantic_index::IndexStats;
use vortex_core::settings::Settings;

#[tauri::command]
#[specta::specta]
pub fn get_settings() -> Settings {
    Settings::load()
}

#[tauri::command]
#[specta::specta]
pub fn set_settings(settings: Settings) -> CmdResult<()> {
    settings.save().map_err(|e| format!("Could not save settings: {e}"))
}

/// Subfolders of the projects folder, with PDF thumbnails (rendered on first use).
#[tauri::command]
#[specta::specta]
pub async fn list_projects() -> CmdResult<Vec<ProjectItem>> {
    blocking(|| {
        let folder = Settings::load().projects_folder.ok_or("No projects folder is set")?;
        Ok(scan_projects(&folder))
    })
    .await
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ProjectInfo {
    /// Canonical path of the project folder.
    pub path: String,
    pub name: String,
    pub tree: Vec<TreeNode>,
    pub stats: IndexStats,
}

/// Opens `path` as the project: indexes it, starts the file watcher and lets the
/// webview load its files (PDFs, images) through the asset protocol.
#[tauri::command]
#[specta::specta]
pub async fn open_project(app: AppHandle, state: State<'_, Shared>, path: String) -> CmdResult<ProjectInfo> {
    let state = state.inner().clone();
    let info = blocking(move || {
        let dir = PathBuf::from(&path).canonicalize().map_err(|e| format!("{path}: {e}"))?;
        if !dir.is_dir() {
            return Err(format!("{} is not a folder", dir.display()));
        }
        let dir_str = dir.to_string_lossy().to_string();
        state.backend.stop().map_err(|e| e.to_string())?;
        if let Ok(mut bases) = state.git_bases.lock() {
            bases.clear();
        }
        state.backend.init_project(&dir_str).map_err(|e| e.to_string())?;
        *state.project.write().map_err(|e| e.to_string())? = Some(dir.clone());

        let mut settings = Settings::load();
        settings.current_project = Some(dir_str.clone());
        let _ = settings.save();

        Ok(ProjectInfo {
            name: dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| dir_str.clone()),
            path: dir_str.clone(),
            tree: build_tree(&dir_str),
            stats: state.backend.get_stats().map_err(|e| e.to_string())?,
        })
    })
    .await?;
    app.asset_protocol_scope()
        .allow_directory(&info.path, true)
        .map_err(|e| format!("Could not allow access to the project folder: {e}"))?;
    let _ = IndexUpdated(info.stats.clone()).emit(&app);
    Ok(info)
}

#[tauri::command]
#[specta::specta]
pub async fn close_project(state: State<'_, Shared>) -> CmdResult<()> {
    let state = state.inner().clone();
    blocking(move || {
        state.backend.stop().map_err(|e| e.to_string())?;
        *state.project.write().map_err(|e| e.to_string())? = None;
        Ok(())
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn file_tree(state: State<'_, Shared>) -> CmdResult<Vec<TreeNode>> {
    let project = state.require_project()?;
    blocking(move || Ok(build_tree(&project.to_string_lossy()))).await
}

#[tauri::command]
#[specta::specta]
pub async fn read_file(path: String) -> CmdResult<String> {
    blocking(move || std::fs::read_to_string(&path).map_err(|e| format!("Could not read {path}: {e}"))).await
}

/// Saves `content` to `path`. Project `.tex`/`.bib` files are reindexed right away.
#[tauri::command]
#[specta::specta]
pub async fn write_file(app: AppHandle, state: State<'_, Shared>, path: String, content: String) -> CmdResult<()> {
    let state = state.inner().clone();
    let stats = blocking(move || {
        std::fs::write(&path, &content).map_err(|e| format!("Could not save {path}: {e}"))?;
        let in_project = state.project().is_some_and(|p| Path::new(&path).starts_with(p));
        let indexed = Path::new(&path)
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("tex") || e.eq_ignore_ascii_case("bib"));
        Ok((in_project && indexed).then(|| state.backend.reindex_file(&path).ok()).flatten())
    })
    .await?;
    if let Some(stats) = stats {
        let _ = IndexUpdated(stats).emit(&app);
    }
    Ok(())
}

/// Creates an empty file inside the project; fails if it already exists.
#[tauri::command]
#[specta::specta]
pub async fn create_file(state: State<'_, Shared>, path: String) -> CmdResult<()> {
    let target = state.in_project(&path)?;
    blocking(move || {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
            .map(|_| ())
            .map_err(|e| format!("Could not create {}: {e}", target.display()))
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn create_folder(state: State<'_, Shared>, path: String) -> CmdResult<()> {
    let target = state.in_project(&path)?;
    blocking(move || std::fs::create_dir(&target).map_err(|e| format!("Could not create {}: {e}", target.display())))
        .await
}

/// Renames or moves within the project; fails if the destination exists.
#[tauri::command]
#[specta::specta]
pub async fn rename_path(state: State<'_, Shared>, from: String, to: String) -> CmdResult<()> {
    let (from, to) = (state.in_project(&from)?, state.in_project(&to)?);
    blocking(move || {
        if to.exists() {
            return Err(format!("{} already exists", to.display()));
        }
        std::fs::rename(&from, &to).map_err(|e| format!("Could not rename {}: {e}", from.display()))
    })
    .await
}

/// Moves a file or folder inside the project to the Trash.
#[tauri::command]
#[specta::specta]
pub async fn delete_path(state: State<'_, Shared>, path: String) -> CmdResult<()> {
    let target = state.in_project(&path)?;
    blocking(move || trash::delete(&target).map_err(|e| format!("Could not move {} to the Trash: {e}", target.display())))
        .await
}

/// Opens an existing file in its default app (e.g. a PDF in Preview). URLs are refused.
#[tauri::command]
#[specta::specta]
pub async fn open_external(path: String) -> CmdResult<()> {
    let p = PathBuf::from(&path);
    if !p.is_absolute() || !p.exists() {
        return Err(format!("{path} is not an existing file"));
    }
    open::that_detached(&p).map_err(|e| format!("Could not open {path}: {e}"))
}

/// Lets the webview load one more file (a PDF or image opened from outside the project).
#[tauri::command]
#[specta::specta]
pub fn grant_file_access(app: AppHandle, path: String) -> CmdResult<()> {
    let p = PathBuf::from(&path);
    let ext = p.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).unwrap_or_default();
    if !p.is_file() || !["pdf", "png", "jpg", "jpeg", "svg"].contains(&ext.as_str()) {
        return Err(format!("{path} is not a PDF or image"));
    }
    app.asset_protocol_scope().allow_file(&p).map_err(|e| e.to_string())
}

/// Exits the app (after the UI has dealt with unsaved changes).
#[tauri::command]
#[specta::specta]
pub fn quit(app: AppHandle) {
    app.exit(0);
}
