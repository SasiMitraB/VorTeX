//! `#[tauri::command]`s: a thin layer over `vortex-core`. Anything that touches
//! the disk, git or TeX runs on a blocking thread so the IPC thread stays free.

pub mod build;
pub mod git;
pub mod index;
pub mod project;
pub mod synctex;
pub mod tools;
