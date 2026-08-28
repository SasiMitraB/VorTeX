#![allow(dead_code)]

use crate::backend::{BackendClient, IndexStats, ProjectItem, SectionItem, TreeNode};
use std::collections::HashSet;

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
}
