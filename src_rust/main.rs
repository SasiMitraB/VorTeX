mod backend;
pub mod services;
mod state;
mod theme;
mod views;

use backend::{BackendClient, FileChangeEvent, IndexStats};
use gpui::prelude::*;
use gpui::*;
use state::{AppState, PaneSide, TabType, ViewMode};
use std::sync::mpsc;
use std::sync::Arc;
use theme::Theme;
use views::editor::LatexEditor;
use views::pdf_preview::render_pdf_preview;
use views::project_selector::render_project_selector;
use views::sidebar::render_sidebar;
use views::status_bar::render_status_bar;
use views::table_editor::{render_table_editor_modal, TableEditorModal};
use views::tabs::render_tab_bar;
use views::workspace::render_toolbar;

pub struct VorTexApp {
    state: AppState,
    editor_left: Entity<LatexEditor>,
    editor_right: Entity<LatexEditor>,
    table_modal: TableEditorModal,
    event_rx: Arc<std::sync::Mutex<mpsc::Receiver<BackendEvent>>>,
    event_tx: mpsc::Sender<BackendEvent>,
}

enum BackendEvent {
    FileChange(FileChangeEvent),
    IndexReady(IndexStats),
    BuildFinished(crate::services::compiler::BuildResult),
}

impl VorTexApp {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let backend = BackendClient::new().expect("Failed to start VorTeX native backend");

        let (tx, rx) = mpsc::channel();
        let tx_change = tx.clone();
        let tx_ready = tx.clone();

        backend.set_on_file_change(move |ev| {
            let _ = tx_change.send(BackendEvent::FileChange(ev));
        });

        backend.set_on_index_ready(move |stats| {
            let _ = tx_ready.send(BackendEvent::IndexReady(stats));
        });

        let editor_left = cx.new(|cx| {
            LatexEditor::new(
                "\\documentclass{article}\n\\usepackage{amsmath}\n\n\\title{Welcome to VorTeX}\n\\author{Author Name}\n\\date{\\today}\n\n\\begin{document}\n\\maketitle\n\n\\section{Introduction}\nWelcome to \\textbf{VorTeX}, a modern GPU-accelerated LaTeX editor built with GPUI!\n\n\\subsection{Features}\n\\begin{itemize}\n  \\item Blazing-fast GPU rendering at 120 FPS\n  \\item Smart LaTeX auto-completion and delimiters\n  \\item 100% Native in-process Rust semantic indexing\n\\end{itemize}\n\n\\end{document}\n",
                None,
                backend.clone(),
                cx,
            )
        });

        let editor_right = cx.new(|cx| {
            LatexEditor::new("", None, backend.clone(), cx)
        });

        let mut app_state = AppState::new(backend);

        // Check if there is a saved projects folder or current project
        if let Ok(Some(folder_val)) = app_state.backend.config_get("projectsFolder") {
            if let Some(folder_str) = folder_val.as_str() {
                app_state.projects_folder = Some(folder_str.to_string());
                if let Ok(projects) = app_state.backend.scan_projects(folder_str) {
                    app_state.projects = projects;
                }
            }
        }

        // Add initial tab to left pane
        let tab_id = app_state.generate_tab_id();
        app_state.pane_left.tabs.push(state::Tab::new_text(
            tab_id.clone(),
            "Welcome.tex".to_string(),
            None,
            String::new(),
        ));
        app_state.pane_left.active_tab_id = Some(tab_id);

        Self {
            state: app_state,
            editor_left,
            editor_right,
            table_modal: TableEditorModal::default(),
            event_rx: Arc::new(std::sync::Mutex::new(rx)),
            event_tx: tx,
        }
    }

    pub fn open_project(&mut self, project_path: String, cx: &mut Context<Self>) {
        self.state.current_project = Some(project_path.clone());
        self.state.current_view = ViewMode::Workspace;

        // Save current project in config
        let _ = self.state.backend.config_set("currentProject", serde_json::json!(project_path));

        // Initialize project indexing & watcher in backend
        let _ = self.state.backend.init_project(&project_path);

        // Load project file tree
        if let Ok(tree) = self.state.backend.read_tree(&project_path) {
            self.state.file_tree = tree;
        }

        // Auto open main.tex if available
        let main_tex = std::path::Path::new(&project_path).join("main.tex");
        if main_tex.exists() {
            if let Ok(content) = self.state.backend.read_file(main_tex.to_str().unwrap()) {
                self.open_file_in_active_pane(main_tex.to_str().unwrap().to_string(), content, cx);
            }
        }

        cx.notify();
    }

    pub fn open_file_in_active_pane(&mut self, path: String, content: String, cx: &mut Context<Self>) {
        let is_pdf = path.to_lowercase().ends_with(".pdf");
        self.state.open_or_switch_file(path.clone(), content.clone());

        if !is_pdf {
            let active_editor = match self.state.active_pane {
                PaneSide::Left => &self.editor_left,
                PaneSide::Right => &self.editor_right,
            };

            active_editor.update(cx, |editor, _cx| {
                editor.set_content(&content, Some(path.clone()));
            });

            // Update document outline
            if path.ends_with(".tex") {
                if let Ok(sections) = self.state.backend.get_sections(&path) {
                    self.state.outline_sections = sections;
                }
            } else {
                self.state.outline_sections.clear();
            }
        }

        cx.notify();
    }

    pub fn save_active_file(&mut self, cx: &mut Context<Self>) {
        let active_pane = self.state.active_pane;
        let tab_opt = self.state.active_tab().cloned();

        if let Some(tab) = tab_opt {
            if tab.tab_type == TabType::Pdf {
                return;
            }

            let editor = match active_pane {
                PaneSide::Left => &self.editor_left,
                PaneSide::Right => &self.editor_right,
            };

            let content = editor.read(cx).get_content();

            if let Some(ref path) = tab.path {
                if let Ok(true) = self.state.backend.write_file(path, &content) {
                    if let Some(t) = self.state.active_tab_mut() {
                        t.dirty = false;
                        t.content = content.clone();
                    }
                    self.state.status_message = Some(format!("Saved {}", tab.name));

                    // Reindex file
                    if let Ok(stats) = self.state.backend.reindex_file(path) {
                        self.state.index_stats = stats;
                    }
                }
            } else {
                // Save As via native dialog
                if let Some(file_path) = rfd::FileDialog::new()
                    .set_file_name(&tab.name)
                    .add_filter("LaTeX Document", &["tex"])
                    .save_file()
                {
                    let path_str = file_path.to_str().unwrap().to_string();
                    if let Ok(true) = self.state.backend.write_file(&path_str, &content) {
                        let name = file_path.file_name().unwrap().to_str().unwrap().to_string();
                        if let Some(t) = self.state.active_tab_mut() {
                            t.path = Some(path_str.clone());
                            t.name = name;
                            t.dirty = false;
                            t.content = content;
                        }
                        self.state.status_message = Some(format!("Saved {}", path_str));
                    }
                }
            }
        }
        cx.notify();
    }

    pub fn resolve_build_target(&self) -> Option<String> {
        // 1. Check if active tab is a .tex file
        if let Some(tab) = self.state.active_tab() {
            if let Some(ref path) = tab.path {
                if path.to_lowercase().ends_with(".tex") {
                    return Some(path.clone());
                }
            }
        }

        // 2. Check if current project has main.tex
        if let Some(ref proj_path) = self.state.current_project {
            let main_tex = std::path::Path::new(proj_path).join("main.tex");
            if main_tex.exists() && main_tex.is_file() {
                return Some(main_tex.to_string_lossy().to_string());
            }

            // 3. Search for any .tex file in the project root directory
            if let Ok(entries) = std::fs::read_dir(proj_path) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_file() && p.extension().map(|e| e.eq_ignore_ascii_case("tex")).unwrap_or(false) {
                        return Some(p.to_string_lossy().to_string());
                    }
                }
            }
        }

        None
    }

    pub fn build_current_project(&mut self, cx: &mut Context<Self>) {
        if self.state.is_building {
            return;
        }

        // Auto-save active file first so latest changes are written to disk
        self.save_active_file(cx);

        let target = match self.resolve_build_target() {
            Some(t) => t,
            None => {
                self.state.status_message = Some("No .tex file found to build".to_string());
                cx.notify();
                return;
            }
        };

        let file_name = std::path::Path::new(&target)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("document.tex")
            .to_string();

        self.state.is_building = true;
        self.state.status_message = Some(format!("Building {}...", file_name));
        cx.notify();

        let tx = self.event_tx.clone();
        let target_clone = target.clone();

        cx.background_executor()
            .spawn(async move {
                let result = crate::services::compiler::build_latex_project(&target_clone);
                let _ = tx.send(BackendEvent::BuildFinished(result));
            })
            .detach();
    }

    pub fn process_background_events(&mut self) {
        if let Ok(rx_lock) = self.event_rx.lock() {
            while let Ok(ev) = rx_lock.try_recv() {
                match ev {
                    BackendEvent::FileChange(change) => {
                        self.state.status_message = Some(format!("File {}: {}", change.change_type, change.path));
                    }
                    BackendEvent::IndexReady(stats) => {
                        self.state.index_stats = stats;
                        self.state.status_message = Some("Semantic index ready".to_string());
                    }
                    BackendEvent::BuildFinished(result) => {
                        self.state.is_building = false;
                        self.state.status_message = Some(result.message.clone());

                        if result.success {
                            // Refresh project file tree to show newly generated files
                            if let Some(ref proj) = self.state.current_project {
                                if let Ok(tree) = self.state.backend.read_tree(proj) {
                                    self.state.file_tree = tree;
                                }
                            }

                            if let Some(ref pdf_path) = result.pdf_path {
                                // If PDF preview is open in either pane, or if right pane is empty, open preview
                                let is_open_left = self.state.pane_left.tabs.iter().any(|t| t.path.as_deref() == Some(pdf_path));
                                let is_open_right = self.state.pane_right.tabs.iter().any(|t| t.path.as_deref() == Some(pdf_path));

                                if is_open_left || is_open_right {
                                    // Already open in tabs
                                } else if self.state.pane_right.tabs.is_empty() {
                                    // Open in right pane for side-by-side view
                                    let id = self.state.generate_tab_id();
                                    let name = std::path::Path::new(pdf_path)
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("Output.pdf")
                                        .to_string();
                                    self.state.pane_right.tabs.push(state::Tab::new_pdf(id.clone(), name, pdf_path.clone()));
                                    self.state.pane_right.active_tab_id = Some(id);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

impl Render for VorTexApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.process_background_events();

        let current_view = self.state.current_view;
        let view_handle = cx.entity().clone();

        match current_view {
            ViewMode::ProjectSelector => {
                let projects = self.state.projects.clone();
                let projects_folder = self.state.projects_folder.clone();
                let proj_scroll = self.state.project_scroll_handle.clone();

                let view1 = view_handle.clone();
                let view2 = view_handle.clone();

                render_project_selector(
                    &projects,
                    projects_folder.as_deref(),
                    &proj_scroll,
                    move |proj_path, _window, cx| {
                        view1.update(cx, |this, cx| {
                            this.open_project(proj_path, cx);
                        });
                    },
                    move |_window, cx| {
                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                            let folder_str = folder.to_str().unwrap().to_string();
                            view2.update(cx, |this, cx| {
                                this.state.projects_folder = Some(folder_str.clone());
                                let _ = this.state.backend.config_set("projectsFolder", serde_json::json!(folder_str));
                                if let Ok(projs) = this.state.backend.scan_projects(&folder_str) {
                                    this.state.projects = projs;
                                }
                                cx.notify();
                            });
                        }
                    },
                )
                .into_any_element()
            }

            ViewMode::Workspace => {
                let project_name = self.state.current_project.as_ref().map(|p| {
                    std::path::Path::new(p).file_name().and_then(|n| n.to_str()).unwrap_or(p.as_str()).to_string()
                });

                let sidebar_vis = self.state.sidebar_visible;
                let file_tree = self.state.file_tree.clone();
                let expanded_folders = self.state.expanded_folders.clone();
                let outline_sections = self.state.outline_sections.clone();
                let active_tab_path = self.state.active_tab().and_then(|t| t.path.clone());
                let act_sidebar = active_tab_path.clone();
                let act_statusbar = active_tab_path.clone();

                let tree_scroll = self.state.sidebar_tree_scroll_handle.clone();
                let outline_scroll = self.state.sidebar_outline_scroll_handle.clone();

                let left_tabs = self.state.pane_left.tabs.clone();
                let left_active_id = self.state.pane_left.active_tab_id.clone();
                let right_tabs = self.state.pane_right.tabs.clone();
                let right_active_id = self.state.pane_right.active_tab_id.clone();
                let has_right_pane = !right_tabs.is_empty();

                let left_active_tab = self.state.pane_left.active_tab().cloned();
                let right_active_tab = self.state.pane_right.active_tab().cloned();

                let table_open = self.table_modal.is_open;
                let table_modal = self.table_modal.default_clone();
                let table_scroll = self.state.table_scroll_handle.clone();

                let stats = self.state.index_stats.clone();
                let cursor_r = self.editor_left.read(cx).cursor_row;
                let cursor_c = self.editor_left.read(cx).cursor_col;
                let total_lines = self.editor_left.read(cx).lines.len();
                let status_msg = self.state.status_message.clone();

                self.editor_left.update(cx, |ed, _cx| {
                    ed.sidebar_visible = sidebar_vis;
                    ed.is_right_pane = false;
                    ed.has_right_pane = has_right_pane;
                });
                self.editor_right.update(cx, |ed, _cx| {
                    ed.sidebar_visible = sidebar_vis;
                    ed.is_right_pane = true;
                    ed.has_right_pane = has_right_pane;
                });

                let ed_left = self.editor_left.clone();
                let ed_right = self.editor_right.clone();

                let is_building = self.state.is_building;
                let v_back = view_handle.clone();
                let v_toggle = view_handle.clone();
                let v_open_f = view_handle.clone();
                let v_open_d = view_handle.clone();
                let v_new_f = view_handle.clone();
                let v_save = view_handle.clone();
                let v_build = view_handle.clone();
                let v_table = view_handle.clone();

                let v_toggle_folder = view_handle.clone();
                let v_open_tree_file = view_handle.clone();
                let v_jump_outline = view_handle.clone();

                let v_sw_left = view_handle.clone();
                let v_cl_left = view_handle.clone();
                let v_new_left = view_handle.clone();
                let v_split_left = view_handle.clone();

                let v_sw_right = view_handle.clone();
                let v_cl_right = view_handle.clone();
                let v_new_right = view_handle.clone();
                let v_unsplit_right = view_handle.clone();

                let v_tbl_ins = view_handle.clone();
                let v_tbl_cls = view_handle.clone();
                let v_tbl_cell = view_handle.clone();
                let v_tbl_ar = view_handle.clone();
                let v_tbl_rr = view_handle.clone();
                let v_tbl_ac = view_handle.clone();
                let v_tbl_rc = view_handle.clone();

                div()
                    .size_full()
                    .bg(Theme::bg_app())
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                        let key = event.keystroke.key.as_str();
                        let is_cmd = event.keystroke.modifiers.platform || event.keystroke.modifiers.control;

                        if is_cmd {
                            match key {
                                "s" => {
                                    this.save_active_file(cx);
                                }
                                "b" => {
                                    this.build_current_project(cx);
                                }
                                "n" => {
                                    this.state.create_new_tab();
                                    cx.notify();
                                }
                                "w" => {
                                    if let Some(id) = this.state.active_pane_state().active_tab_id.clone() {
                                        this.state.active_pane_state_mut().close_tab(&id);
                                        cx.notify();
                                    }
                                }
                                _ => {}
                            }
                        }
                    }))
                    .child(
                        // Top Toolbar
                        render_toolbar(
                            project_name,
                            sidebar_vis,
                            is_building,
                            move |_window, cx| {
                                v_back.update(cx, |this, cx| {
                                    this.state.current_view = ViewMode::ProjectSelector;
                                    if let Some(ref folder) = this.state.projects_folder {
                                        if let Ok(projs) = this.state.backend.scan_projects(folder) {
                                            this.state.projects = projs;
                                        }
                                    }
                                    cx.notify();
                                });
                            },
                            move |_window, cx| {
                                v_toggle.update(cx, |this, cx| {
                                    this.state.sidebar_visible = !this.state.sidebar_visible;
                                    cx.notify();
                                });
                            },
                            move |_window, cx| {
                                if let Some(file) = rfd::FileDialog::new()
                                    .add_filter("LaTeX Documents", &["tex", "bib", "pdf", "md", "txt"])
                                    .pick_file()
                                {
                                    let path_str = file.to_str().unwrap().to_string();
                                    v_open_f.update(cx, |this, cx| {
                                        let content = this.state.backend.read_file(&path_str).unwrap_or_default();
                                        this.open_file_in_active_pane(path_str, content, cx);
                                    });
                                }
                            },
                            move |_window, cx| {
                                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                    let folder_str = folder.to_str().unwrap().to_string();
                                    v_open_d.update(cx, |this, cx| {
                                        this.open_project(folder_str, cx);
                                    });
                                }
                            },
                            move |_window, cx| {
                                v_new_f.update(cx, |this, cx| {
                                    this.state.create_new_tab();
                                    cx.notify();
                                });
                            },
                            move |_window, cx| {
                                v_save.update(cx, |this, cx| {
                                    this.save_active_file(cx);
                                });
                            },
                            move |_window, cx| {
                                v_build.update(cx, |this, cx| {
                                    this.build_current_project(cx);
                                });
                            },
                            move |_window, cx| {
                                v_table.update(cx, |this, cx| {
                                    this.table_modal.is_open = true;
                                    cx.notify();
                                });
                            },
                        ),
                    )
                    .child(
                        // Main workspace content (Sidebar + Split Panes)
                        div()
                            .flex_1()
                            .flex()
                            .overflow_hidden()
                            .when(sidebar_vis, move |d| {
                                d.child(render_sidebar(
                                    &file_tree,
                                    &expanded_folders,
                                    &outline_sections,
                                    act_sidebar.as_deref(),
                                    &tree_scroll,
                                    &outline_scroll,
                                    move |path, _window, cx| {
                                        v_toggle_folder.update(cx, |this, cx| {
                                            if this.state.expanded_folders.contains(&path) {
                                                this.state.expanded_folders.remove(&path);
                                            } else {
                                                this.state.expanded_folders.insert(path);
                                            }
                                            cx.notify();
                                        });
                                    },
                                    move |path, _window, cx| {
                                        v_open_tree_file.update(cx, |this, cx| {
                                            let content = this.state.backend.read_file(&path).unwrap_or_default();
                                            this.open_file_in_active_pane(path, content, cx);
                                        });
                                    },
                                    move |line_num, _window, cx| {
                                        v_jump_outline.update(cx, |this, cx| {
                                            this.editor_left.update(cx, |editor, _cx| {
                                                editor.jump_to_line(line_num);
                                            });
                                            cx.notify();
                                        });
                                    },
                                ))
                            })
                            .child(
                                // Editor Split Panes Area
                                div()
                                    .flex_1()
                                    .flex()
                                    .overflow_hidden()
                                    // Left Pane
                                    .child(
                                        div()
                                            .flex_1()
                                            .flex()
                                            .flex_col()
                                            .overflow_hidden()
                                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _event: &MouseDownEvent, _window, cx| {
                                                this.state.active_pane = PaneSide::Left;
                                                cx.notify();
                                            }))
                                            .child(render_tab_bar(
                                                &left_tabs,
                                                left_active_id.as_deref(),
                                                PaneSide::Left,
                                                move |id, _pane, _window, cx| {
                                                    v_sw_left.update(cx, |this, cx| {
                                                        this.state.pane_left.switch_tab(&id);
                                                        if let Some(tab) = this.state.pane_left.active_tab() {
                                                            if tab.tab_type == TabType::Text {
                                                                let c = tab.content.clone();
                                                                let p = tab.path.clone();
                                                                this.editor_left.update(cx, |ed, _cx| ed.set_content(&c, p));
                                                            }
                                                        }
                                                        cx.notify();
                                                    });
                                                },
                                                move |id, _pane, _window, cx| {
                                                    v_cl_left.update(cx, |this, cx| {
                                                        this.state.pane_left.close_tab(&id);
                                                        cx.notify();
                                                    });
                                                },
                                                move |_pane, _window, cx| {
                                                    v_new_left.update(cx, |this, cx| {
                                                        this.state.create_new_tab();
                                                        cx.notify();
                                                    });
                                                },
                                                move |_window, cx| {
                                                    v_split_left.update(cx, |this, cx| {
                                                        // Duplicate active tab into right pane for split view
                                                        if let Some(tab) = this.state.pane_left.active_tab().cloned() {
                                                            this.state.pane_right.tabs.push(tab.clone());
                                                            this.state.pane_right.active_tab_id = Some(tab.id);
                                                            this.state.active_pane = PaneSide::Right;
                                                            cx.notify();
                                                        }
                                                    });
                                                },
                                            ))
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .flex()
                                                    .child(if let Some(tab) = left_active_tab {
                                                        match tab.tab_type {
                                                            TabType::Text => ed_left.into_any_element(),
                                                            TabType::Pdf => render_pdf_preview(
                                                                tab.path.as_deref().unwrap_or(""),
                                                                |path, _window, _cx| {
                                                                    let _ = open::that(path);
                                                                },
                                                            ).into_any_element(),
                                                        }
                                                    } else {
                                                        empty_pane_placeholder()
                                                    }),
                                            ),
                                    )
                                    // Split Divider & Right Pane (if active)
                                    .when(has_right_pane, move |d| {
                                        let right_ed = ed_right.clone();
                                        d.child(
                                            div().w(px(2.0)).h_full().bg(Theme::border_subtle())
                                        )
                                        .child(
                                            div()
                                                .flex_1()
                                                .flex()
                                                .flex_col()
                                                .overflow_hidden()
                                                .on_mouse_down(MouseButton::Left, cx.listener(|this, _event: &MouseDownEvent, _window, cx| {
                                                    this.state.active_pane = PaneSide::Right;
                                                    cx.notify();
                                                }))
                                                .child(render_tab_bar(
                                                    &right_tabs,
                                                    right_active_id.as_deref(),
                                                    PaneSide::Right,
                                                    move |id, _pane, _window, cx| {
                                                        v_sw_right.update(cx, |this, cx| {
                                                            this.state.pane_right.switch_tab(&id);
                                                            cx.notify();
                                                        });
                                                    },
                                                    move |id, _pane, _window, cx| {
                                                        v_cl_right.update(cx, |this, cx| {
                                                            this.state.pane_right.close_tab(&id);
                                                            cx.notify();
                                                        });
                                                    },
                                                    move |_pane, _window, cx| {
                                                        v_new_right.update(cx, |this, cx| {
                                                            let id = this.state.generate_tab_id();
                                                            this.state.pane_right.tabs.push(state::Tab::new_text(
                                                                id.clone(),
                                                                "Split-Doc.tex".to_string(),
                                                                None,
                                                                String::new(),
                                                            ));
                                                            this.state.pane_right.active_tab_id = Some(id);
                                                            cx.notify();
                                                        });
                                                    },
                                                    move |_window, cx| {
                                                        v_unsplit_right.update(cx, |this, cx| {
                                                            this.state.pane_right.tabs.clear();
                                                            this.state.pane_right.active_tab_id = None;
                                                            this.state.active_pane = PaneSide::Left;
                                                            cx.notify();
                                                        });
                                                    },
                                                ))
                                                .child(
                                                    div()
                                                        .flex_1()
                                                        .flex()
                                                        .child(if let Some(tab) = right_active_tab {
                                                            match tab.tab_type {
                                                                TabType::Text => right_ed.into_any_element(),
                                                                TabType::Pdf => render_pdf_preview(
                                                                    tab.path.as_deref().unwrap_or(""),
                                                                    |path, _window, _cx| {
                                                                        let _ = open::that(path);
                                                                    },
                                                                ).into_any_element(),
                                                            }
                                                        } else {
                                                            empty_pane_placeholder()
                                                        }),
                                                ),
                                        )
                                    }),
                            ),
                    )
                    .child(
                        // Bottom Status Bar
                        render_status_bar(
                            act_statusbar.as_deref(),
                            cursor_r,
                            cursor_c,
                            total_lines,
                            &stats,
                            status_msg.as_deref(),
                        ),
                    )
                    .when(table_open, move |d| {
                        d.child(render_table_editor_modal(
                            &table_modal,
                            &table_scroll,
                            move |latex_code, _window, cx| {
                                v_tbl_ins.update(cx, |this, cx| {
                                    this.editor_left.update(cx, |ed, _cx| {
                                        ed.insert_text(&latex_code);
                                    });
                                    this.table_modal.is_open = false;
                                    cx.notify();
                                });
                            },
                            move |_window, cx| {
                                v_tbl_cls.update(cx, |this, cx| {
                                    this.table_modal.is_open = false;
                                    cx.notify();
                                });
                            },
                            move |r, c, val, _window, cx| {
                                v_tbl_cell.update(cx, |this, cx| {
                                    this.table_modal.set_cell(r, c, val);
                                    cx.notify();
                                });
                            },
                            move |_window, cx| {
                                v_tbl_ar.update(cx, |this, cx| {
                                    this.table_modal.add_row();
                                    cx.notify();
                                });
                            },
                            move |_window, cx| {
                                v_tbl_rr.update(cx, |this, cx| {
                                    this.table_modal.remove_row();
                                    cx.notify();
                                });
                            },
                            move |_window, cx| {
                                v_tbl_ac.update(cx, |this, cx| {
                                    this.table_modal.add_col();
                                    cx.notify();
                                });
                            },
                            move |_window, cx| {
                                v_tbl_rc.update(cx, |this, cx| {
                                    this.table_modal.remove_col();
                                    cx.notify();
                                });
                            },
                        ))
                    })
                    .into_any_element()
            }
        }
    }
}

impl TableEditorModal {
    fn default_clone(&self) -> Self {
        Self {
            rows: self.rows,
            cols: self.cols,
            cells: self.cells.clone(),
            is_open: self.is_open,
            on_insert: None,
        }
    }
}

fn empty_pane_placeholder() -> AnyElement {
    div()
        .size_full()
        .bg(Theme::bg_editor())
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_2()
        .child(
            div()
                .text_3xl()
                .text_color(Theme::text_dim())
                .child("λ"),
        )
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .text_color(Theme::text_dim())
                .child("No tab open"),
        )
        .child(
            div()
                .text_xs()
                .text_color(Theme::text_dim())
                .child("Use ⌘N to create a tab or select a file from Explorer"),
        )
        .into_any_element()
}

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.activate(true);
        let bounds = Bounds::centered(None, size(px(1100.0), px(760.0)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some("VorTeX - LaTeX Editor".into()),
                appears_transparent: true,
                traffic_light_position: None,
            }),
            ..Default::default()
        };

        cx.open_window(options, |_window, cx| {
            cx.new(|cx| VorTexApp::new(cx))
        })
        .expect("Failed to open VorTeX main window");
    });
}
