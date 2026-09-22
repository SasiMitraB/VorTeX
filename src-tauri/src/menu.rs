//! The native menu bar. App items send a `menu-command` event to the UI, which
//! owns the behaviour; the OS items (Hide, Copy, Minimize, ...) are native.

use crate::events::MenuCommand;
use tauri::menu::{Menu, MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::{AppHandle, Runtime};
use tauri_specta::Event;

fn item<R: Runtime>(app: &AppHandle<R>, id: &str, label: &str, accel: Option<&str>) -> tauri::Result<tauri::menu::MenuItem<R>> {
    let b = MenuItemBuilder::with_id(id, label);
    match accel {
        Some(a) => b.accelerator(a).build(app),
        None => b.build(app),
    }
}

pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let i = |id: &str, label: &str, accel: Option<&str>| item(app, id, label, accel);

    let app_menu = SubmenuBuilder::new(app, "VorTeX")
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .item(&i("quit", "Quit VorTeX", Some("CmdOrCtrl+Q"))?)
        .build()?;
    let file = SubmenuBuilder::new(app, "File")
        .item(&i("new_tab", "New Tab", Some("CmdOrCtrl+N"))?)
        .item(&i("open_file", "Open File…", Some("CmdOrCtrl+O"))?)
        .item(&i("open_folder", "Open Folder…", Some("CmdOrCtrl+Shift+O"))?)
        .separator()
        .item(&i("save", "Save", Some("CmdOrCtrl+S"))?)
        .item(&i("close_tab", "Close Tab", Some("CmdOrCtrl+W"))?)
        .separator()
        .item(&i("show_projects", "Show Projects", Some("CmdOrCtrl+Shift+P"))?)
        .item(&i("change_projects_folder", "Change Projects Folder…", None)?)
        .build()?;
    // Undo/Redo go to the focused editor or input; clipboard items are native so
    // copy/paste work everywhere in the webview.
    let edit = SubmenuBuilder::new(app, "Edit")
        .item(&i("undo", "Undo", Some("CmdOrCtrl+Z"))?)
        .item(&i("redo", "Redo", Some("CmdOrCtrl+Shift+Z"))?)
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .separator()
        .item(&i("find", "Find…", Some("CmdOrCtrl+F"))?)
        .item(&i("toggle_comment", "Toggle Comment", Some("CmdOrCtrl+/"))?)
        .build()?;
    let insert = SubmenuBuilder::new(app, "Insert").item(&i("insert_table", "Table…", Some("CmdOrCtrl+Alt+T"))?).build()?;
    let view = SubmenuBuilder::new(app, "View")
        .item(&i("toggle_sidebar", "Toggle Sidebar", Some("CmdOrCtrl+\\"))?)
        .item(&i("cycle_theme", "Cycle Theme (Auto / Light / Dark)", Some("CmdOrCtrl+Shift+T"))?)
        .build()?;
    let build = SubmenuBuilder::new(app, "Build")
        .item(&i("build", "Build", Some("CmdOrCtrl+B"))?)
        .item(&i("clean_build", "Clean and Build", Some("CmdOrCtrl+Shift+B"))?)
        .item(&i("sync_pdf", "Sync PDF to Cursor", Some("CmdOrCtrl+J"))?)
        .separator()
        .item(&i("compare_versions", "Compare Versions (latexdiff)…", None)?)
        .build()?;
    let window = SubmenuBuilder::new(app, "Window").minimize().maximize().build()?;

    MenuBuilder::new(app).items(&[&app_menu, &file, &edit, &insert, &view, &build, &window]).build()
}

/// The command for a menu item id (`new_tab` → `MenuCommand::NewTab`).
pub fn command(id: &str) -> Option<MenuCommand> {
    let mut camel = String::new();
    let mut upper = false;
    for c in id.chars() {
        if c == '_' {
            upper = true;
        } else if upper {
            camel.push(c.to_ascii_uppercase());
            upper = false;
        } else {
            camel.push(c);
        }
    }
    serde_json::from_value(serde_json::Value::String(camel)).ok()
}

/// Forwards a clicked (or shortcut-triggered) menu item to the UI.
pub fn forward<R: Runtime>(app: &AppHandle<R>, id: &str) {
    if let Some(cmd) = command(id) {
        let _ = cmd.emit(app);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_menu_item_maps_to_a_command() {
        let src = include_str!("menu.rs");
        let ids: Vec<&str> = src.split("&i(\"").skip(1).map(|s| s.split('"').next().unwrap()).collect();
        assert!(ids.len() > 15);
        for id in ids {
            assert!(super::command(id).is_some(), "menu id {id} has no MenuCommand");
        }
    }
}
