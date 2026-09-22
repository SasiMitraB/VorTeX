//! Source control for the open project (read-only: no commit, push or pull).

use crate::state::{blocking, CmdResult, Shared};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::{Path, PathBuf};
use tauri::State;
use vortex_core::git::{self, Commit, FileDiff, FileStatus, GutterMarkers};

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct GitStatus {
    pub root: String,
    pub branch: Option<String>,
    /// Paths are relative to `root`.
    pub changes: Vec<FileStatus>,
}

/// `null` when the project is not in a git repository.
#[tauri::command]
#[specta::specta]
pub async fn git_status(state: State<'_, Shared>) -> CmdResult<Option<GitStatus>> {
    let project = state.require_project()?;
    blocking(move || {
        Ok(git::repo_root(&project).map(|root| GitStatus {
            branch: git::current_branch(&root),
            changes: git::status(&root),
            root: root.to_string_lossy().to_string(),
        }))
    })
    .await
}

/// Commits touching `file`, or the whole project when `file` is `null`; newest first.
#[tauri::command]
#[specta::specta]
pub async fn git_history(state: State<'_, Shared>, file: Option<String>, limit: u32) -> CmdResult<Vec<Commit>> {
    let project = state.require_project()?;
    blocking(move || {
        let Some(root) = git::repo_root(&project) else {
            return Ok(Vec::new());
        };
        let limit = limit.clamp(1, 1000) as usize;
        Ok(match file {
            Some(file) => git::relative_path(&root, Path::new(&file)).map(|rel| git::file_log(&root, &rel, limit)).unwrap_or_default(),
            None => git::log(&root, &git::relative_path(&root, &project).unwrap_or_default(), limit),
        })
    })
    .await
}

/// Diff of `path` at `rev` (default `HEAD`) against `buffer` (unsaved editor text), or the file on disk.
#[tauri::command]
#[specta::specta]
pub async fn git_diff(path: String, rev: Option<String>, buffer: Option<String>) -> CmdResult<FileDiff> {
    blocking(move || {
        let abs = PathBuf::from(&path);
        let root = git::repo_root(&abs).ok_or("Not in a git repository")?;
        let rel = git::relative_path(&root, &abs).ok_or("File is outside the repository")?;
        let rev = rev.unwrap_or_else(|| "HEAD".to_string());
        let base = git::file_at_revision(&root, &rev, &rel).unwrap_or_default();
        let current = match buffer {
            Some(text) => text.into_bytes(),
            None => std::fs::read(&abs).unwrap_or_default(),
        };
        match (git::as_text(&base), git::as_text(&current)) {
            (Some(old), Some(new)) => Ok(git::unified_diff(&old, &new, 3)),
            _ => Err("Binary file — no text diff to show".to_string()),
        }
    })
    .await
}

/// Added/modified/removed markers for the editor gutter: `buffer` against the committed `path`.
/// Untracked files and files outside a repository get no markers.
#[tauri::command]
#[specta::specta]
pub async fn git_line_markers(state: State<'_, Shared>, path: String, buffer: String) -> CmdResult<GutterMarkers> {
    let state = state.inner().clone();
    blocking(move || {
        let abs = PathBuf::from(&path);
        let cached = state.git_bases.lock().ok().and_then(|b| b.get(&abs).cloned());
        let base = match cached {
            Some(base) => base,
            None => {
                let base = (|| {
                    let root = git::repo_root(&abs)?;
                    let rel = git::relative_path(&root, &abs)?;
                    git::as_text(&git::file_at_revision(&root, "HEAD", &rel)?)
                })();
                if let Ok(mut bases) = state.git_bases.lock() {
                    bases.insert(abs, base.clone());
                }
                base
            }
        };
        Ok(base.map(|base| git::gutter_markers(&base, &buffer)).unwrap_or_default())
    })
    .await
}
