#![allow(dead_code)]

use crate::backend::{BackendClient, IndexStats, ProjectItem, SectionItem, TreeNode};
use crate::services::synctex::SynctexForwardResult;
use crate::views::pdf_viewer::SynctexHighlight;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    ProjectSelector,
    Workspace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneSide {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabType {
    Text,
    Pdf,
}

#[derive(Debug, Clone)]
pub struct Tab {
    pub id: String,
    pub path: Option<String>,
    pub name: String,
    pub content: String,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub selection: Option<((usize, usize), (usize, usize))>, // ((start_row, start_col), (end_row, end_col))
    pub scroll_top: f32,
    pub dirty: bool,
    pub tab_type: TabType,
}

impl Tab {
    pub fn new_text(id: String, name: String, path: Option<String>, content: String) -> Self {
        Self {
            id,
            path,
            name,
            content,
            cursor_row: 0,
            cursor_col: 0,
            selection: None,
            scroll_top: 0.0,
            dirty: false,
            tab_type: TabType::Text,
        }
    }

    pub fn new_pdf(id: String, name: String, path: String) -> Self {
        Self {
            id,
            path: Some(path),
            name,
            content: String::new(),
            cursor_row: 0,
            cursor_col: 0,
            selection: None,
            scroll_top: 0.0,
            dirty: false,
            tab_type: TabType::Pdf,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PaneState {
    pub tabs: Vec<Tab>,
    pub active_tab_id: Option<String>,
}

impl PaneState {
    pub fn active_tab(&self) -> Option<&Tab> {
        self.active_tab_id
            .as_ref()
            .and_then(|id| self.tabs.iter().find(|t| &t.id == id))
    }

    pub fn active_tab_mut(&mut self) -> Option<&mut Tab> {
        if let Some(ref id) = self.active_tab_id.clone() {
            self.tabs.iter_mut().find(|t| &t.id == id)
        } else {
            None
        }
    }

    pub fn switch_tab(&mut self, id: &str) {
        if self.tabs.iter().any(|t| t.id == id) {
            self.active_tab_id = Some(id.to_string());
        }
    }

    pub fn close_tab(&mut self, id: &str) {
        if let Some(pos) = self.tabs.iter().position(|t| t.id == id) {
            self.tabs.remove(pos);
            if self.active_tab_id.as_deref() == Some(id) {
                if !self.tabs.is_empty() {
                    let next_pos = if pos >= self.tabs.len() {
                        self.tabs.len() - 1
                    } else {
                        pos
                    };
                    self.active_tab_id = Some(self.tabs[next_pos].id.clone());
                } else {
                    self.active_tab_id = None;
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct PdfViewerState {
    pub pdf_path: String,
    pub page_count: usize,
    pub zoom: f32,
    pub current_page: usize,
    pub scroll_handle: gpui::ScrollHandle,
    pub page_images: Vec<String>,
    pub page_width_pts: f32,
    pub page_height_pts: f32,
    pub highlight: Option<SynctexHighlight>,
    pub last_render_mtime: Option<std::time::SystemTime>,
    pub is_rendering: bool,
    pub render_error: Option<String>,
    pub is_syncing: bool,
}

impl PdfViewerState {
    pub fn new(pdf_path: String) -> Self {
        Self {
            pdf_path,
            page_count: 0,
            zoom: 1.0,
            current_page: 1,
            scroll_handle: gpui::ScrollHandle::new(),
            page_images: Vec::new(),
            page_width_pts: 595.276,
            page_height_pts: 841.89,
            highlight: None,
            last_render_mtime: None,
            is_rendering: false,
            render_error: None,
            is_syncing: false,
        }
    }

    pub fn zoom_in(&mut self) {
        self.zoom = (self.zoom + 0.25).min(3.0);
    }

    pub fn zoom_out(&mut self) {
        self.zoom = (self.zoom - 0.25).max(0.5);
    }

    pub fn zoom_reset(&mut self) {
        self.zoom = 1.0;
    }

    pub fn scroll_to_page(&mut self, page: usize, y_in_page: f32) {
        if self.page_count == 0 || page == 0 {
            return;
        }
        let gap = 16.0;
        let page_h = self.page_height_pts * self.zoom;
        let padding_top = 16.0;
        let page_offset = (page - 1) as f32 * (page_h + gap) + padding_top + y_in_page * self.zoom;
        // Center in viewport approx 400px offset, clamp
        let target = (page_offset - 200.0).max(0.0);
        self.scroll_handle
            .set_offset(gpui::point(gpui::px(0.0), gpui::px(-target)));
    }

    pub fn set_highlight_from_forward(&mut self, fwd: SynctexForwardResult) {
        let y = if fwd.height > 0.0 { fwd.v } else { fwd.y };
        let hl = SynctexHighlight::from(fwd);
        let page = hl.page;
        let y_in_page = y;
        self.highlight = Some(hl);
        self.scroll_to_page(page, y_in_page);
        self.current_page = page;
    }

    pub fn clear_highlight(&mut self) {
        self.highlight = None;
    }
}

pub struct AppState {
    pub backend: BackendClient,
    pub current_view: ViewMode,
    pub projects_folder: Option<String>,
    pub projects: Vec<ProjectItem>,
    pub current_project: Option<String>,
    pub file_tree: Vec<TreeNode>,
    pub expanded_folders: HashSet<String>,
    pub outline_sections: Vec<SectionItem>,
    pub index_stats: IndexStats,
    pub pane_left: PaneState,
    pub pane_right: PaneState,
    pub active_pane: PaneSide,
    pub sidebar_visible: bool,
    pub table_editor_open: bool,
    pub table_rows: usize,
    pub table_cols: usize,
    pub table_data: Vec<Vec<String>>,
    pub status_message: Option<String>,
    pub is_building: bool,
    pub next_untitled_num: usize,
    pub project_scroll_handle: gpui::ScrollHandle,
    pub sidebar_tree_scroll_handle: gpui::ScrollHandle,
    pub sidebar_outline_scroll_handle: gpui::ScrollHandle,
    pub table_scroll_handle: gpui::ScrollHandle,
    pub pdf_viewers: HashMap<String, PdfViewerState>,
    pub pdf_dpi: u32,
}

impl AppState {
    pub fn new(backend: BackendClient) -> Self {
        let mut table_data = Vec::new();
        for _ in 0..4 {
            table_data.push(vec![String::new(); 3]);
        }

        Self {
            backend,
            current_view: ViewMode::ProjectSelector,
            projects_folder: None,
            projects: Vec::new(),
            current_project: None,
            file_tree: Vec::new(),
            expanded_folders: HashSet::new(),
            outline_sections: Vec::new(),
            index_stats: IndexStats::default(),
            pane_left: PaneState::default(),
            pane_right: PaneState::default(),
            active_pane: PaneSide::Left,
            sidebar_visible: true,
            table_editor_open: false,
            table_rows: 4,
            table_cols: 3,
            table_data,
            status_message: None,
            is_building: false,
            next_untitled_num: 1,
            project_scroll_handle: gpui::ScrollHandle::new(),
            sidebar_tree_scroll_handle: gpui::ScrollHandle::new(),
            sidebar_outline_scroll_handle: gpui::ScrollHandle::new(),
            table_scroll_handle: gpui::ScrollHandle::new(),
            pdf_viewers: HashMap::new(),
            pdf_dpi: 144,
        }
    }

    pub fn active_pane_state(&self) -> &PaneState {
        match self.active_pane {
            PaneSide::Left => &self.pane_left,
            PaneSide::Right => &self.pane_right,
        }
    }

    pub fn active_pane_state_mut(&mut self) -> &mut PaneState {
        match self.active_pane {
            PaneSide::Left => &mut self.pane_left,
            PaneSide::Right => &mut self.pane_right,
        }
    }

    pub fn active_tab(&self) -> Option<&Tab> {
        self.active_pane_state().active_tab()
    }

    pub fn active_tab_mut(&mut self) -> Option<&mut Tab> {
        self.active_pane_state_mut().active_tab_mut()
    }

    pub fn generate_tab_id(&self) -> String {
        format!("tab_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos())
    }

    pub fn open_or_switch_file(&mut self, path: String, content: String) {
        let name = std::path::Path::new(&path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled")
            .to_string();

        let is_pdf = path.to_lowercase().ends_with(".pdf");

        // Check if tab already exists in active pane
        let pane = self.active_pane_state_mut();
        if let Some(existing) = pane.tabs.iter().find(|t| t.path.as_deref() == Some(&path)) {
            let id = existing.id.clone();
            pane.switch_tab(&id);
            return;
        }

        // Check other pane
        let other_pane = match self.active_pane {
            PaneSide::Left => &mut self.pane_right,
            PaneSide::Right => &mut self.pane_left,
        };
        if let Some(existing) = other_pane.tabs.iter().find(|t| t.path.as_deref() == Some(&path)) {
            let id = existing.id.clone();
            other_pane.switch_tab(&id);
            self.active_pane = match self.active_pane {
                PaneSide::Left => PaneSide::Right,
                PaneSide::Right => PaneSide::Left,
            };
            return;
        }

        // Create new tab in active pane
        let id = self.generate_tab_id();
        let tab = if is_pdf {
            Tab::new_pdf(id.clone(), name, path)
        } else {
            Tab::new_text(id.clone(), name, Some(path), content)
        };

        let active_p = self.active_pane_state_mut();
        active_p.tabs.push(tab);
        active_p.active_tab_id = Some(id);
    }

    pub fn create_new_tab(&mut self) {
        let num = self.next_untitled_num;
        self.next_untitled_num += 1;
        let id = self.generate_tab_id();
        let name = format!("Untitled-{}", num);
        let tab = Tab::new_text(id.clone(), name, None, String::new());

        let pane = self.active_pane_state_mut();
        pane.tabs.push(tab);
        pane.active_tab_id = Some(id);
    }

    pub fn get_or_create_pdf_viewer(&mut self, pdf_path: &str) -> &mut PdfViewerState {
        if !self.pdf_viewers.contains_key(pdf_path) {
            let state = PdfViewerState::new(pdf_path.to_string());
            self.pdf_viewers.insert(pdf_path.to_string(), state);
        }
        self.pdf_viewers.get_mut(pdf_path).unwrap()
    }

    pub fn ensure_pdf_viewer_loaded(&mut self, pdf_path: &str) {
        // Synchronous version for tests / simple use - still blocks on pdftoppm
        let dpi = self.pdf_dpi;
        // Check mtime to decide if re-render needed
        let pdf_mtime = std::fs::metadata(pdf_path)
            .and_then(|m| m.modified())
            .ok();

        let needs_render = {
            if let Some(state) = self.pdf_viewers.get(pdf_path) {
                if let Some(last) = state.last_render_mtime {
                    if let Some(cur) = pdf_mtime {
                        cur > last || state.page_images.is_empty()
                    } else {
                        state.page_images.is_empty()
                    }
                } else {
                    true
                }
            } else {
                true
            }
        };

        if !needs_render {
            return;
        }

        // Try to get page info and render (blocking)
        let page_count = crate::services::pdf_renderer::get_pdf_page_count(pdf_path).unwrap_or(0);
        let (pw, ph) =
            crate::services::pdf_renderer::get_pdf_page_dimensions(pdf_path).unwrap_or((595.276, 841.89));
        let cache = crate::services::pdf_renderer::ensure_pdf_rendered(pdf_path, dpi);

        let state = self.get_or_create_pdf_viewer(pdf_path);
        state.page_count = page_count;
        state.page_width_pts = pw;
        state.page_height_pts = ph;
        state.last_render_mtime = pdf_mtime;
        state.is_rendering = false;
        if let Some(c) = cache {
            state.page_images = c.page_images;
            state.page_count = c.page_count;
            state.render_error = None;
        } else if page_count == 0 {
            state.page_images.clear();
            state.render_error = Some("Failed to render PDF".to_string());
        }
    }

    /// Lightweight prepare for UI: ensures viewer entry exists, loads cached page info if available,
    /// and marks is_rendering if async render is needed. Returns true if async render should be spawned.
    pub fn prepare_pdf_viewer_async(&mut self, pdf_path: &str) -> bool {
        if !std::path::Path::new(pdf_path).exists() {
            // Ensure empty viewer
            let viewer = self.get_or_create_pdf_viewer(pdf_path);
            viewer.page_count = 0;
            viewer.page_images.clear();
            viewer.render_error = Some("PDF not found".to_string());
            viewer.is_rendering = false;
            return false;
        }

        let pdf_mtime = std::fs::metadata(pdf_path)
            .and_then(|m| m.modified())
            .ok();

        // Check if we already have a valid cache without heavy rendering
        let dpi = self.pdf_dpi;
        let needs_render = {
            if let Some(state) = self.pdf_viewers.get(pdf_path) {
                if state.is_rendering {
                    return false; // already rendering
                }
                if let Some(last) = state.last_render_mtime {
                    if let Some(cur) = pdf_mtime {
                        cur > last || state.page_images.is_empty()
                    } else {
                        state.page_images.is_empty()
                    }
                } else {
                    true
                }
            } else {
                true
            }
        };

        // Fast path: if cache is valid (checks file existence quickly), just ensure viewer has images
        if !needs_render {
            return false;
        }

        // Check if on-disk cache is valid (lightweight check via is_cache_valid but still needs pdfinfo)
        // We do a quick check to see if we can populate from cache without rendering
        if crate::services::pdf_renderer::is_cache_valid(pdf_path, dpi) {
            // Cache valid - populate viewer synchronously (fast, just path construction)
            let page_count = crate::services::pdf_renderer::get_pdf_page_count(pdf_path).unwrap_or(0);
            let (pw, ph) = crate::services::pdf_renderer::get_pdf_page_dimensions(pdf_path)
                .unwrap_or((595.276, 841.89));
            // Construct page image paths without rendering
            if let Some(cache_dir) = crate::services::pdf_renderer::get_pdf_cache_dir(pdf_path, dpi) {
                let mut page_images = Vec::new();
                for n in 1..=page_count.max(1) {
                    page_images.push(cache_dir.join(format!("page-{:04}.png", n)).to_string_lossy().to_string());
                }
                let viewer = self.get_or_create_pdf_viewer(pdf_path);
                viewer.page_count = page_count;
                viewer.page_width_pts = pw;
                viewer.page_height_pts = ph;
                viewer.page_images = page_images;
                viewer.last_render_mtime = pdf_mtime;
                viewer.is_rendering = false;
                viewer.render_error = None;
                return false;
            }
        }

        // Need async render - mark as rendering and set placeholder page count/dims if not set
        let viewer = self.get_or_create_pdf_viewer(pdf_path);
        if viewer.page_count == 0 {
            // Try to get quick page count for placeholder UI, even if cache invalid
            if let Some(pc) = crate::services::pdf_renderer::get_pdf_page_count(pdf_path) {
                viewer.page_count = pc;
            }
            if let Some((pw, ph)) = crate::services::pdf_renderer::get_pdf_page_dimensions(pdf_path) {
                viewer.page_width_pts = pw;
                viewer.page_height_pts = ph;
            }
        }
        viewer.is_rendering = true;
        viewer.render_error = None;
        true
    }

    pub fn finish_pdf_render(
        &mut self,
        pdf_path: &str,
        cache: Option<crate::services::pdf_renderer::PdfRenderCache>,
        page_count: usize,
        dims: (f32, f32),
    ) {
        let viewer = self.get_or_create_pdf_viewer(pdf_path);
        viewer.is_rendering = false;
        viewer.render_error = None;
        viewer.page_count = page_count;
        viewer.page_width_pts = dims.0;
        viewer.page_height_pts = dims.1;
        viewer.last_render_mtime = std::fs::metadata(pdf_path).and_then(|m| m.modified()).ok();
        if let Some(c) = cache {
            viewer.page_images = c.page_images;
            viewer.page_count = c.page_count;
        } else if page_count == 0 {
            viewer.page_images.clear();
            viewer.render_error = Some("Render failed".to_string());
        }
    }

    pub fn fail_pdf_render(&mut self, pdf_path: &str, error: String) {
        if let Some(viewer) = self.pdf_viewers.get_mut(pdf_path) {
            viewer.is_rendering = false;
            viewer.render_error = Some(error);
        }
    }
}
