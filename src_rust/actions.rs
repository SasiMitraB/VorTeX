//! Application actions, key bindings and the native (OS) menu bar.
//!
//! App-level actions are handled globally in `main.rs`, so their menu items are
//! always enabled. Editor actions are handled by `LatexEditor` and are only
//! enabled while an editor has focus (matching standard macOS behaviour).

use gpui::{actions, App, KeyBinding, Menu, MenuItem, OsAction, SystemMenuType};

/// Key context set on the editor element; editor bindings are scoped to it.
pub const EDITOR_CONTEXT: &str = "LatexEditor";

actions!(
    vortex,
    [
        Quit,
        Hide,
        HideOthers,
        ShowAll,
        NewTab,
        OpenFile,
        OpenFolder,
        Save,
        CloseTab,
        ShowProjects,
        ChangeProjectsFolder,
        InsertTable,
        ToggleSidebar,
        ToggleTheme,
        Build,
        SyncPdf,
        Minimize,
        Zoom,
    ]
);

actions!(
    editor,
    [Undo, Redo, Cut, Copy, Paste, SelectAll, ToggleComment]
);

/// `cmd-` on macOS, `ctrl-` elsewhere.
fn primary(keys: &str) -> String {
    if cfg!(target_os = "macos") {
        format!("cmd-{keys}")
    } else {
        format!("ctrl-{keys}")
    }
}

pub fn bind_keys(cx: &mut App) {
    let editor = Some(EDITOR_CONTEXT);
    cx.bind_keys([
        KeyBinding::new(&primary("q"), Quit, None),
        KeyBinding::new(&primary("h"), Hide, None),
        KeyBinding::new(&primary("alt-h"), HideOthers, None),
        KeyBinding::new(&primary("n"), NewTab, None),
        KeyBinding::new(&primary("o"), OpenFile, None),
        KeyBinding::new(&primary("shift-o"), OpenFolder, None),
        KeyBinding::new(&primary("s"), Save, None),
        KeyBinding::new(&primary("w"), CloseTab, None),
        KeyBinding::new(&primary("shift-p"), ShowProjects, None),
        KeyBinding::new(&primary("alt-t"), InsertTable, None),
        KeyBinding::new(&primary("\\"), ToggleSidebar, None),
        KeyBinding::new(&primary("shift-t"), ToggleTheme, None),
        KeyBinding::new(&primary("b"), Build, None),
        KeyBinding::new(&primary("j"), SyncPdf, None),
        KeyBinding::new(&primary("m"), Minimize, None),
        KeyBinding::new(&primary("z"), Undo, editor),
        KeyBinding::new(&primary("shift-z"), Redo, editor),
        KeyBinding::new(&primary("x"), Cut, editor),
        KeyBinding::new(&primary("c"), Copy, editor),
        KeyBinding::new(&primary("v"), Paste, editor),
        KeyBinding::new(&primary("a"), SelectAll, editor),
        KeyBinding::new(&primary("/"), ToggleComment, editor),
    ]);
}

pub fn app_menus() -> Vec<Menu> {
    vec![
        Menu {
            name: "VorTeX".into(),
            items: vec![
                MenuItem::os_submenu("Services", SystemMenuType::Services),
                MenuItem::separator(),
                MenuItem::action("Hide VorTeX", Hide),
                MenuItem::action("Hide Others", HideOthers),
                MenuItem::action("Show All", ShowAll),
                MenuItem::separator(),
                MenuItem::action("Quit VorTeX", Quit),
            ],
        },
        Menu {
            name: "File".into(),
            items: vec![
                MenuItem::action("New Tab", NewTab),
                MenuItem::action("Open File…", OpenFile),
                MenuItem::action("Open Folder…", OpenFolder),
                MenuItem::separator(),
                MenuItem::action("Save", Save),
                MenuItem::action("Close Tab", CloseTab),
                MenuItem::separator(),
                MenuItem::action("Show Projects", ShowProjects),
                MenuItem::action("Change Projects Folder…", ChangeProjectsFolder),
            ],
        },
        Menu {
            name: "Edit".into(),
            items: vec![
                MenuItem::os_action("Undo", Undo, OsAction::Undo),
                MenuItem::os_action("Redo", Redo, OsAction::Redo),
                MenuItem::separator(),
                MenuItem::os_action("Cut", Cut, OsAction::Cut),
                MenuItem::os_action("Copy", Copy, OsAction::Copy),
                MenuItem::os_action("Paste", Paste, OsAction::Paste),
                MenuItem::os_action("Select All", SelectAll, OsAction::SelectAll),
                MenuItem::separator(),
                MenuItem::action("Toggle Comment", ToggleComment),
            ],
        },
        Menu {
            name: "Insert".into(),
            items: vec![MenuItem::action("Table…", InsertTable)],
        },
        Menu {
            name: "View".into(),
            items: vec![
                MenuItem::action("Toggle Sidebar", ToggleSidebar),
                MenuItem::action("Cycle Theme (Auto / Light / Dark)", ToggleTheme),
            ],
        },
        Menu {
            name: "Build".into(),
            items: vec![
                MenuItem::action("Build", Build),
                MenuItem::action("Sync PDF to Cursor", SyncPdf),
            ],
        },
        Menu {
            name: "Window".into(),
            items: vec![
                MenuItem::action("Minimize", Minimize),
                MenuItem::action("Zoom", Zoom),
            ],
        },
    ]
}
