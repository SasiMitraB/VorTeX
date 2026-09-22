/// Every `#[tauri::command]`. Tauri generates an `allow-<name>` permission for each;
/// `capabilities/default.json` grants them to the main window. Commands not listed
/// here cannot be called from the frontend.
const COMMANDS: &[&str] = &[
    // project
    "get_settings",
    "set_settings",
    "list_projects",
    "open_project",
    "close_project",
    "file_tree",
    "read_file",
    "write_file",
    "create_file",
    "create_folder",
    "rename_path",
    "delete_path",
    "open_external",
    "grant_file_access",
    "quit",
    // index
    "index_stats",
    "outline",
    "labels",
    "todos",
    "tables",
    "complete",
    // build
    "main_document",
    "build",
    "clean_build",
    // synctex
    "synctex_forward",
    "synctex_inverse",
    // git
    "git_status",
    "git_history",
    "git_diff",
    "git_line_markers",
    // tools
    "latexdiff_options",
    "latexdiff",
    "grammar_check",
    "math_at",
    "math_render",
    "table_load",
    "table_apply",
    "table_latex",
];

fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )
    .expect("failed to run tauri-build");
}
