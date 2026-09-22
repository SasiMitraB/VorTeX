//! State shared by all commands.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, RwLock};
use vortex_core::backend::BackendClient;

pub type Shared = Arc<AppState>;

pub type CmdResult<T> = Result<T, String>;

#[derive(Default)]
pub struct AppState {
    /// Index, watcher and matcher for the open project.
    pub backend: BackendClient,
    /// Canonical path of the open project folder.
    pub project: RwLock<Option<PathBuf>>,
    /// Set while a build runs; builds don't overlap.
    pub building: AtomicBool,
    /// Committed (HEAD) text per absolute file path for the git gutter; `None` = untracked.
    /// Cleared on `git-changed`.
    pub git_bases: Mutex<HashMap<PathBuf, Option<String>>>,
}

impl AppState {
    pub fn project(&self) -> Option<PathBuf> {
        self.project.read().ok().and_then(|p| p.clone())
    }

    pub fn require_project(&self) -> CmdResult<PathBuf> {
        self.project().ok_or_else(|| "No project is open".to_string())
    }

    /// The project folder, or the folder of `active` when no project is open.
    pub fn project_or_parent(&self, active: Option<&Path>) -> Option<PathBuf> {
        self.project().or_else(|| active.and_then(|a| a.parent()).map(Path::to_path_buf))
    }

    /// `path` made absolute and checked to be inside the open project (for create/rename/delete).
    pub fn in_project(&self, path: &str) -> CmdResult<PathBuf> {
        let project = self.require_project()?;
        let path = vortex_core::project::normalize_path(Path::new(path));
        // The target may not exist yet; its parent must.
        let parent = path.parent().ok_or("Invalid path")?;
        let parent = parent.canonicalize().map_err(|e| format!("{}: {e}", parent.display()))?;
        let full = match path.file_name() {
            Some(name) => parent.join(name),
            None => return Err("Invalid path".to_string()),
        };
        if full.starts_with(&project) && full != project {
            Ok(full)
        } else {
            Err(format!("{} is outside the open project", full.display()))
        }
    }
}

/// Runs blocking work (disk, git, TeX) off the IPC thread.
pub async fn blocking<T, F>(f: F) -> CmdResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> CmdResult<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())?
}
