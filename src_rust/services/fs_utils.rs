use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeNode {
    pub name: String,
    pub path: String,
    #[serde(rename = "type")]
    pub node_type: String, // "file" | "folder"
    #[serde(default)]
    pub children: Option<Vec<TreeNode>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectItem {
    pub name: String,
    pub path: String,
    #[serde(rename = "previewPdf")]
    pub preview_pdf: Option<String>,
    #[serde(rename = "previewImage", default)]
    pub preview_image: Option<String>,
}

pub fn get_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".vortex-editor").join("config.json"))
}

pub fn read_config() -> serde_json::Value {
    if let Some(config_path) = get_config_path() {
        if let Ok(data) = fs::read_to_string(config_path) {
            if let Ok(json) = serde_json::from_str(&data) {
                return json;
            }
        }
    }
    serde_json::json!({})
}

pub fn write_config(config: &serde_json::Value) -> Result<(), std::io::Error> {
    if let Some(config_path) = get_config_path() {
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(config)?;
        fs::write(config_path, data)?;
    }
    Ok(())
}

pub fn get_config_val(key: &str) -> Option<serde_json::Value> {
    let cfg = read_config();
    cfg.get(key).cloned()
}

pub fn set_config_val(key: &str, val: serde_json::Value) -> Result<(), std::io::Error> {
    let mut cfg = read_config();
    if let Some(obj) = cfg.as_object_mut() {
        obj.insert(key.to_string(), val);
    } else {
        let mut map = serde_json::Map::new();
        map.insert(key.to_string(), val);
        cfg = serde_json::Value::Object(map);
    }
    write_config(&cfg)
}

pub fn build_tree(dir_path: &str) -> Vec<TreeNode> {
    let path = Path::new(dir_path);
    if !path.exists() || !path.is_dir() {
        return Vec::new();
    }

    let mut nodes = Vec::new();

    if let Ok(entries) = fs::read_dir(path) {
        for entry_res in entries.flatten() {
            let file_name = entry_res.file_name().to_string_lossy().to_string();

            // Skip hidden files, node_modules, .git
            if file_name.starts_with('.') || file_name == "node_modules" || file_name == "target" {
                continue;
            }

            let full_path = entry_res.path().to_string_lossy().to_string();
            let is_dir = entry_res.file_type().map(|ft| ft.is_dir()).unwrap_or(false);

            if is_dir {
                let children = build_tree(&full_path);
                nodes.push(TreeNode {
                    name: file_name,
                    path: full_path,
                    node_type: "folder".to_string(),
                    children: Some(children),
                });
            } else {
                nodes.push(TreeNode {
                    name: file_name,
                    path: full_path,
                    node_type: "file".to_string(),
                    children: None,
                });
            }
        }
    }

    // Sort folders before files, then alphabetical
    nodes.sort_by(|a, b| {
        if a.node_type != b.node_type {
            if a.node_type == "folder" {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            }
        } else {
            a.name.to_lowercase().cmp(&b.name.to_lowercase())
        }
    });

    nodes
}

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::process::Command;

pub fn find_project_pdf(project_dir: &Path) -> Option<String> {
    if !project_dir.exists() || !project_dir.is_dir() {
        return None;
    }

    // 1. Direct check for main.pdf in root
    let root_main = project_dir.join("main.pdf");
    if root_main.is_file() {
        return Some(root_main.to_string_lossy().to_string());
    }

    // 2. Direct check in common build folders: build/main.pdf, out/main.pdf, output/main.pdf, dist/main.pdf
    for sub in &["build", "out", "output", "dist", "target", "auxil", "export"] {
        let sub_main = project_dir.join(sub).join("main.pdf");
        if sub_main.is_file() {
            return Some(sub_main.to_string_lossy().to_string());
        }
    }

    // 3. Any .pdf in root directory
    if let Ok(entries) = fs::read_dir(project_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() && p.extension().map(|e| e.eq_ignore_ascii_case("pdf")).unwrap_or(false) {
                return Some(p.to_string_lossy().to_string());
            }
        }
    }

    // 4. Any .pdf in common build subfolders
    for sub in &["build", "out", "output", "dist", "target"] {
        let sub_dir = project_dir.join(sub);
        if sub_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(&sub_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_file() && p.extension().map(|e| e.eq_ignore_ascii_case("pdf")).unwrap_or(false) {
                        return Some(p.to_string_lossy().to_string());
                    }
                }
            }
        }
    }

    None
}

pub fn get_or_create_pdf_thumbnail(pdf_path: &str) -> Option<String> {
    let pdf_p = Path::new(pdf_path);
    if !pdf_p.exists() || !pdf_p.is_file() {
        return None;
    }

    let cache_dir = dirs::home_dir()?.join(".vortex-editor").join("thumbnails");
    if fs::create_dir_all(&cache_dir).is_err() {
        return None;
    }

    let mut hasher = DefaultHasher::new();
    pdf_path.hash(&mut hasher);
    let path_hash = hasher.finish();

    let thumb_file_name = format!("{:016x}.png", path_hash);
    let thumb_path = cache_dir.join(&thumb_file_name);

    // Check if valid cached thumbnail exists and is newer than the PDF
    if thumb_path.exists() {
        if let (Ok(pdf_meta), Ok(thumb_meta)) = (fs::metadata(pdf_p), fs::metadata(&thumb_path)) {
            if let (Ok(pdf_mtime), Ok(thumb_mtime)) = (pdf_meta.modified(), thumb_meta.modified()) {
                if thumb_mtime >= pdf_mtime && thumb_meta.len() > 0 {
                    return Some(thumb_path.to_string_lossy().to_string());
                }
            }
        }
    }

    let temp_subfolder = cache_dir.join(format!("tmp_{:016x}", path_hash));
    let _ = fs::create_dir_all(&temp_subfolder);

    let mut generated = false;

    // Attempt 1: qlmanage (macOS QuickLook - fast native thumbnail generator)
    if cfg!(target_os = "macos") || Path::new("/usr/bin/qlmanage").exists() {
        if let Ok(output) = Command::new("qlmanage")
            .args(["-t", "-s", "600", "-o"])
            .arg(&temp_subfolder)
            .arg(pdf_path)
            .output()
        {
            if output.status.success() {
                if let Ok(entries) = fs::read_dir(&temp_subfolder) {
                    for entry in entries.flatten() {
                        let ep = entry.path();
                        if ep.extension().map(|e| e.eq_ignore_ascii_case("png")).unwrap_or(false) {
                            if let Ok(meta) = fs::metadata(&ep) {
                                if meta.len() > 0 {
                                    if fs::rename(&ep, &thumb_path).is_ok() || fs::copy(&ep, &thumb_path).is_ok() {
                                        generated = true;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Attempt 2: pdftoppm (poppler utility - cross-platform fallback)
    if !generated {
        let ppm_prefix = temp_subfolder.join("page");
        if let Ok(output) = Command::new("pdftoppm")
            .args(["-png", "-f", "1", "-l", "1", "-scale-to", "600"])
            .arg(pdf_path)
            .arg(&ppm_prefix)
            .output()
        {
            if output.status.success() {
                if let Ok(entries) = fs::read_dir(&temp_subfolder) {
                    for entry in entries.flatten() {
                        let ep = entry.path();
                        if ep.extension().map(|e| e.eq_ignore_ascii_case("png")).unwrap_or(false) {
                            if let Ok(meta) = fs::metadata(&ep) {
                                if meta.len() > 0 {
                                    if fs::rename(&ep, &thumb_path).is_ok() || fs::copy(&ep, &thumb_path).is_ok() {
                                        generated = true;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Cleanup temporary subfolder
    let _ = fs::remove_dir_all(&temp_subfolder);

    if generated && thumb_path.exists() {
        Some(thumb_path.to_string_lossy().to_string())
    } else {
        None
    }
}

pub fn scan_projects(projects_folder: &str) -> Vec<ProjectItem> {
    let base_path = Path::new(projects_folder);
    if !base_path.exists() || !base_path.is_dir() {
        return Vec::new();
    }

    let mut projects = Vec::new();

    if let Ok(entries) = fs::read_dir(base_path) {
        for entry in entries.flatten() {
            let file_name = entry.file_name().to_string_lossy().to_string();
            if file_name.starts_with('.') {
                continue;
            }

            let p_path = entry.path();
            if p_path.is_dir() {
                let project_path_str = p_path.to_string_lossy().to_string();
                let preview_pdf = find_project_pdf(&p_path);
                let preview_image = preview_pdf.as_deref().and_then(get_or_create_pdf_thumbnail);

                projects.push(ProjectItem {
                    name: file_name,
                    path: project_path_str,
                    preview_pdf,
                    preview_image,
                });
            }
        }
    }

    projects.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    projects
}
