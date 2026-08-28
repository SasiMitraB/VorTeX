#![allow(dead_code)]

use anyhow::Result;
use std::fs;
use std::sync::{Arc, Mutex};

#[allow(unused_imports)]
pub use crate::services::bibtex_parser::{BibEntryItem, BibFields};
pub use crate::services::compiler::{build_latex_project, BuildResult};
pub use crate::services::file_watcher::{FileChangeEvent, FileWatcher};
pub use crate::services::fs_utils::{build_tree, get_config_val, scan_projects, set_config_val, ProjectItem, TreeNode};
pub use crate::services::fuzzy_matcher::{FuzzyMatchResult, Matcher};
#[allow(unused_imports)]
pub use crate::services::latex_parser::{LabelItem, SectionItem};
pub use crate::services::semantic_index::{IndexStats, SemanticIndex};

pub type FileChangeCallback = Arc<dyn Fn(FileChangeEvent) + Send + Sync + 'static>;
pub type IndexReadyCallback = Arc<dyn Fn(IndexStats) + Send + Sync + 'static>;

#[derive(Clone)]
pub struct BackendClient {
    semantic_index: SemanticIndex,
    matcher: Arc<Matcher>,
    file_watcher: Arc<Mutex<Option<FileWatcher>>>,
    on_file_change: Arc<Mutex<Option<FileChangeCallback>>>,
    on_index_ready: Arc<Mutex<Option<IndexReadyCallback>>>,
}

impl Default for BackendClient {
    fn default() -> Self {
        Self {
            semantic_index: SemanticIndex::new(),
            matcher: Arc::new(Matcher::new()),
            file_watcher: Arc::new(Mutex::new(None)),
            on_file_change: Arc::new(Mutex::new(None)),
            on_index_ready: Arc::new(Mutex::new(None)),
        }
    }
}

impl BackendClient {
    pub fn new() -> Result<Self> {
        Ok(Self::default())
    }

    pub fn set_on_file_change<F>(&self, callback: F)
    where
        F: Fn(FileChangeEvent) + Send + Sync + 'static,
    {
        if let Ok(mut lock) = self.on_file_change.lock() {
            *lock = Some(Arc::new(callback));
        }
    }

    pub fn set_on_index_ready<F>(&self, callback: F)
    where
        F: Fn(IndexStats) + Send + Sync + 'static,
    {
        if let Ok(mut lock) = self.on_index_ready.lock() {
            *lock = Some(Arc::new(callback));
        }
    }

    pub fn ping(&self) -> Result<bool> {
        Ok(true)
    }

    pub fn read_tree(&self, root_path: &str) -> Result<Vec<TreeNode>> {
        Ok(build_tree(root_path))
    }

    pub fn read_file(&self, file_path: &str) -> Result<String> {
        fs::read_to_string(file_path).map_err(|e| anyhow::anyhow!("Failed to read file {}: {}", file_path, e))
    }

    pub fn write_file(&self, file_path: &str, content: &str) -> Result<bool> {
        fs::write(file_path, content)?;
        Ok(true)
    }

    pub fn config_get(&self, key: &str) -> Result<Option<serde_json::Value>> {
        Ok(get_config_val(key))
    }

    pub fn config_set(&self, key: &str, value: serde_json::Value) -> Result<bool> {
        set_config_val(key, value)?;
        Ok(true)
    }

    pub fn scan_projects(&self, projects_folder: &str) -> Result<Vec<ProjectItem>> {
        Ok(scan_projects(projects_folder))
    }

    pub fn init_project(&self, project_path: &str) -> Result<bool> {
        self.semantic_index.init_project(project_path);

        let index_clone = self.semantic_index.clone();
        let change_cb_clone = self.on_file_change.clone();

        // Setup native OS file watcher
        if let Ok(watcher) = FileWatcher::new(project_path, move |event| {
            if event.change_type == "deleted" {
                index_clone.remove_file(&event.path);
            } else if let Ok(content) = fs::read_to_string(&event.path) {
                index_clone.update_file(&event.path, &content);
            }

            if let Ok(cb_lock) = change_cb_clone.lock() {
                if let Some(ref cb) = *cb_lock {
                    cb(event);
                }
            }
        }) {
            if let Ok(mut watcher_lock) = self.file_watcher.lock() {
                *watcher_lock = Some(watcher);
            }
        }

        // Fire on_index_ready
        let stats = self.semantic_index.get_stats();
        if let Ok(ready_lock) = self.on_index_ready.lock() {
            if let Some(ref cb) = *ready_lock {
                cb(stats);
            }
        }

        Ok(true)
    }

    pub fn get_sections(&self, file_path: &str) -> Result<Vec<SectionItem>> {
        Ok(self.semantic_index.get_sections_for_file(file_path))
    }

    pub fn fuzzy_search_labels(&self, query: &str, current_file: Option<&str>) -> Result<Vec<FuzzyMatchResult>> {
        let labels = self.semantic_index.get_all_labels();
        Ok(self.matcher.search_labels(query, &labels, current_file))
    }

    pub fn fuzzy_search_citations(&self, query: &str, _current_file: Option<&str>) -> Result<Vec<FuzzyMatchResult>> {
        let bibs = self.semantic_index.get_all_citations();
        Ok(self.matcher.search_bib_entries(query, &bibs))
    }

    pub fn reindex_file(&self, file_path: &str) -> Result<IndexStats> {
        if let Ok(content) = fs::read_to_string(file_path) {
            self.semantic_index.update_file(file_path, &content);
        }
        Ok(self.semantic_index.get_stats())
    }

    pub fn get_stats(&self) -> Result<IndexStats> {
        Ok(self.semantic_index.get_stats())
    }

    pub fn build_project(&self, tex_file_path: &str) -> Result<BuildResult> {
        Ok(build_latex_project(tex_file_path))
    }

    pub fn stop(&self) -> Result<()> {
        if let Ok(mut watcher_lock) = self.file_watcher.lock() {
            *watcher_lock = None;
        }
        self.semantic_index.clear();
        Ok(())
    }
}
