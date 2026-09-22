//! The VorTeX desktop app: Tauri window, typed commands and events over `vortex-core`.
//!
//! TypeScript bindings for every command and event are generated into
//! `ui/src/bindings.ts` (on debug startup, and by `cargo test -p vortex-app`).

mod commands;
mod events;
mod menu;
mod state;

use commands::{build, git, index, project, synctex, tools};
use events::{BuildFinished, BuildStarted, FsChanged, GitChanged, IndexUpdated, MenuCommand};
use state::{AppState, Shared};
use std::sync::Arc;
use tauri::Manager;
use tauri_specta::{collect_commands, collect_events, ErrorHandlingMode, Event};
use vortex_core::file_watcher::WatchedKind;

/// Where the generated TypeScript bindings go.
pub const BINDINGS_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../ui/src/bindings.ts");

pub fn specta_builder() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            project::get_settings,
            project::set_settings,
            project::list_projects,
            project::open_project,
            project::close_project,
            project::file_tree,
            project::read_file,
            project::write_file,
            project::create_file,
            project::create_folder,
            project::rename_path,
            project::delete_path,
            project::open_external,
            project::grant_file_access,
            project::quit,
            index::index_stats,
            index::outline,
            index::labels,
            index::todos,
            index::tables,
            index::complete,
            build::main_document,
            build::build,
            build::clean_build,
            synctex::synctex_forward,
            synctex::synctex_inverse,
            git::git_status,
            git::git_history,
            git::git_diff,
            git::git_line_markers,
            tools::latexdiff_options,
            tools::latexdiff,
            tools::grammar_check,
            tools::math_at,
            tools::math_render,
            tools::table_load,
            tools::table_apply,
            tools::table_latex,
        ])
        .events(collect_events![IndexUpdated, FsChanged, BuildStarted, BuildFinished, GitChanged, MenuCommand])
        .error_handling(ErrorHandlingMode::Throw)
        // Line numbers, offsets and counts: far below 2^53.
        .dangerously_cast_bigints_to_number()
}

pub fn export_bindings(builder: &tauri_specta::Builder<tauri::Wry>) {
    builder
        .export(
            specta_typescript::Typescript::default().header("// @ts-nocheck\n/* eslint-disable */"),
            BINDINGS_PATH,
        )
        .expect("failed to export TypeScript bindings");
}

pub fn run() {
    let builder = specta_builder();
    #[cfg(debug_assertions)]
    export_bindings(&builder);

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage::<Shared>(Arc::new(AppState::default()))
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);
            forward_watcher_events(app.handle().clone());
            app.set_menu(menu::build(app.handle())?)?;
            Ok(())
        })
        .on_menu_event(|app, event| menu::forward(app, event.id().as_ref()))
        .on_window_event(|window, event| {
            // The UI asks about unsaved changes, then calls `quit`.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = MenuCommand::CloseWindow.emit(window.app_handle());
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running VorTeX");
}

/// Turns file-watcher callbacks into `fs-changed`, `index-updated` and `git-changed` events.
fn forward_watcher_events(app: tauri::AppHandle) {
    let state = app.state::<Shared>().inner().clone();
    let handle = app.clone();
    let shared = state.clone();
    state.backend.set_on_file_change(move |event| {
        let kind = event.kind;
        if kind == WatchedKind::Source {
            if let Ok(stats) = shared.backend.get_stats() {
                let _ = IndexUpdated(stats).emit(&handle);
            }
        }
        if kind != WatchedKind::Pdf {
            // Edits change `git status`; HEAD/index changes also change the committed text.
            if kind == WatchedKind::Git {
                if let Ok(mut bases) = shared.git_bases.lock() {
                    bases.clear();
                }
            }
            let _ = GitChanged.emit(&handle);
        }
        let _ = FsChanged(event).emit(&handle);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regenerates `ui/src/bindings.ts`. CI fails if the committed file differs.
    #[test]
    fn export_bindings_to_ui() {
        export_bindings(&specta_builder());
        let bindings = std::fs::read_to_string(BINDINGS_PATH).unwrap();
        assert!(bindings.contains("openProject: (path: string)"));
        assert!(bindings.contains("buildFinished"));
    }

    /// Every command must be registered in `lib.rs`, listed in `build.rs` (which generates its
    /// permission) and granted in `capabilities/default.json`, or calling it fails at runtime.
    #[test]
    fn commands_are_registered_listed_and_granted() {
        let dir = env!("CARGO_MANIFEST_DIR");
        let quoted = |text: &str| -> Vec<String> {
            text.split('"').skip(1).step_by(2).map(str::to_string).collect()
        };
        let build_rs = std::fs::read_to_string(format!("{dir}/build.rs")).unwrap();
        let listed = quoted(&build_rs[build_rs.find("COMMANDS").unwrap()..build_rs.find("fn main").unwrap()]);

        let lib_rs = std::fs::read_to_string(format!("{dir}/src/lib.rs")).unwrap();
        let block = &lib_rs[lib_rs.find("collect_commands![").unwrap()..];
        let block = &block[..block.find("])").unwrap()];
        let registered: Vec<String> = block
            .split(',')
            .filter_map(|s| s.trim().rsplit("::").next().map(str::to_string))
            .filter(|s| !s.is_empty() && !s.starts_with("collect_commands"))
            .collect();

        let caps: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(format!("{dir}/capabilities/default.json")).unwrap()).unwrap();
        let granted: Vec<String> = caps["permissions"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|p| p.as_str()?.strip_prefix("allow-").map(|c| c.replace('-', "_")))
            .collect();

        let mut listed_sorted = listed.clone();
        listed_sorted.sort();
        let mut registered_sorted = registered.clone();
        registered_sorted.sort();
        let mut granted_sorted = granted.clone();
        granted_sorted.sort();
        assert_eq!(registered_sorted, listed_sorted, "lib.rs collect_commands! vs build.rs COMMANDS");
        assert_eq!(granted_sorted, listed_sorted, "capabilities/default.json vs build.rs COMMANDS");
    }
}
