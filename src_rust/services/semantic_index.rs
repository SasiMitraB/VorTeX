use crate::services::bibtex_parser::{parse_bib_file, BibEntryItem};
use crate::services::latex_parser::{
    parse_latex_file, BibRef, CitationItem, LabelItem, RefItem, SectionItem, TodoItem,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

const INDEX_VERSION: usize = 1;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IndexStats {
    #[serde(default)]
    pub labels: usize,
    #[serde(default)]
    pub bibentries: usize,
    #[serde(default, rename = "filesIndexed")]
    pub files_indexed: usize,
    pub project: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiskIndexData {
    pub version: usize,
    pub project: Option<String>,
    pub timestamp: i64,
    pub labels: Vec<(String, LabelItem)>,
    pub bibentries: Vec<(String, BibEntryItem)>,
    pub citations: Vec<(String, Vec<CitationItem>)>,
    pub refs: Vec<(String, Vec<RefItem>)>,
    pub sections: Vec<(String, Vec<SectionItem>)>,
    #[serde(default)]
    pub todos: Vec<(String, Vec<TodoItem>)>,
    pub bibliographies: Vec<BibRef>,
    pub file_hashes: Vec<(String, String)>,
}

#[derive(Default)]
pub struct SemanticIndexInner {
    pub labels: HashMap<String, LabelItem>,
    pub bibentries: HashMap<String, BibEntryItem>,
    pub citations: HashMap<String, Vec<CitationItem>>,
    pub refs: HashMap<String, Vec<RefItem>>,
    pub sections: HashMap<String, Vec<SectionItem>>,
    pub todos: HashMap<String, Vec<TodoItem>>,
    pub bibliographies: Vec<BibRef>,
    pub file_hashes: HashMap<String, String>,
    pub current_project: Option<String>,
}

#[derive(Clone)]
pub struct SemanticIndex {
    inner: Arc<RwLock<SemanticIndexInner>>,
    save_lock: Arc<Mutex<()>>,
}

impl Default for SemanticIndex {
    fn default() -> Self {
        Self {
            inner: Arc::new(RwLock::new(SemanticIndexInner::default())),
            save_lock: Arc::new(Mutex::new(())),
        }
    }
}

impl SemanticIndex {
    pub fn new() -> Self {
        Self::default()
    }

    fn index_file_path() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".vortex-editor").join("index.json"))
    }

    fn hash_content(content: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    pub fn init_project(&self, project_path: &str) {
        {
            let mut write = self.inner.write().unwrap();
            write.current_project = Some(project_path.to_string());
        }
        self.load_from_disk();
        self.index_project_files(project_path);
    }

    pub fn index_project_files(&self, project_path: &str) {
        let p = Path::new(project_path);
        if !p.exists() || !p.is_dir() {
            return;
        }

        let mut files_to_index = Vec::new();
        for entry in walkdir::WalkDir::new(p)
            .into_iter()
            .filter_entry(|e| {
                if e.depth() == 0 {
                    return true;
                }
                let name = e.file_name().to_string_lossy();
                !name.starts_with('.') && name != "node_modules" && name != "target"
            })
            .flatten()
        {
            if entry.file_type().is_file() {
                let ext = entry
                    .path()
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                if ext == "tex" || ext == "bib" {
                    files_to_index.push(entry.path().to_string_lossy().to_string());
                }
            }
        }

        for file_path in files_to_index {
            if let Ok(content) = fs::read_to_string(&file_path) {
                self.update_file(&file_path, &content);
            }
        }
    }

    pub fn update_file(&self, file_path: &str, content: &str) {
        let hash = Self::hash_content(content);

        {
            let read = self.inner.read().unwrap();
            if read.file_hashes.get(file_path) == Some(&hash) {
                return;
            }
        }

        let ext = Path::new(file_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let mut write = self.inner.write().unwrap();
        write.file_hashes.insert(file_path.to_string(), hash);
        Self::remove_file_entries_inner(&mut write, file_path);

        if ext == "tex" {
            let parsed = parse_latex_file(file_path, content);
            for label in parsed.labels {
                write
                    .labels
                    .insert(format!("{}:{}", file_path, label.key), label);
            }
            if !parsed.citations.is_empty() {
                write
                    .citations
                    .insert(file_path.to_string(), parsed.citations);
            }
            if !parsed.refs.is_empty() {
                write.refs.insert(file_path.to_string(), parsed.refs);
            }
            if !parsed.sections.is_empty() {
                write.sections.insert(file_path.to_string(), parsed.sections);
            }
            if !parsed.todos.is_empty() {
                write.todos.insert(file_path.to_string(), parsed.todos);
            }
            let mut bibs_to_load = Vec::new();
            for bib in &parsed.bibliographies {
                if !write
                    .bibliographies
                    .iter()
                    .any(|b| b.file == bib.file && b.source_file == bib.source_file)
                {
                    write.bibliographies.push(bib.clone());
                }

                let parent = Path::new(file_path).parent().unwrap_or(Path::new(""));
                let raw_name = &bib.file;
                let candidates = [
                    parent.join(raw_name),
                    parent.join(format!("{}.bib", raw_name)),
                ];
                for cand in &candidates {
                    if cand.is_file() {
                        let cand_str = cand.to_string_lossy().to_string();
                        if !write.file_hashes.contains_key(&cand_str) {
                            if let Ok(bib_content) = fs::read_to_string(cand) {
                                bibs_to_load.push((cand_str, bib_content));
                            }
                        }
                        break;
                    }
                }
            }

            for (bib_path, bib_content) in bibs_to_load {
                let bib_hash = Self::hash_content(&bib_content);
                write.file_hashes.insert(bib_path.clone(), bib_hash);
                let entries = parse_bib_file(&bib_path, &bib_content);
                for entry in entries {
                    write.bibentries.insert(entry.key.clone(), entry);
                }
            }
        } else if ext == "bib" {
            let entries = parse_bib_file(file_path, content);
            for entry in entries {
                write.bibentries.insert(entry.key.clone(), entry);
            }
        }

        drop(write);
        self.save_to_disk();
    }

    pub fn remove_file(&self, file_path: &str) {
        let mut write = self.inner.write().unwrap();
        Self::remove_file_entries_inner(&mut write, file_path);
        write.file_hashes.remove(file_path);
        drop(write);
        self.save_to_disk();
    }

    fn remove_file_entries_inner(inner: &mut SemanticIndexInner, file_path: &str) {
        inner.labels.retain(|_, v| v.file != file_path);
        inner.bibentries.retain(|_, v| v.file != file_path);
        inner.citations.remove(file_path);
        inner.refs.remove(file_path);
        inner.sections.remove(file_path);
        inner.todos.remove(file_path);
        inner.bibliographies.retain(|b| b.source_file != file_path);
    }

    pub fn get_all_labels(&self) -> Vec<LabelItem> {
        let read = self.inner.read().unwrap();
        read.labels.values().cloned().collect()
    }

    pub fn get_all_citations(&self) -> Vec<BibEntryItem> {
        let read = self.inner.read().unwrap();
        read.bibentries.values().cloned().collect()
    }

    pub fn get_labels_for_file(&self, file_path: &str) -> Vec<LabelItem> {
        let read = self.inner.read().unwrap();
        read.labels
            .values()
            .filter(|l| l.file == file_path)
            .cloned()
            .collect()
    }

    pub fn get_labels_excluding_file(&self, file_path: &str) -> Vec<LabelItem> {
        let read = self.inner.read().unwrap();
        read.labels
            .values()
            .filter(|l| l.file != file_path)
            .cloned()
            .collect()
    }

    pub fn get_referenced_labels(&self, file_path: &str) -> HashSet<String> {
        let read = self.inner.read().unwrap();
        read.refs
            .get(file_path)
            .map(|refs| refs.iter().map(|r| r.key.clone()).collect())
            .unwrap_or_default()
    }

    pub fn get_cited_keys(&self, file_path: &str) -> HashSet<String> {
        let read = self.inner.read().unwrap();
        let mut keys = HashSet::new();
        if let Some(citations) = read.citations.get(file_path) {
            for cite in citations {
                for k in &cite.keys {
                    keys.insert(k.clone());
                }
            }
        }
        keys
    }

    pub fn get_sections_for_file(&self, file_path: &str) -> Vec<SectionItem> {
        let read = self.inner.read().unwrap();
        read.sections.get(file_path).cloned().unwrap_or_default()
    }

    pub fn get_all_sections(&self) -> Vec<SectionItem> {
        let read = self.inner.read().unwrap();
        let mut all = Vec::new();
        for list in read.sections.values() {
            all.extend(list.clone());
        }
        all.sort_by(|a, b| a.file.cmp(&b.file).then(a.line.cmp(&b.line)));
        all
    }

    pub fn get_all_todos(&self) -> Vec<TodoItem> {
        let read = self.inner.read().unwrap();
        let mut all = Vec::new();
        for list in read.todos.values() {
            all.extend(list.clone());
        }
        all.sort_by(|a, b| a.file.cmp(&b.file).then(a.line.cmp(&b.line)));
        all
    }

    pub fn get_todos_for_file(&self, file_path: &str) -> Vec<TodoItem> {
        let read = self.inner.read().unwrap();
        read.todos.get(file_path).cloned().unwrap_or_default()
    }

    pub fn get_stats(&self) -> IndexStats {
        let read = self.inner.read().unwrap();
        IndexStats {
            labels: read.labels.len(),
            bibentries: read.bibentries.len(),
            files_indexed: read.file_hashes.len(),
            project: read.current_project.clone(),
        }
    }

    pub fn clear(&self) {
        let mut write = self.inner.write().unwrap();
        write.labels.clear();
        write.bibentries.clear();
        write.citations.clear();
        write.refs.clear();
        write.sections.clear();
        write.todos.clear();
        write.bibliographies.clear();
        write.file_hashes.clear();
        write.current_project = None;
    }

    pub fn save_to_disk(&self) {
        let _guard = self.save_lock.lock().unwrap();
        let read = self.inner.read().unwrap();
        if read.current_project.is_none() {
            return;
        }

        if let Some(path) = Self::index_file_path() {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }

            let data = DiskIndexData {
                version: INDEX_VERSION,
                project: read.current_project.clone(),
                timestamp: chrono::Utc::now().timestamp_millis(),
                labels: read.labels.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                bibentries: read.bibentries.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                citations: read.citations.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                refs: read.refs.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                sections: read.sections.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                todos: read.todos.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                bibliographies: read.bibliographies.clone(),
                file_hashes: read.file_hashes.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            };

            if let Ok(json) = serde_json::to_string_pretty(&data) {
                let _ = fs::write(path, json);
            }
        }
    }

    pub fn load_from_disk(&self) {
        if let Some(path) = Self::index_file_path() {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(data) = serde_json::from_str::<DiskIndexData>(&content) {
                    let mut write = self.inner.write().unwrap();
                    if data.version == INDEX_VERSION && data.project == write.current_project {
                        write.labels = data.labels.into_iter().collect();
                        write.bibentries = data.bibentries.into_iter().collect();
                        write.citations = data.citations.into_iter().collect();
                        write.refs = data.refs.into_iter().collect();
                        write.sections = data.sections.into_iter().collect();
                        write.todos = data.todos.into_iter().collect();
                        write.bibliographies = data.bibliographies;
                        write.file_hashes = data.file_hashes.into_iter().collect();
                    }
                }
            }
        }
    }
}
