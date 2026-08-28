use notify_debouncer_mini::{new_debouncer, DebouncedEventKind, Debouncer};
use notify::{RecommendedWatcher, RecursiveMode};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChangeEvent {
    #[serde(rename = "type")]
    pub change_type: String, // "added" | "changed" | "deleted"
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

                        // Filter by extension: .tex or .bib
                        let ext = path
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("")
                            .to_lowercase();

                        if ext != "tex" && ext != "bib" {
                            continue;
                        }

                        // Filter out build artifacts and hidden directories
                        if path_str.contains("/.git/")
                            || path_str.contains("/target/")
                            || path_str.contains("/node_modules/")
                            || path_str.ends_with(".aux")
                            || path_str.ends_with(".log")
                            || path_str.ends_with(".synctex.gz")
                        {
                            continue;
                        }

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
