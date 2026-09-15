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
use theme::{detect_system_theme, Theme, ThemeMode, ThemePreference};
use views::editor::LatexEditor;
#[allow(unused_imports)]
use views::pdf_preview::render_pdf_preview;
use views::pdf_viewer::render_pdf_viewer;
use views::project_selector::render_project_selector;
use views::sidebar::render_sidebar;
use views::status_bar::render_status_bar;
use views::table_editor::{render_table_editor_modal, TableSpreadsheet};
use views::tabs::render_tab_bar;
use views::workspace::render_toolbar;

pub struct VorTexApp {
    state: AppState,
    editor_left: Entity<LatexEditor>,
    editor_right: Entity<LatexEditor>,
    table_sheet: TableSpreadsheet,
    event_rx: Arc<std::sync::Mutex<mpsc::Receiver<BackendEvent>>>,
    event_tx: mpsc::Sender<BackendEvent>,
}

enum BackendEvent {
    FileChange(FileChangeEvent),
    IndexReady(IndexStats),
    BuildFinished(crate::services::compiler::BuildResult),
    PdfRenderFinished {
        pdf_path: String,
        page_count: usize,
        dims: (f32, f32),
        cache: Option<crate::services::pdf_renderer::PdfRenderCache>,
        error: Option<String>,
    },
    SynctexForwardFinished {
        pdf_path: String,
        line: usize,
        result: Option<crate::services::synctex::SynctexForwardResult>,
    },
    SynctexInverseFinished {
        result: Option<crate::services::synctex::SynctexInverseResult>,
    },
    OsAppearanceChanged(ThemeMode),
}

impl VorTexApp {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let backend = BackendClient::new().expect("Failed to start VorTeX native backend");

        let (tx, rx) = mpsc::channel();
        let tx_change = tx.clone();
        let tx_ready = tx.clone();
        let tx_theme = tx.clone();

        // Background thread to detect OS-level appearance changes dynamically
        std::thread::spawn(move || {
            let mut last_detected = detect_system_theme();
            loop {
                std::thread::sleep(std::time::Duration::from_millis(1000));
                let current = detect_system_theme();
                if current != last_detected {
                    last_detected = current;
                    let _ = tx_theme.send(BackendEvent::OsAppearanceChanged(current));
                }
            }
        });

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

        // Check if there is a saved theme preference
        if let Ok(Some(pref_val)) = app_state.backend.config_get("themePreference") {
            if let Some(pref_str) = pref_val.as_str() {
                let pref = match pref_str.to_lowercase().as_str() {
                    "dark" => ThemePreference::Dark,
                    "light" => ThemePreference::Light,
                    _ => ThemePreference::Auto,
                };
                app_state.theme_preference = pref;
            }
        }

        let effective_mode = match app_state.theme_preference {
            ThemePreference::Dark => ThemeMode::Dark,
            ThemePreference::Light => ThemeMode::Light,
            ThemePreference::Auto => detect_system_theme(),
        };
        Theme::set_mode(effective_mode);
        app_state.theme_mode = effective_mode;

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
            table_sheet: TableSpreadsheet::default(),
            event_rx: Arc::new(std::sync::Mutex::new(rx)),
            event_tx: tx,
        }
    }

    pub fn toggle_theme(&mut self, cx: &mut Context<Self>) {
        let next_pref = self.state.theme_preference.next();
        self.state.theme_preference = next_pref;
        let effective_mode = match next_pref {
            ThemePreference::Dark => ThemeMode::Dark,
            ThemePreference::Light => ThemeMode::Light,
            ThemePreference::Auto => detect_system_theme(),
        };
        Theme::set_mode(effective_mode);
        self.state.theme_mode = effective_mode;
        let pref_str = match next_pref {
            ThemePreference::Auto => "auto",
            ThemePreference::Light => "light",
            ThemePreference::Dark => "dark",
        };
        let _ = self.state.backend.config_set("themePreference", serde_json::json!(pref_str));
        self.state.status_message = Some(format!("Theme: {}", next_pref.label(effective_mode)));
        cx.notify();
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

        // Load project outline, tables, and todos
        if let Ok(sections) = self.state.backend.get_all_sections() {
            self.state.outline_sections = sections;
        }
        if let Ok(tables) = self.state.backend.get_tables(None) {
            self.state.outline_tables = tables;
        }
        if let Ok(todos) = self.state.backend.get_todos(None) {
            self.state.todos = todos;
        }
        // Load all labels
        let all_labels = self.state.backend.get_all_labels().unwrap_or_default();
        self.state.outline_labels = all_labels;

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

        if is_pdf {
            self.spawn_pdf_render_if_needed(path.clone(), cx);
        }

        if !is_pdf {
            let active_editor = match self.state.active_pane {
                PaneSide::Left => &self.editor_left,
                PaneSide::Right => &self.editor_right,
            };

            active_editor.update(cx, |editor, _cx| {
                editor.set_content(&content, Some(path.clone()));
            });

            // Update document outline, tables, and todos
            if let Ok(sections) = self.state.backend.get_all_sections() {
                if !sections.is_empty() {
                    self.state.outline_sections = sections;
                } else if let Ok(sec) = self.state.backend.get_sections(&path) {
                    self.state.outline_sections = sec;
                }
            } else if path.ends_with(".tex") {
                if let Ok(sections) = self.state.backend.get_sections(&path) {
                    self.state.outline_sections = sections;
                }
            }
            if let Ok(tables) = self.state.backend.get_tables(None) {
                self.state.outline_tables = tables;
            }
            if let Ok(todos) = self.state.backend.get_todos(None) {
                self.state.todos = todos;
            }
            // Refresh all labels
            let all_labels = self.state.backend.get_all_labels().unwrap_or_default();
            self.state.outline_labels = all_labels;
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

                    // Refresh outline, tables, and todos
                    if let Ok(sections) = self.state.backend.get_all_sections() {
                        self.state.outline_sections = sections;
                    }
                    if let Ok(tables) = self.state.backend.get_tables(None) {
                        self.state.outline_tables = tables;
                    }
                    if let Ok(todos) = self.state.backend.get_todos(None) {
                        self.state.todos = todos;
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

    pub fn resolve_pdf_path(&self) -> Option<String> {
        // 1. If active tab is PDF, use it
        if let Some(tab) = self.state.active_tab() {
            if tab.tab_type == TabType::Pdf {
                if let Some(ref p) = tab.path {
                    return Some(p.clone());
                }
            }
        }
        // 2. Check any open PDF tab
        for pane in [&self.state.pane_left, &self.state.pane_right] {
            for tab in &pane.tabs {
                if tab.tab_type == TabType::Pdf {
                    if let Some(ref p) = tab.path {
                        if std::path::Path::new(p).exists() {
                            return Some(p.clone());
                        }
                    }
                }
            }
        }
        // 3. Try to derive from build target (same stem .pdf)
        if let Some(tex_path) = self.resolve_build_target() {
            let tex_p = std::path::Path::new(&tex_path);
            if let Some(stem) = tex_p.file_stem().and_then(|s| s.to_str()) {
                if let Some(parent) = tex_p.parent() {
                    let pdf_candidate = parent.join(format!("{}.pdf", stem));
                    if pdf_candidate.exists() {
                        return Some(pdf_candidate.to_string_lossy().to_string());
                    }
                }
            }
        }
        // 4. Current project main.pdf
        if let Some(ref proj) = self.state.current_project {
            let main_pdf = std::path::Path::new(proj).join("main.pdf");
            if main_pdf.exists() {
                return Some(main_pdf.to_string_lossy().to_string());
            }
            if let Some(found) = crate::services::fs_utils::find_project_pdf(std::path::Path::new(proj)) {
                return Some(found);
            }
        }
        None
    }

    /// Spawn background PDF render if needed (non-blocking)
    pub fn spawn_pdf_render_if_needed(&mut self, pdf_path: String, cx: &mut Context<Self>) {
        // Quick prepare check on main thread (fast, no heavy pdftoppm)
        let needs = self.state.prepare_pdf_viewer_async(&pdf_path);
        if !needs {
            return;
        }
        // Already marked as is_rendering in prepare
        let tx = self.event_tx.clone();
        let dpi = self.state.pdf_dpi;
        cx.background_executor()
            .spawn(async move {
                // Heavy work off main thread
                let page_count = crate::services::pdf_renderer::get_pdf_page_count(&pdf_path).unwrap_or(0);
                let dims = crate::services::pdf_renderer::get_pdf_page_dimensions(&pdf_path)
                    .unwrap_or((595.276, 841.89));
                let cache = crate::services::pdf_renderer::ensure_pdf_rendered(&pdf_path, dpi);
                let result = if cache.is_some() {
                    Ok((page_count, dims, cache))
                } else if page_count == 0 {
                    Err("Failed to render PDF - pdfinfo returned 0 pages".to_string())
                } else {
                    // Cache may be None even if page_count>0 if rendering failed
                    Err("pdftoppm rendering failed".to_string())
                };
                match result {
                    Ok((pc, d, c)) => {
                        let _ = tx.send(BackendEvent::PdfRenderFinished {
                            pdf_path,
                            page_count: pc,
                            dims: d,
                            cache: c,
                            error: None,
                        });
                    }
                    Err(e) => {
                        let _ = tx.send(BackendEvent::PdfRenderFinished {
                            pdf_path,
                            page_count,
                            dims,
                            cache: None,
                            error: Some(e),
                        });
                    }
                }
            })
            .detach();
        self.state.status_message = Some("Rendering PDF...".to_string());
        cx.notify();
    }

    pub fn forward_sync_to_pdf(&mut self, cx: &mut Context<Self>) {
        // Get editor cursor and file path from active pane's editor
        let (cursor_row, cursor_col, file_path_opt) = {
            let editor = match self.state.active_pane {
                PaneSide::Left => &self.editor_left,
                PaneSide::Right => &self.editor_right,
            };
            let ed = editor.read(cx);
            (ed.cursor_row, ed.cursor_col, ed.current_file_path.clone())
        };

        let tex_path = if let Some(p) = file_path_opt {
            p
        } else if let Some(t) = self.resolve_build_target() {
            t
        } else {
            self.state.status_message = Some("No source file for SyncTeX forward search".to_string());
            cx.notify();
            return;
        };

        let pdf_path = match self.resolve_pdf_path() {
            Some(p) => p,
            None => {
                self.state.status_message = Some("No PDF found. Build first.".to_string());
                cx.notify();
                return;
            }
        };

        // Ensure viewer entry exists (lightweight)
        self.state.get_or_create_pdf_viewer(&pdf_path);
        // Trigger async render if needed to ensure PDF is viewable
        self.spawn_pdf_render_if_needed(pdf_path.clone(), cx);

        // 1-based line for synctex
        let line = cursor_row + 1;
        let col = cursor_col;
        let col_arg = col;
        let page_hint = self
            .state
            .pdf_viewers
            .get(&pdf_path)
            .map(|v| v.current_page)
            .unwrap_or(0);

        // Mark syncing state
        if let Some(v) = self.state.pdf_viewers.get_mut(&pdf_path) {
            v.is_syncing = true;
        }
        self.state.status_message = Some(format!("SyncTeX → page ? (line {})...", line));
        cx.notify();

        // Spawn background SyncTeX forward search
        let tx = self.event_tx.clone();
        let tex_clone = tex_path.clone();
        let pdf_clone = pdf_path.clone();
        cx.background_executor()
            .spawn(async move {
                let result = crate::services::synctex::synctex_forward_search(
                    &tex_clone, line, col_arg, &pdf_clone, page_hint,
                );
                let _ = tx.send(BackendEvent::SynctexForwardFinished {
                    pdf_path: pdf_clone,
                    line,
                    result,
                });
            })
            .detach();
    }

    pub fn handle_synctex_forward_result(
        &mut self,
        pdf_path: String,
        line: usize,
        result: Option<crate::services::synctex::SynctexForwardResult>,
        cx: &mut Context<Self>,
    ) {
        if let Some(v) = self.state.pdf_viewers.get_mut(&pdf_path) {
            v.is_syncing = false;
        }
        match result {
            Some(res) => {
                let page = res.page;
                self.state.status_message = Some(format!("SyncTeX → page {} (line {})", page, line));
                // Switch right pane to PDF if not already visible
                let is_open_left = self.state.pane_left.tabs.iter().any(|t| t.path.as_deref() == Some(&pdf_path));
                let is_open_right = self.state.pane_right.tabs.iter().any(|t| t.path.as_deref() == Some(&pdf_path));
                if !is_open_left && !is_open_right {
                    let id = self.state.generate_tab_id();
                    let name = std::path::Path::new(&pdf_path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Output.pdf")
                        .to_string();
                    self.state.pane_right.tabs.push(state::Tab::new_pdf(id.clone(), name, pdf_path.clone()));
                    self.state.pane_right.active_tab_id = Some(id);
                    self.state.active_pane = PaneSide::Right;
                    self.spawn_pdf_render_if_needed(pdf_path.clone(), cx);
                } else if is_open_left && !is_open_right {
                    self.state.active_pane = PaneSide::Left;
                } else if is_open_right {
                    self.state.active_pane = PaneSide::Right;
                    if let Some(tab) = self.state.pane_right.tabs.iter().find(|t| t.path.as_deref() == Some(&pdf_path)) {
                        let id = tab.id.clone();
                        self.state.pane_right.switch_tab(&id);
                    }
                }
                if let Some(viewer) = self.state.pdf_viewers.get_mut(&pdf_path) {
                    viewer.set_highlight_from_forward(res);
                }
                cx.notify();
            }
            None => {
                self.state.status_message = Some(format!("SyncTeX forward: no result for line {}", line));
                cx.notify();
            }
        }
    }

    pub fn spawn_inverse_search(
        &mut self,
        pdf_path: String,
        page: usize,
        x: f32,
        y: f32,
        cx: &mut Context<Self>,
    ) {
        self.state.status_message = Some(format!("SyncTeX ← querying page {}...", page));
        cx.notify();
        let tx = self.event_tx.clone();
        cx.background_executor()
            .spawn(async move {
                let result =
                    crate::services::synctex::synctex_inverse_search(&pdf_path, page, x, y);
                let _ = tx.send(BackendEvent::SynctexInverseFinished { result });
            })
            .detach();
    }

    pub fn handle_inverse_search(
        &mut self,
        result: crate::services::synctex::SynctexInverseResult,
        cx: &mut Context<Self>,
    ) {
        let input_path = result.input.clone();
        let line = result.line;
        let col = result.column;

        // Try to open the file (if not already open)
        let content = self
            .state
            .backend
            .read_file(&input_path)
            .unwrap_or_default();

        // Open or switch to this file in active pane (prefer left)
        let _prev_pane = self.state.active_pane;
        self.state.active_pane = PaneSide::Left;
        self.open_file_in_active_pane(input_path.clone(), content, cx);
        self.state.active_pane = PaneSide::Left;

        // Jump editor to line
        self.editor_left.update(cx, |ed, _cx| {
            ed.jump_to_line(line);
            // Try to set column if available
            if col > 0 {
                let max_col = ed.current_line_len();
                ed.cursor_col = col.min(max_col);
            }
        });

        self.state.status_message = Some(format!("SyncTeX ← {}:{}", input_path, line));
        cx.notify();
    }

    pub fn jump_to_document_location(&mut self, file_path: String, line: usize, cx: &mut Context<Self>) {
        let current_path = self.state.active_tab().and_then(|t| t.path.clone());
        if current_path.as_deref() != Some(&file_path) && !file_path.is_empty() {
            let content = self.state.backend.read_file(&file_path).unwrap_or_default();
            self.open_file_in_active_pane(file_path, content, cx);
        }

        let active_editor = match self.state.active_pane {
            PaneSide::Left => &self.editor_left,
            PaneSide::Right => &self.editor_right,
        };
        active_editor.update(cx, |editor, _cx| {
            editor.jump_to_line(line);
        });
        cx.notify();
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

    pub fn process_background_events(&mut self, cx: &mut Context<Self>) {
        // Drain events without holding lock during handling
        let events: Vec<BackendEvent> = if let Ok(rx_lock) = self.event_rx.lock() {
            let mut evs = Vec::new();
            while let Ok(ev) = rx_lock.try_recv() {
                evs.push(ev);
            }
            evs
        } else {
            Vec::new()
        };

        for ev in events {
            match ev {
                BackendEvent::FileChange(change) => {
                    self.state.status_message = Some(format!("File {}: {}", change.change_type, change.path));
                    if change.path.to_lowercase().ends_with(".pdf") {
                        if let Some(v) = self.state.pdf_viewers.get_mut(&change.path) {
                            v.last_render_mtime = None;
                            v.is_rendering = false;
                        }
                    } else if change.path.to_lowercase().ends_with(".tex") {
                        if let Ok(sections) = self.state.backend.get_all_sections() {
                            self.state.outline_sections = sections;
                        }
                        if let Ok(tables) = self.state.backend.get_tables(None) {
                            self.state.outline_tables = tables;
                        }
                        if let Ok(todos) = self.state.backend.get_todos(None) {
                            self.state.todos = todos;
                        }
                    }
                }
                BackendEvent::IndexReady(stats) => {
                    self.state.index_stats = stats;
                    self.state.status_message = Some("Semantic index ready".to_string());
                    if let Ok(sections) = self.state.backend.get_all_sections() {
                        self.state.outline_sections = sections;
                    }
                    if let Ok(tables) = self.state.backend.get_tables(None) {
                        self.state.outline_tables = tables;
                    }
                    if let Ok(todos) = self.state.backend.get_todos(None) {
                        self.state.todos = todos;
                    }
                }
                BackendEvent::BuildFinished(result) => {
                    self.state.is_building = false;
                    self.state.status_message = Some(result.message.clone());

                    if result.success {
                        if let Some(ref proj) = self.state.current_project {
                            if let Ok(tree) = self.state.backend.read_tree(proj) {
                                self.state.file_tree = tree;
                            }
                        }

                        if let Some(ref pdf_path) = result.pdf_path {
                            if let Some(v) = self.state.pdf_viewers.get_mut(pdf_path) {
                                v.last_render_mtime = None;
                                v.is_rendering = false;
                            }
                            // Defer actual rendering to next render's async spawn
                            let is_open_left = self.state.pane_left.tabs.iter().any(|t| t.path.as_deref() == Some(pdf_path));
                            let is_open_right = self.state.pane_right.tabs.iter().any(|t| t.path.as_deref() == Some(pdf_path));

                            if is_open_left || is_open_right {
                                // Already open - will be re-rendered async on next frame
                            } else if self.state.pane_right.tabs.is_empty() {
                                let id = self.state.generate_tab_id();
                                let name = std::path::Path::new(pdf_path)
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("Output.pdf")
                                    .to_string();
                                self.state.pane_right.tabs.push(state::Tab::new_pdf(id.clone(), name, pdf_path.clone()));
                                self.state.pane_right.active_tab_id = Some(id);
                                // Prepare placeholder
                                self.state.get_or_create_pdf_viewer(pdf_path);
                            }
                            cx.notify();
                        }
                    }
                    cx.notify();
                }
                BackendEvent::PdfRenderFinished {
                    pdf_path,
                    page_count,
                    dims,
                    cache,
                    error,
                } => {
                    if let Some(err) = error {
                        self.state.fail_pdf_render(&pdf_path, err.clone());
                        self.state.status_message = Some(format!("PDF render failed: {}", err));
                    } else {
                        self.state.finish_pdf_render(&pdf_path, cache, page_count, dims);
                        self.state.status_message = Some(format!("PDF rendered: {} pages", page_count));
                    }
                    cx.notify();
                }
                BackendEvent::SynctexForwardFinished {
                    pdf_path,
                    line,
                    result,
                } => {
                    self.handle_synctex_forward_result(pdf_path, line, result, cx);
                }
                BackendEvent::SynctexInverseFinished { result } => {
                    if let Some(inv) = result {
                        self.handle_inverse_search(inv, cx);
                    } else {
                        self.state.status_message = Some("SyncTeX inverse: no result".to_string());
                        cx.notify();
                    }
                }
                BackendEvent::OsAppearanceChanged(mode) => {
                    if self.state.theme_preference == ThemePreference::Auto {
                        Theme::set_mode(mode);
                        self.state.theme_mode = mode;
                        cx.notify();
                    }
                }
            }
        }
    }
}

impl Render for VorTexApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.state.theme_preference == ThemePreference::Auto {
            let os_mode = match window.appearance() {
                WindowAppearance::Dark | WindowAppearance::VibrantDark => ThemeMode::Dark,
                WindowAppearance::Light | WindowAppearance::VibrantLight => ThemeMode::Light,
            };
            if Theme::mode() != os_mode {
                Theme::set_mode(os_mode);
                self.state.theme_mode = os_mode;
            }
        }

        self.process_background_events(cx);

        let current_view = self.state.current_view;
        let view_handle = cx.entity().clone();
        let theme_label = self.state.theme_preference.label(self.state.theme_mode);

        match current_view {
            ViewMode::ProjectSelector => {
                let projects = self.state.projects.clone();
                let projects_folder = self.state.projects_folder.clone();
                let proj_scroll = self.state.project_scroll_handle.clone();

                let view1 = view_handle.clone();
                let view2 = view_handle.clone();
                let view_theme = view_handle.clone();

                render_project_selector(
                    &projects,
                    projects_folder.as_deref(),
                    &proj_scroll,
                    &theme_label,
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
                    move |_window, cx| {
                        view_theme.update(cx, |this, cx| {
                            this.toggle_theme(cx);
                        });
                    },
                )
                .into_any_element()
            }

            ViewMode::Workspace => {
                let project_name = self.state.current_project.as_ref().map(|p| {
                    std::path::Path::new(p).file_name().and_then(|n| n.to_str()).unwrap_or(p.as_str()).to_string()
                });

                let sidebar_vis = self.state.sidebar_visible;
                let sidebar_tab = self.state.sidebar_tab;
                let file_tree = self.state.file_tree.clone();
                let expanded_folders = self.state.expanded_folders.clone();
                let outline_sections = self.state.outline_sections.clone();
                let _outline_tables = self.state.outline_tables.clone();
                let outline_labels = self.state.outline_labels.clone();
                let todos = self.state.todos.clone();
                let sidebar_label_filter = self.state.sidebar_label_filter;
                let active_tab_path = self.state.active_tab().and_then(|t| t.path.clone());
                let act_sidebar = active_tab_path.clone();
                let act_statusbar = active_tab_path.clone();

                let tree_scroll = self.state.sidebar_tree_scroll_handle.clone();
                let outline_scroll = self.state.sidebar_outline_scroll_handle.clone();
                let todo_scroll = self.state.sidebar_todo_scroll_handle.clone();

                let left_tabs = self.state.pane_left.tabs.clone();
                let left_active_id = self.state.pane_left.active_tab_id.clone();
                let right_tabs = self.state.pane_right.tabs.clone();
                let right_active_id = self.state.pane_right.active_tab_id.clone();
                let has_right_pane = !right_tabs.is_empty();

                let left_active_tab = self.state.pane_left.active_tab().cloned();
                let right_active_tab = self.state.pane_right.active_tab().cloned();

                let table_open = self.table_sheet.is_open;
                let table_sheet_clone = self.table_sheet.default_clone();
                let table_scroll = self.state.table_scroll_handle.clone();

                // Ensure PDF viewers are prepared; spawn async render if needed (non-blocking)
                {
                    let mut to_ensure = Vec::new();
                    if let Some(ref tab) = left_active_tab {
                        if tab.tab_type == TabType::Pdf {
                            if let Some(ref p) = tab.path {
                                to_ensure.push(p.clone());
                            }
                        }
                    }
                    if let Some(ref tab) = right_active_tab {
                        if tab.tab_type == TabType::Pdf {
                            if let Some(ref p) = tab.path {
                                to_ensure.push(p.clone());
                            }
                        }
                    }
                    for pdf_path in to_ensure {
                        self.spawn_pdf_render_if_needed(pdf_path, cx);
                    }
                }

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
                let v_select_sidebar_tab = view_handle.clone();
                let v_jump_location = view_handle.clone();
                let v_open_table = view_handle.clone();
                let v_label_filter = view_handle.clone();

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
                let v_tbl_sel = view_handle.clone();
                let v_tbl_ed = view_handle.clone();
                let v_tbl_ch = view_handle.clone();
                let v_tbl_ar = view_handle.clone();
                let v_tbl_rr = view_handle.clone();
                let v_tbl_ac = view_handle.clone();
                let v_tbl_rc = view_handle.clone();
                let v_tbl_cyc = view_handle.clone();
                let v_tbl_bt = view_handle.clone();
                let v_tbl_mr = view_handle.clone();
                let v_tbl_md = view_handle.clone();
                let v_tbl_um = view_handle.clone();
                let v_tbl_cap = view_handle.clone();
                let v_tbl_lbl = view_handle.clone();

                let v_left_zoom_in = view_handle.clone();
                let v_left_zoom_out = view_handle.clone();
                let v_left_zoom_reset = view_handle.clone();
                let v_left_reload = view_handle.clone();
                let v_left_inverse = view_handle.clone();

                let v_right_zoom_in = view_handle.clone();
                let v_right_zoom_out = view_handle.clone();
                let v_right_zoom_reset = view_handle.clone();
                let v_right_reload = view_handle.clone();
                let v_right_inverse = view_handle.clone();

                let v_sync_forward = view_handle.clone();
                let v_theme = view_handle.clone();

                div()
                    .size_full()
                    .bg(Theme::bg_app())
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                        let key = event.keystroke.key.as_str();
                        let is_cmd = event.keystroke.modifiers.platform || event.keystroke.modifiers.control;
                        let is_shift = event.keystroke.modifiers.shift;

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
                                "j" => {
                                    // Forward SyncTeX: editor -> PDF (Cmd+J, Cmd+Shift+J both)
                                    this.forward_sync_to_pdf(cx);
                                }
                                "g" => {
                                    // Also support Cmd+G for forward sync
                                    if is_shift {
                                        this.forward_sync_to_pdf(cx);
                                    }
                                }
                                "t" | "T" => {
                                    if is_shift {
                                        this.toggle_theme(cx);
                                    }
                                }
                                _ => {}
                            }
                        }
                        // Also handle Cmd+Shift+J without relying on key match above? Already covered
                        if is_cmd && is_shift && key == "j" {
                            this.forward_sync_to_pdf(cx);
                        }
                        if is_cmd && is_shift && (key == "t" || key == "T") {
                            this.toggle_theme(cx);
                        }
                    }))
                    .child(
                        // Top Toolbar
                        render_toolbar(
                            project_name,
                            sidebar_vis,
                            is_building,
                            &theme_label,
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
                                    let cur_file = this.state.active_tab().and_then(|t| t.path.clone());
                                    let table_under_cursor = this.editor_left.read(cx).find_table_at_cursor();
                                    if let Some((table_src, span)) = table_under_cursor {
                                        this.table_sheet.load_from_latex(&table_src, Some(span), cur_file);
                                    } else {
                                        this.table_sheet = TableSpreadsheet::default();
                                        this.table_sheet.model.source_file = cur_file;
                                    }
                                    this.table_sheet.is_open = true;
                                    cx.notify();
                                });
                            },
                            move |_window, cx| {
                                v_sync_forward.update(cx, |this, cx| {
                                    this.forward_sync_to_pdf(cx);
                                });
                            },
                            move |_window, cx| {
                                v_theme.update(cx, |this, cx| {
                                    this.toggle_theme(cx);
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
                                    sidebar_tab,
                                    &file_tree,
                                    &expanded_folders,
                                    &outline_sections,
                                    &outline_labels,
                                    sidebar_label_filter,
                                    &todos,
                                    act_sidebar.as_deref(),
                                    &tree_scroll,
                                    &outline_scroll,
                                    &todo_scroll,
                                    move |tab, _window, cx| {
                                        v_select_sidebar_tab.update(cx, |this, cx| {
                                            this.state.sidebar_tab = tab;
                                            cx.notify();
                                        });
                                    },
                                    move |filter, _window, cx| {
                                        v_label_filter.update(cx, |this, cx| {
                                            this.state.sidebar_label_filter = filter;
                                            cx.notify();
                                        });
                                    },
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
                                    move |file_path, line_num, _window, cx| {
                                        v_jump_location.update(cx, |this, cx| {
                                            this.jump_to_document_location(file_path, line_num, cx);
                                        });
                                    },
                                    move |file_path, byte_start, byte_end, _window, cx| {
                                        v_open_table.update(cx, |this, cx| {
                                            let content = this.state.backend.read_file(&file_path).unwrap_or_default();
                                            this.open_file_in_active_pane(file_path.clone(), content.clone(), cx);
                                            if byte_start < content.len() && byte_end <= content.len() && byte_start < byte_end {
                                                let snippet = &content[byte_start..byte_end];
                                                this.table_sheet.load_from_latex(snippet, Some(byte_start..byte_end), Some(file_path));
                                            } else {
                                                this.table_sheet = TableSpreadsheet::default();
                                                this.table_sheet.model.source_file = Some(file_path);
                                            }
                                            this.table_sheet.is_open = true;
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
                                                    .size_full()
                                                    .overflow_hidden()
                                                    .flex()
                                                    .flex_col()
                                                    .child(if let Some(tab) = left_active_tab {
                                                        match tab.tab_type {
                                                            TabType::Text => ed_left.into_any_element(),
                                                            TabType::Pdf => {
                                                                let pdf_path = tab.path.clone().unwrap_or_default();
                                                                let viewer = self.state.pdf_viewers.get(&pdf_path).cloned();
                                                                let (page_images, page_count, zoom, highlight, scroll_handle, pw, ph, is_rendering) = if let Some(v) = viewer {
                                                                    (v.page_images.clone(), v.page_count, v.zoom, v.highlight.clone(), v.scroll_handle.clone(), v.page_width_pts, v.page_height_pts, v.is_rendering)
                                                                } else {
                                                                    (Vec::new(), 0, 1.0, None, gpui::ScrollHandle::new(), 595.276, 841.89, false)
                                                                };
                                                                let p1 = pdf_path.clone();
                                                                let p2 = pdf_path.clone();
                                                                let p3 = pdf_path.clone();
                                                                let p4 = pdf_path.clone();
                                                                let p5 = pdf_path.clone();
                                                                render_pdf_viewer(
                                                                    &pdf_path,
                                                                    &page_images,
                                                                    page_count,
                                                                    zoom,
                                                                    highlight,
                                                                    &scroll_handle,
                                                                    pw,
                                                                    ph,
                                                                    sidebar_vis,
                                                                    has_right_pane,
                                                                    false,
                                                                    {
                                                                        let pdf = p1.clone();
                                                                        let v = v_left_zoom_in.clone();
                                                                        move |_w, cx| {
                                                                            v.update(cx, |this, cx| {
                                                                                if let Some(viewer) = this.state.pdf_viewers.get_mut(&pdf) {
                                                                                    viewer.zoom_in();
                                                                                    cx.notify();
                                                                                }
                                                                            });
                                                                        }
                                                                    },
                                                                    {
                                                                        let pdf = p2.clone();
                                                                        let v = v_left_zoom_out.clone();
                                                                        move |_w, cx| {
                                                                            v.update(cx, |this, cx| {
                                                                                if let Some(viewer) = this.state.pdf_viewers.get_mut(&pdf) {
                                                                                    viewer.zoom_out();
                                                                                    cx.notify();
                                                                                }
                                                                            });
                                                                        }
                                                                    },
                                                                    {
                                                                        let pdf = p3.clone();
                                                                        let v = v_left_zoom_reset.clone();
                                                                        move |_w, cx| {
                                                                            v.update(cx, |this, cx| {
                                                                                if let Some(viewer) = this.state.pdf_viewers.get_mut(&pdf) {
                                                                                    viewer.zoom_reset();
                                                                                    cx.notify();
                                                                                }
                                                                            });
                                                                        }
                                                                    },
                                                                    {
                                                                        let pdf = p4.clone();
                                                                        let v = v_left_reload.clone();
                                                                        move |_w, cx| {
                                                                            v.update(cx, |this, cx| {
                                                                                crate::services::pdf_renderer::clear_pdf_cache(&pdf);
                                                                                if let Some(viewer) = this.state.pdf_viewers.get_mut(&pdf) {
                                                                                    viewer.last_render_mtime = None;
                                                                                    viewer.is_rendering = false;
                                                                                    viewer.render_error = None;
                                                                                }
                                                                                this.spawn_pdf_render_if_needed(pdf.clone(), cx);
                                                                            });
                                                                        }
                                                                    },
                                                                    {
                                                                        let pdf = p5.clone();
                                                                        move |_w, _cx| {
                                                                            let _ = open::that(&pdf);
                                                                        }
                                                                    },
                                                                    {
                                                                        let v = v_left_inverse.clone();
                                                                        move |pdf_path, page, x, y, _win, cx| {
                                                                            v.update(cx, |this, cx| {
                                                                                this.spawn_inverse_search(pdf_path, page, x, y, cx);
                                                                            });
                                                                        }
                                                                    },
                                                                    is_rendering,
                                                                ).into_any_element()
                                                            }
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
                                                        .size_full()
                                                        .overflow_hidden()
                                                        .flex()
                                                        .flex_col()
                                                        .child(if let Some(tab) = right_active_tab {
                                                            match tab.tab_type {
                                                                TabType::Text => right_ed.into_any_element(),
                                                                TabType::Pdf => {
                                                                    let pdf_path = tab.path.clone().unwrap_or_default();
                                                                    let viewer = self.state.pdf_viewers.get(&pdf_path).cloned();
                                                                    let (page_images, page_count, zoom, highlight, scroll_handle, pw, ph, is_rendering) = if let Some(v) = viewer {
                                                                        (v.page_images.clone(), v.page_count, v.zoom, v.highlight.clone(), v.scroll_handle.clone(), v.page_width_pts, v.page_height_pts, v.is_rendering)
                                                                    } else {
                                                                        (Vec::new(), 0, 1.0, None, gpui::ScrollHandle::new(), 595.276, 841.89, false)
                                                                    };
                                                                    let p1 = pdf_path.clone();
                                                                    let p2 = pdf_path.clone();
                                                                    let p3 = pdf_path.clone();
                                                                    let p4 = pdf_path.clone();
                                                                    let p5 = pdf_path.clone();
                                                                    render_pdf_viewer(
                                                                        &pdf_path,
                                                                        &page_images,
                                                                        page_count,
                                                                        zoom,
                                                                        highlight,
                                                                        &scroll_handle,
                                                                        pw,
                                                                        ph,
                                                                        sidebar_vis,
                                                                        has_right_pane,
                                                                        true,
                                                                        {
                                                                            let pdf = p1.clone();
                                                                            let v = v_right_zoom_in.clone();
                                                                            move |_w, cx| {
                                                                                v.update(cx, |this, cx| {
                                                                                    if let Some(viewer) = this.state.pdf_viewers.get_mut(&pdf) {
                                                                                        viewer.zoom_in();
                                                                                        cx.notify();
                                                                                    }
                                                                                });
                                                                            }
                                                                        },
                                                                        {
                                                                            let pdf = p2.clone();
                                                                            let v = v_right_zoom_out.clone();
                                                                            move |_w, cx| {
                                                                                v.update(cx, |this, cx| {
                                                                                    if let Some(viewer) = this.state.pdf_viewers.get_mut(&pdf) {
                                                                                        viewer.zoom_out();
                                                                                        cx.notify();
                                                                                    }
                                                                                });
                                                                            }
                                                                        },
                                                                        {
                                                                            let pdf = p3.clone();
                                                                            let v = v_right_zoom_reset.clone();
                                                                            move |_w, cx| {
                                                                                v.update(cx, |this, cx| {
                                                                                    if let Some(viewer) = this.state.pdf_viewers.get_mut(&pdf) {
                                                                                        viewer.zoom_reset();
                                                                                        cx.notify();
                                                                                    }
                                                                                });
                                                                            }
                                                                        },
                                                                        {
                                                                            let pdf = p4.clone();
                                                                            let v = v_right_reload.clone();
                                                                            move |_w, cx| {
                                                                                v.update(cx, |this, cx| {
                                                                                    crate::services::pdf_renderer::clear_pdf_cache(&pdf);
                                                                                    if let Some(viewer) = this.state.pdf_viewers.get_mut(&pdf) {
                                                                                        viewer.last_render_mtime = None;
                                                                                        viewer.is_rendering = false;
                                                                                        viewer.render_error = None;
                                                                                    }
                                                                                    this.spawn_pdf_render_if_needed(pdf.clone(), cx);
                                                                                });
                                                                            }
                                                                        },
                                                                        {
                                                                            let pdf = p5.clone();
                                                                            move |_w, _cx| {
                                                                                let _ = open::that(&pdf);
                                                                            }
                                                                        },
                                                                        {
                                                                            let v = v_right_inverse.clone();
                                                                            move |pdf_path, page, x, y, _win, cx| {
                                                                                v.update(cx, |this, cx| {
                                                                                    this.spawn_inverse_search(pdf_path, page, x, y, cx);
                                                                                });
                                                                            }
                                                                        },
                                                                        is_rendering,
                                                                    ).into_any_element()
                                                                }
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
                            &table_sheet_clone,
                            &table_scroll,
                            move |latex_code, _window, cx| {
                                v_tbl_ins.update(cx, |this, cx| {
                                    let span_opt = this.table_sheet.model.source_span.clone();
                                    if let Some(span) = span_opt {
                                        this.editor_left.update(cx, |ed, _cx| {
                                            ed.replace_range(span, &latex_code);
                                        });
                                    } else {
                                        this.editor_left.update(cx, |ed, _cx| {
                                            ed.insert_text(&latex_code);
                                        });
                                    }
                                    this.table_sheet.is_open = false;
                                    cx.notify();
                                });
                            },
                            move |_window, cx| {
                                v_tbl_cls.update(cx, |this, cx| {
                                    this.table_sheet.is_open = false;
                                    cx.notify();
                                });
                            },
                            move |r, c, _window, cx| {
                                v_tbl_sel.update(cx, |this, cx| {
                                    if this.table_sheet.active_cell == Some((r, c)) && this.table_sheet.editing_cell.is_none() {
                                        this.table_sheet.start_edit(r, c);
                                    } else {
                                        this.table_sheet.commit_edit();
                                        this.table_sheet.active_cell = Some((r, c));
                                    }
                                    cx.notify();
                                });
                            },
                            move |r, c, _window, cx| {
                                v_tbl_ed.update(cx, |this, cx| {
                                    this.table_sheet.start_edit(r, c);
                                    cx.notify();
                                });
                            },
                            move |r, c, val, _window, cx| {
                                v_tbl_ch.update(cx, |this, cx| {
                                    this.table_sheet.set_cell(r, c, val);
                                    cx.notify();
                                });
                            },
                            move |_window, cx| {
                                v_tbl_ar.update(cx, |this, cx| {
                                    this.table_sheet.add_row();
                                    cx.notify();
                                });
                            },
                            move |_window, cx| {
                                v_tbl_rr.update(cx, |this, cx| {
                                    this.table_sheet.remove_row();
                                    cx.notify();
                                });
                            },
                            move |_window, cx| {
                                v_tbl_ac.update(cx, |this, cx| {
                                    this.table_sheet.add_col();
                                    cx.notify();
                                });
                            },
                            move |_window, cx| {
                                v_tbl_rc.update(cx, |this, cx| {
                                    this.table_sheet.remove_col();
                                    cx.notify();
                                });
                            },
                            move |col, _window, cx| {
                                v_tbl_cyc.update(cx, |this, cx| {
                                    this.table_sheet.cycle_col_align(col);
                                    cx.notify();
                                });
                            },
                            move |_window, cx| {
                                v_tbl_bt.update(cx, |this, cx| {
                                    this.table_sheet.toggle_booktabs();
                                    cx.notify();
                                });
                            },
                            move |r, c, _window, cx| {
                                v_tbl_mr.update(cx, |this, cx| {
                                    if c + 1 < this.table_sheet.model.num_cols() {
                                        this.table_sheet.merge_selection(r, c, r, c + 1);
                                        cx.notify();
                                    }
                                });
                            },
                            move |r, c, _window, cx| {
                                v_tbl_md.update(cx, |this, cx| {
                                    if r + 1 < this.table_sheet.model.num_rows() {
                                        this.table_sheet.merge_selection(r, c, r + 1, c);
                                        cx.notify();
                                    }
                                });
                            },
                            move |r, c, _window, cx| {
                                v_tbl_um.update(cx, |this, cx| {
                                    this.table_sheet.unmerge_cell(r, c);
                                    cx.notify();
                                });
                            },
                            move |caption, _window, cx| {
                                v_tbl_cap.update(cx, |this, cx| {
                                    this.table_sheet.caption_buffer = caption;
                                    this.table_sheet.sync_caption_label();
                                    cx.notify();
                                });
                            },
                            move |label, _window, cx| {
                                v_tbl_lbl.update(cx, |this, cx| {
                                    this.table_sheet.label_buffer = label;
                                    this.table_sheet.sync_caption_label();
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
