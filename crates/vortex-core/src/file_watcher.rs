use notify_debouncer_mini::{new_debouncer, DebouncedEventKind, Debouncer};
use notify::{RecommendedWatcher, RecursiveMode};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum WatchedKind {
    /// `.tex` or `.bib`: indexed.
    Source,
    /// A build output; the PDF viewer reloads it.
    Pdf,
    /// `.git/HEAD` or `.git/index`: a commit, checkout or stage happened.
    Git,
}

impl WatchedKind {
    /// What the watcher reports for `path`, or `None` for files it ignores.
    pub fn of(path: &Path) -> Option<Self> {
        let s = path.to_string_lossy();
        if s.ends_with("/.git/HEAD") || s.ends_with("/.git/index") {
            return Some(WatchedKind::Git);
        }
        if s.contains("/.git/") || s.contains("/target/") || s.contains("/node_modules/") {
            return None;
        }
        match path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref() {
            Some("tex") | Some("bib") => Some(WatchedKind::Source),
            Some("pdf") => Some(WatchedKind::Pdf),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct FileChangeEvent {
    #[serde(rename = "type")]
    pub change_type: String, // "changed" | "deleted"
    pub kind: WatchedKind,
    pub path: String,
    #[serde(rename = "relativePath")]
    pub relative_path: Option<String>,
}

pub struct FileWatcher {
    _debouncer: Debouncer<RecommendedWatcher>,
    project_path: PathBuf,
}

impl FileWatcher {
    pub fn new<F>(project_path: &str, on_event: F) -> Result<Self, anyhow::Error>
    where
        F: Fn(FileChangeEvent) + Send + Sync + 'static,
    {
        let (tx, rx) = mpsc::channel();
        let proj_buf = PathBuf::from(project_path);
        let proj_buf_clone = proj_buf.clone();

        // 300ms debounce
        let mut debouncer = new_debouncer(Duration::from_millis(300), tx)?;

        debouncer
            .watcher()
            .watch(Path::new(project_path), RecursiveMode::Recursive)?;

        std::thread::spawn(move || {
            while let Ok(events_res) = rx.recv() {
                if let Ok(events) = events_res {
                    for event in events {
                        let path = event.path;
                        let path_str = path.to_string_lossy().to_string();
                        let Some(kind) = WatchedKind::of(&path) else {
                            continue;
                        };

                        let relative_path = path
                            .strip_prefix(&proj_buf_clone)
                            .ok()
                            .map(|p| p.to_string_lossy().to_string());

                        let change_type = if !path.exists() {
                            "deleted"
                        } else {
                            match event.kind {
                                DebouncedEventKind::Any => "changed",
                                _ => "changed",
                            }
                        };

                        on_event(FileChangeEvent {
                            change_type: change_type.to_string(),
                            kind,
                            path: path_str,
                            relative_path,
                        });
                    }
                }
            }
        });

        Ok(Self {
            _debouncer: debouncer,
            project_path: proj_buf,
        })
    }

    pub fn project_path(&self) -> &Path {
        &self.project_path
    }
}
