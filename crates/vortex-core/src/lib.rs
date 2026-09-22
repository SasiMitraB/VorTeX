//! VorTeX core: everything the editor does that isn't drawing pixels.
//!
//! No UI toolkit is referenced here. The Tauri app (`src-tauri`) is a thin
//! command/event layer over these modules. API types derive `specta::Type`
//! when the `specta` feature is on, so they can be exported to TypeScript.

pub mod backend;
pub mod bibtex_parser;
pub mod build_log;
pub mod compiler;
pub mod completion;
pub mod file_watcher;
pub mod fs_utils;
pub mod fuzzy_matcher;
pub mod git;
pub mod grammar_checker;
pub mod grammar_preprocess;
pub mod latex_parser;
pub mod latexdiff;
pub mod math_preview;
pub mod pdf_renderer;
pub mod project;
pub mod semantic_index;
pub mod settings;
pub mod synctex;
pub mod table_editor;
pub mod table_parser;
pub mod text;
