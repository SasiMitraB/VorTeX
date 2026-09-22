mod actions;
mod backend;
mod icons;
pub mod services;
mod state;
mod theme;
mod views;

use backend::{BackendClient, FileChangeEvent, IndexStats};
use gpui::prelude::*;
use gpui::*;
use services::{git, latexdiff};
use state::{AppState, DiffSpec, DiffViewState, PaneSide, SidebarTab, TabType, ViewMode};
use std::sync::mpsc;
use std::sync::Arc;
use icons::{icon, IconName};
use theme::{detect_system_theme, Theme, ThemeMode, ThemePreference};
use views::diff_view::render_diff_view;
use views::latexdiff_dialog::render_latexdiff_dialog;
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

    pub fn show_projects(&mut self, cx: &mut Context<Self>) {
        self.state.current_view = ViewMode::ProjectSelector;
        if let Some(ref folder) = self.state.projects_folder {
            if let Ok(projs) = self.state.backend.scan_projects(folder) {
                self.state.projects = projs;
            }
        }
        cx.notify();
    }

    pub fn change_projects_folder(&mut self, cx: &mut Context<Self>) {
        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
            let folder_str = folder.to_str().unwrap().to_string();
            self.state.projects_folder = Some(folder_str.clone());
            let _ = self.state.backend.config_set("projectsFolder", serde_json::json!(folder_str));
            if let Ok(projs) = self.state.backend.scan_projects(&folder_str) {
                self.state.projects = projs;
            }
            cx.notify();
        }
    }

    pub fn open_file_dialog(&mut self, cx: &mut Context<Self>) {
        if let Some(file) = rfd::FileDialog::new()
            .add_filter("LaTeX Documents", &["tex", "bib", "pdf", "md", "txt"])
            .pick_file()
        {
            let path_str = file.to_str().unwrap().to_string();
            let content = self.state.backend.read_file(&path_str).unwrap_or_default();
            self.state.current_view = ViewMode::Workspace;
            self.open_file_in_active_pane(path_str, content, cx);
        }
    }

    pub fn open_folder_dialog(&mut self, cx: &mut Context<Self>) {
        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
            let folder_str = folder.to_str().unwrap().to_string();
            self.open_project(folder_str, cx);
        }
    }

    pub fn new_tab(&mut self, cx: &mut Context<Self>) {
        self.state.current_view = ViewMode::Workspace;
        self.state.create_new_tab();
        cx.notify();
    }

    pub fn close_active_tab(&mut self, cx: &mut Context<Self>) {
        if let Some(id) = self.state.active_pane_state().active_tab_id.clone() {
            self.state.active_pane_state_mut().close_tab(&id);
            cx.notify();
        }
    }

    pub fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.state.sidebar_visible = !self.state.sidebar_visible;
        cx.notify();
    }

    /// Opens the table editor, pre-loaded with the table under the cursor if there is one.
    pub fn open_table_editor(&mut self, cx: &mut Context<Self>) {
        let cur_file = self.state.active_tab().and_then(|t| t.path.clone());
        let table_under_cursor = self.editor_left.read(cx).find_table_at_cursor();
        if let Some((table_src, span)) = table_under_cursor {
            self.table_sheet.load_from_latex(&table_src, Some(span), cur_file);
        } else {
            self.table_sheet = TableSpreadsheet::default();
            self.table_sheet.model.source_file = cur_file;
        }
        self.table_sheet.is_open = true;
        cx.notify();
    }

    /// Absolute path of the file in the active tab (text or PDF).
    fn active_file_path(&self) -> Option<String> {
        self.state.active_tab().filter(|t| t.tab_type != TabType::Diff).and_then(|t| t.path.clone())
    }

    /// Reloads branch, changed files and the active file's history in the background.
    pub fn refresh_git(&mut self, cx: &mut Context<Self>) {
        let Some(project) = self.state.current_project.clone() else {
            return;
        };
        let active_file = self.active_file_path();
        self.state.git.refreshed_for = Some(active_file.clone());
        if self.state.git.refreshing {
            return;
        }
        self.state.git.refreshing = true;
        cx.notify();
        let requested_for = active_file.clone();

        cx.spawn(|this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let result = cx
                    .background_executor()
                    .spawn(async move {
                        let root = git::repo_root(std::path::Path::new(&project))?;
                        let branch = git::current_branch(&root);
                        let changes = git::status(&root);
                        let history_file = active_file
                            .as_deref()
                            .and_then(|p| git::relative_path(&root, std::path::Path::new(p)));
                        let history = history_file
                            .as_deref()
                            .map(|rel| git::file_log(&root, rel, 50))
                            .unwrap_or_default();
                        Some((root, branch, changes, history_file, history))
                    })
                    .await;
                let _ = this.update(&mut cx, |this, cx| {
                    let g = &mut this.state.git;
                    g.refreshing = false;
                    g.loaded = true;
                    match result {
                        Some((root, branch, changes, history_file, history)) => {
                            g.root = Some(root);
                            g.branch = branch;
                            g.changes = changes;
                            g.history_file = history_file;
                            g.history = history;
                        }
                        None => {
                            g.root = None;
                            g.changes.clear();
                            g.history.clear();
                            g.history_file = None;
                        }
                    }
                    // The active file changed while this refresh ran: load its history too.
                    if this.state.git.refreshed_for != Some(requested_for) {
                        this.refresh_git(cx);
                    }
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// Refresh button: also re-reads HEAD for the gutter markers (it may have moved).
    pub fn refresh_git_all(&mut self, cx: &mut Context<Self>) {
        self.editor_left.update(cx, |ed, cx| {
            ed.invalidate_git_base();
            cx.notify();
        });
        self.editor_right.update(cx, |ed, cx| {
            ed.invalidate_git_base();
            cx.notify();
        });
        self.refresh_git(cx);
    }

    /// Opens (or re-focuses) a diff tab comparing `spec.base_rev` with the working copy.
    pub fn open_git_diff(&mut self, spec: DiffSpec, cx: &mut Context<Self>) {
        let existing = self
            .state
            .diff_views
            .iter()
            .find(|(_, v)| v.spec == spec)
            .map(|(id, _)| id.clone())
            .filter(|id| self.state.active_pane_state().tabs.iter().any(|t| &t.id == id));
        let tab_id = match existing {
            Some(id) => id,
            None => {
                let id = self.state.generate_tab_id();
                let file_name = spec.rel_path.rsplit('/').next().unwrap_or(&spec.rel_path);
                let rev = if spec.base_rev == "HEAD" { "HEAD".to_string() } else { spec.base_rev.chars().take(7).collect() };
                let pane = self.state.active_pane_state_mut();
                pane.tabs.push(state::Tab::new_diff(id.clone(), format!("{file_name} ({rev})")));
                self.state.diff_views.insert(
                    id.clone(),
                    DiffViewState { spec, diff: None, message: None, scroll_handle: gpui::ScrollHandle::new() },
                );
                id
            }
        };
        self.state.active_pane_state_mut().switch_tab(&tab_id);
        self.load_diff(tab_id, cx);
    }

    /// (Re)computes a diff tab's contents. The working copy is the open editor's
    /// buffer when the file is being edited (so unsaved changes show), else the file on disk.
    pub fn load_diff(&mut self, tab_id: String, cx: &mut Context<Self>) {
        let Some(view) = self.state.diff_views.get(&tab_id) else {
            return;
        };
        let spec = view.spec.clone();
        let abs = spec.root.join(&spec.rel_path);
        let abs_str = abs.to_string_lossy().to_string();
        let buffer = [&self.editor_left, &self.editor_right].into_iter().find_map(|ed| {
            let ed = ed.read(cx);
            (ed.current_file_path.as_deref() == Some(abs_str.as_str())).then(|| ed.get_content())
        });

        cx.spawn(|this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let outcome = cx
                    .background_executor()
                    .spawn(async move {
                        let base = git::file_at_revision(&spec.root, &spec.base_rev, &spec.rel_path).unwrap_or_default();
                        let current = match buffer {
                            Some(text) => text.into_bytes(),
                            None => std::fs::read(&abs).unwrap_or_default(),
                        };
                        match (git::as_text(&base), git::as_text(&current)) {
                            (Some(old), Some(new)) => Ok(git::unified_diff(&old, &new, 3)),
                            _ => Err("Binary file — no text diff to show".to_string()),
                        }
                    })
                    .await;
                let _ = this.update(&mut cx, |this, cx| {
                    if let Some(view) = this.state.diff_views.get_mut(&tab_id) {
                        match outcome {
                            Ok(diff) => {
                                view.diff = Some(diff);
                                view.message = None;
                            }
                            Err(msg) => {
                                view.diff = None;
                                view.message = Some(msg);
                            }
                        }
                        cx.notify();
                    }
                });
            }
        })
        .detach();
    }

    pub fn open_git_change(&mut self, rel_path: String, cx: &mut Context<Self>) {
        let Some(root) = self.state.git.root.clone() else {
            return;
        };
        let spec = DiffSpec { root, rel_path, base_rev: "HEAD".to_string(), base_label: "HEAD".to_string() };
        self.open_git_diff(spec, cx);
    }

    pub fn open_git_commit(&mut self, commit: git::Commit, cx: &mut Context<Self>) {
        let (Some(root), Some(rel_path)) = (self.state.git.root.clone(), self.state.git.history_file.clone()) else {
            return;
        };
        let subject: String = commit.subject.chars().take(48).collect();
        let spec = DiffSpec {
            root,
            rel_path,
            base_label: format!("{} · {}", commit.short_hash, subject),
            base_rev: commit.hash,
        };
        self.open_git_diff(spec, cx);
    }

    /// Opens the "Compare Versions" dialog and loads the project's commit history.
    pub fn open_latexdiff_dialog(&mut self, cx: &mut Context<Self>) {
        let Some(project) = self.state.current_project.clone() else {
            return;
        };
        let d = &mut self.state.latexdiff;
        d.open = true;
        if d.running {
            cx.notify();
            return;
        }
        d.loading = true;
        d.error = None;
        cx.notify();
        let active = self.active_file_path();

        cx.spawn(|this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let loaded = cx
                    .background_executor()
                    .spawn(async move {
                        let project = std::path::PathBuf::from(project);
                        let root = git::repo_root(&project)?;
                        let scope = git::relative_path(&root, &project).unwrap_or_default();
                        let main_rel = latexdiff::find_main_document(active.as_deref().map(std::path::Path::new), &project)
                            .and_then(|main| git::relative_path(&root, &main));
                        let commits = git::log(&root, &scope, 200);
                        Some((root, scope, main_rel, commits))
                    })
                    .await;
                let _ = this.update(&mut cx, |this, cx| {
                    let d = &mut this.state.latexdiff;
                    d.loading = false;
                    match loaded {
                        Some((root, scope, main_rel, commits)) => {
                            d.repo_root = Some(root);
                            d.scope = scope;
                            d.main_rel = main_rel;
                            // Default: latest commit → working copy
                            d.old = (!commits.is_empty()).then_some(0);
                            d.new = None;
                            d.commits = commits;
                        }
                        None => {
                            d.repo_root = None;
                            d.commits.clear();
                            d.old = None;
                            d.new = None;
                        }
                    }
                    cx.notify();
                });
            }
        })
        .detach();
    }

    pub fn generate_latexdiff(&mut self, cx: &mut Context<Self>) {
        let d = &self.state.latexdiff;
        let (Some(root), Some(main_rel), Some(old)) = (d.repo_root.clone(), d.main_rel.clone(), d.old) else {
            return;
        };
        if d.running {
            return;
        }
        let old_rev = d.commits[old].hash.clone();
        let new_rev = d.new.map(|i| d.commits[i].hash.clone());
        let name = latexdiff::output_name(&main_rel, &old_rev, new_rev.as_deref());
        let main_dir = root.join(&main_rel).parent().map(|p| p.to_path_buf()).unwrap_or_else(|| root.clone());
        let req = latexdiff::LatexDiffRequest {
            repo_root: root,
            scope: d.scope.clone(),
            main_rel,
            old_rev,
            new_rev,
            output_pdf: main_dir.join(&name),
        };

        // The working copy is read from disk, so unsaved edits must land first.
        if req.new_rev.is_none() {
            self.save_active_file(cx);
        }
        let d = &mut self.state.latexdiff;
        d.running = true;
        d.error = None;
        self.state.status_message = Some("Generating change-tracking PDF…".to_string());
        cx.notify();

        cx.spawn(|this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let result = cx.background_executor().spawn(async move { latexdiff::generate(&req) }).await;
                let _ = this.update(&mut cx, |this, cx| {
                    this.state.latexdiff.running = false;
                    match result {
                        Ok(pdf) => {
                            this.state.latexdiff.open = false;
                            this.state.status_message = Some(format!("✓ Saved {name}"));
                            if let Some(ref proj) = this.state.current_project {
                                if let Ok(tree) = this.state.backend.read_tree(proj) {
                                    this.state.file_tree = tree;
                                }
                            }
                            this.show_pdf(pdf.to_string_lossy().to_string(), cx);
                        }
                        Err(err) => {
                            this.state.status_message = Some("✗ latexdiff PDF failed".to_string());
                            this.state.latexdiff.error = Some(err);
                        }
                    }
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// Shows `pdf_path` in the right pane (re-rendering it if it is already open).
    fn show_pdf(&mut self, pdf_path: String, cx: &mut Context<Self>) {
        if let Some(v) = self.state.pdf_viewers.get_mut(&pdf_path) {
            v.last_render_mtime = None;
            v.is_rendering = false;
        }
        let existing = [PaneSide::Left, PaneSide::Right].into_iter().find_map(|side| {
            let pane = match side {
                PaneSide::Left => &self.state.pane_left,
                PaneSide::Right => &self.state.pane_right,
            };
            pane.tabs.iter().find(|t| t.path.as_deref() == Some(pdf_path.as_str())).map(|t| (side, t.id.clone()))
        });
        match existing {
            Some((side, id)) => {
                let pane = match side {
                    PaneSide::Left => &mut self.state.pane_left,
                    PaneSide::Right => &mut self.state.pane_right,
                };
                pane.switch_tab(&id);
                self.state.active_pane = side;
            }
            None => {
                let id = self.state.generate_tab_id();
                let name = std::path::Path::new(&pdf_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Diff.pdf")
                    .to_string();
                self.state.pane_right.tabs.push(state::Tab::new_pdf(id.clone(), name, pdf_path.clone()));
                self.state.pane_right.active_tab_id = Some(id);
                self.state.active_pane = PaneSide::Right;
            }
        }
        self.state.get_or_create_pdf_viewer(&pdf_path);
        self.spawn_pdf_render_if_needed(pdf_path, cx);
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

        self.state.git = state::GitPanelState::default();
        self.state.diff_views.clear();
        self.refresh_git(cx);

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
            if tab.tab_type != TabType::Text {
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
                    self.refresh_git(cx);
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
                    if self.state.sidebar_tab == SidebarTab::Git {
                        self.refresh_git(cx);
                    }
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
                        view2.update(cx, |this, cx| this.change_projects_folder(cx));
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

                // Keep the Source Control panel's history in sync with the active file
                if sidebar_vis
                    && sidebar_tab == SidebarTab::Git
                    && self.state.git.refreshed_for.as_ref() != Some(&self.active_file_path())
                {
                    self.refresh_git(cx);
                }
                let git_state = self.state.git.clone();
                let v_git_refresh = view_handle.clone();
                let v_git_change = view_handle.clone();
                let v_git_commit = view_handle.clone();
                let v_git_latexdiff = view_handle.clone();
                let latexdiff_dialog = self.state.latexdiff.open.then(|| self.state.latexdiff.clone());
                let v_ld_old = view_handle.clone();
                let v_ld_new = view_handle.clone();
                let v_ld_gen = view_handle.clone();
                let v_ld_close = view_handle.clone();
                let left_diff = left_active_tab_diff(&self.state);
                let right_diff = right_active_tab_diff(&self.state);
                let v_diff_left = view_handle.clone();
                let v_diff_right = view_handle.clone();

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
                let v_build = view_handle.clone();

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

                div()
                    .size_full()
                    .bg(Theme::bg_app())
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .child(
                        // Top Toolbar
                        render_toolbar(
                            project_name,
                            sidebar_vis,
                            is_building,
                            move |_window, cx| {
                                v_back.update(cx, |this, cx| this.show_projects(cx));
                            },
                            move |_window, cx| {
                                v_toggle.update(cx, |this, cx| this.toggle_sidebar(cx));
                            },
                            move |_window, cx| {
                                v_build.update(cx, |this, cx| this.build_current_project(cx));
                            },
                            move |_window, cx| {
                                v_sync_forward.update(cx, |this, cx| this.forward_sync_to_pdf(cx));
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
                                            if tab == SidebarTab::Git {
                                                this.refresh_git(cx);
                                            }
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
                                    &git_state,
                                    move |_window, cx| {
                                        v_git_refresh.update(cx, |this, cx| this.refresh_git_all(cx));
                                    },
                                    move |rel_path, _window, cx| {
                                        v_git_change.update(cx, |this, cx| this.open_git_change(rel_path, cx));
                                    },
                                    move |commit, _window, cx| {
                                        v_git_commit.update(cx, |this, cx| this.open_git_commit(commit, cx));
                                    },
                                    move |_window, cx| {
                                        v_git_latexdiff.update(cx, |this, cx| this.open_latexdiff_dialog(cx));
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
                                                            TabType::Diff => match left_diff {
                                                                Some(view) => {
                                                                    let id = tab.id.clone();
                                                                    render_diff_view(&view, move |_w, cx| {
                                                                        v_diff_left.update(cx, |this, cx| this.load_diff(id.clone(), cx));
                                                                    })
                                                                    .into_any_element()
                                                                }
                                                                None => empty_pane_placeholder(),
                                                            },
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
                                                                TabType::Diff => match right_diff {
                                                                    Some(view) => {
                                                                        let id = tab.id.clone();
                                                                        render_diff_view(&view, move |_w, cx| {
                                                                            v_diff_right.update(cx, |this, cx| this.load_diff(id.clone(), cx));
                                                                        })
                                                                        .into_any_element()
                                                                    }
                                                                    None => empty_pane_placeholder(),
                                                                },
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
                    .when_some(latexdiff_dialog, move |d, dialog| {
                        d.child(render_latexdiff_dialog(
                            &dialog,
                            move |i, _window, cx| {
                                v_ld_old.update(cx, |this, cx| {
                                    this.state.latexdiff.old = Some(i);
                                    cx.notify();
                                });
                            },
                            move |i, _window, cx| {
                                v_ld_new.update(cx, |this, cx| {
                                    this.state.latexdiff.new = i;
                                    cx.notify();
                                });
                            },
                            move |_window, cx| {
                                v_ld_gen.update(cx, |this, cx| this.generate_latexdiff(cx));
                            },
                            move |_window, cx| {
                                v_ld_close.update(cx, |this, cx| {
                                    this.state.latexdiff.open = false;
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

fn left_active_tab_diff(state: &AppState) -> Option<DiffViewState> {
    let id = state.pane_left.active_tab_id.as_ref()?;
    state.diff_views.get(id).cloned()
}

fn right_active_tab_diff(state: &AppState) -> Option<DiffViewState> {
    let id = state.pane_right.active_tab_id.as_ref()?;
    state.diff_views.get(id).cloned()
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
        .child(icon(IconName::FilePlus).size(px(36.0)).text_color(Theme::text_dim()))
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

/// Routes an app-level action from the menu bar / key bindings to the main window's view.
fn on_app_action<A: Action>(
    cx: &mut App,
    handle: WindowHandle<VorTexApp>,
    f: impl Fn(&mut VorTexApp, &mut Context<VorTexApp>) + 'static,
) {
    cx.on_action(move |_: &A, cx| {
        let _ = handle.update(cx, |app, _window, cx| f(app, cx));
    });
}

fn register_app_actions(cx: &mut App, handle: WindowHandle<VorTexApp>) {
    cx.on_action(|_: &actions::Quit, cx| cx.quit());
    cx.on_action(|_: &actions::Hide, cx| cx.hide());
    cx.on_action(|_: &actions::HideOthers, cx| cx.hide_other_apps());
    cx.on_action(|_: &actions::ShowAll, cx| cx.unhide_other_apps());
    cx.on_action(|_: &actions::Minimize, cx| {
        if let Some(window) = cx.active_window() {
            let _ = window.update(cx, |_, window, _| window.minimize_window());
        }
    });
    cx.on_action(|_: &actions::Zoom, cx| {
        if let Some(window) = cx.active_window() {
            let _ = window.update(cx, |_, window, _| window.zoom_window());
        }
    });

    let in_workspace = |app: &VorTexApp| app.state.current_view == ViewMode::Workspace;
    on_app_action::<actions::NewTab>(cx, handle, |app, cx| app.new_tab(cx));
    on_app_action::<actions::OpenFile>(cx, handle, |app, cx| app.open_file_dialog(cx));
    on_app_action::<actions::OpenFolder>(cx, handle, |app, cx| app.open_folder_dialog(cx));
    on_app_action::<actions::ShowProjects>(cx, handle, |app, cx| app.show_projects(cx));
    on_app_action::<actions::ChangeProjectsFolder>(cx, handle, |app, cx| app.change_projects_folder(cx));
    on_app_action::<actions::ToggleTheme>(cx, handle, |app, cx| app.toggle_theme(cx));
    on_app_action::<actions::Save>(cx, handle, move |app, cx| {
        if in_workspace(app) {
            app.save_active_file(cx);
        }
    });
    on_app_action::<actions::CloseTab>(cx, handle, move |app, cx| {
        if in_workspace(app) {
            app.close_active_tab(cx);
        }
    });
    on_app_action::<actions::InsertTable>(cx, handle, move |app, cx| {
        if in_workspace(app) {
            app.open_table_editor(cx);
        }
    });
    on_app_action::<actions::ToggleSidebar>(cx, handle, move |app, cx| {
        if in_workspace(app) {
            app.toggle_sidebar(cx);
        }
    });
    on_app_action::<actions::Build>(cx, handle, move |app, cx| {
        if in_workspace(app) && !app.state.is_building {
            app.build_current_project(cx);
        }
    });
    on_app_action::<actions::CompareVersions>(cx, handle, move |app, cx| {
        if in_workspace(app) {
            app.open_latexdiff_dialog(cx);
        }
    });
    on_app_action::<actions::SyncPdf>(cx, handle, move |app, cx| {
        if in_workspace(app) {
            app.forward_sync_to_pdf(cx);
        }
    });
}

fn main() {
    Application::new().with_assets(icons::Assets).run(|cx: &mut App| {
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

        actions::bind_keys(cx);
        cx.set_menus(actions::app_menus());

        let handle = cx
            .open_window(options, |_window, cx| cx.new(|cx| VorTexApp::new(cx)))
            .expect("Failed to open VorTeX main window");
        register_app_actions(cx, handle);
    });
}
