//! Events Rust sends to the UI. Names are the kebab-case type names
//! (`IndexUpdated` → `index-updated`); the generated bindings expose them as
//! `events.indexUpdated.listen(...)`.

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_specta::Event;
use vortex_core::compiler::BuildResult;
use vortex_core::file_watcher::FileChangeEvent;
use vortex_core::semantic_index::IndexStats;

/// The semantic index changed (project opened, or a `.tex`/`.bib` file changed).
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct IndexUpdated(pub IndexStats);

/// A watched file changed on disk: sources, PDFs (reload the viewer) or git metadata.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct FsChanged(pub FileChangeEvent);

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct BuildStarted {
    pub main_file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct BuildFinished(pub BuildResult);

/// Repository state may have changed (working tree edit, commit, checkout, stage).
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct GitChanged;

/// A menu item or its shortcut was used. `closeWindow` is sent when the user closes
/// the window, so the UI can ask about unsaved changes and then call `quit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub enum MenuCommand {
    Quit,
    CloseWindow,
    NewTab,
    OpenFile,
    OpenFolder,
    Save,
    CloseTab,
    ShowProjects,
    ChangeProjectsFolder,
    Undo,
    Redo,
    Find,
    ToggleComment,
    InsertTable,
    ToggleSidebar,
    CycleTheme,
    Build,
    CleanBuild,
    SyncPdf,
    CompareVersions,
}
