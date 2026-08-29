use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct PdfRenderCache {
    /// DPI used for rendering (affects cache key)
    pub dpi: u32,
    /// Paths to rendered page images in order (1-indexed page -> path)
    pub page_images: Vec<String>,
    /// Number of pages
    pub page_count: usize,
    /// Cache directory for this PDF+DPI combo
    pub cache_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct PdfPageInfo {
    pub page_count: usize,
    pub page_width_pts: f32,
    pub page_height_pts: f32,
}

pub fn get_pdf_cache_root() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".vortex-editor").join("pdf_cache"))
}

/// Deterministic 64-bit FNV-1a hash of the PDF path
pub fn hash_pdf_key(pdf_path: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in pdf_path.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", hash)
}

/// Get cache directory for a given PDF path and DPI
pub fn get_pdf_cache_dir(pdf_path: &str, dpi: u32) -> Option<PathBuf> {
    let root = get_pdf_cache_root()?;
    let pdf_key = hash_pdf_key(pdf_path);
    Some(root.join(pdf_key).join(format!("dpi_{}", dpi)))
}

/// Get page count using MuPDF in-process
pub fn get_pdf_page_count(pdf_path: &str) -> Option<usize> {
    let pdf_p = Path::new(pdf_path);
    if !pdf_p.exists() || !pdf_p.is_file() {
        return None;
    }

    let doc = mupdf::Document::open(pdf_path).ok()?;
    let count = doc.page_count().ok()?;
    if count > 0 {
        Some(count as usize)
    } else {
        None
    }
}

/// Get page dimensions in points (72 dpi) using MuPDF
pub fn get_pdf_page_dimensions(pdf_path: &str) -> Option<(f32, f32)> {
    let pdf_p = Path::new(pdf_path);
    if !pdf_p.exists() || !pdf_p.is_file() {
        return None;
    }

    let doc = mupdf::Document::open(pdf_path).ok()?;
    let page = doc.load_page(0).ok()?;
    let bounds = page.bounds().ok()?;
    let width = bounds.width();
    let height = bounds.height();
    if width > 0.0 && height > 0.0 {
        Some((width, height))
    } else {
        Some((595.276, 841.89))
    }
}

/// Ensure PDF is rendered to image cache using native MuPDF.
/// Returns list of image paths (ordered by page).
/// Uses caching: if cache images exist and are newer than PDF, they are reused.
pub fn ensure_pdf_rendered(pdf_path: &str, dpi: u32) -> Option<PdfRenderCache> {
    let pdf_p = Path::new(pdf_path);
    if !pdf_p.exists() || !pdf_p.is_file() {
        return None;
    }

    let page_count = get_pdf_page_count(pdf_path).unwrap_or(1);
    let cache_dir = get_pdf_cache_dir(pdf_path, dpi)?;

    if fs::create_dir_all(&cache_dir).is_err() {
        return None;
    }

    // Check if cache is valid: all page images exist, newer than PDF, non-empty
    let pdf_mtime = fs::metadata(pdf_p).and_then(|m| m.modified()).ok();
    let mut all_valid = true;
    let mut page_images = Vec::with_capacity(page_count);

    for page_num in 1..=page_count {
        let img_path = cache_dir.join(format!("page-{:04}.png", page_num));
        page_images.push(img_path.to_string_lossy().to_string());
    }

    for img_str in &page_images {
        let img_p = Path::new(img_str);
        if !img_p.exists() {
            all_valid = false;
            break;
        }
        if let Ok(meta) = fs::metadata(img_p) {
            if meta.len() == 0 {
                all_valid = false;
                break;
            }
            if let (Some(pdf_t), Ok(img_t)) = (pdf_mtime, meta.modified()) {
                if img_t < pdf_t {
                    all_valid = false;
                    break;
                }
            }
        } else {
            all_valid = false;
            break;
        }
    }

    if all_valid {
        return Some(PdfRenderCache {
            dpi,
            page_images,
            page_count,
            cache_dir,
        });
    }

    // Render pages using MuPDF
    render_pdf_pages(pdf_path, &cache_dir, dpi, page_count)?;

    // Verify rendered files
    let mut verified_images = Vec::with_capacity(page_count);
    for page_num in 1..=page_count {
        let img_path = cache_dir.join(format!("page-{:04}.png", page_num));
        if img_path.exists() {
            verified_images.push(img_path.to_string_lossy().to_string());
        } else {
            verified_images.push(String::new());
        }
    }

    if verified_images.first().map(|s| s.is_empty()).unwrap_or(true) {
        return None;
    }

    Some(PdfRenderCache {
        dpi,
        page_images: verified_images,
        page_count,
        cache_dir,
    })
}

fn render_pdf_pages(pdf_path: &str, cache_dir: &Path, dpi: u32, page_count: usize) -> Option<()> {
    let doc = mupdf::Document::open(pdf_path).ok()?;
    let scale = (dpi as f32) / 72.0;
    let matrix = mupdf::Matrix::new_scale(scale, scale);
    let colorspace = mupdf::Colorspace::device_rgb();

    // Clean old cache pngs if any
    if let Ok(entries) = fs::read_dir(cache_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().map(|e| e.eq_ignore_ascii_case("png")).unwrap_or(false) {
                let _ = fs::remove_file(p);
            }
        }
    }

    for page_idx in 0..page_count {
        let page_num = page_idx + 1;
        let img_path = cache_dir.join(format!("page-{:04}.png", page_num));
        let page = doc.load_page(page_idx as i32).ok()?;
        let pixmap = page.to_pixmap(&matrix, &colorspace, false, true).ok()?;
        let img_path_str = img_path.to_str()?;
        pixmap.save_as(img_path_str, mupdf::ImageFormat::PNG).ok()?;
    }

    Some(())
}

/// Clear all cached DPI renderings for a specific PDF
pub fn clear_pdf_cache(pdf_path: &str) {
    if let Some(root) = get_pdf_cache_root() {
        let pdf_key = hash_pdf_key(pdf_path);
        let pdf_dir = root.join(pdf_key);
        if pdf_dir.exists() {
            let _ = fs::remove_dir_all(pdf_dir);
        }
    }
}

/// Get cache status: whether pdf needs re-render
pub fn is_cache_valid(pdf_path: &str, dpi: u32) -> bool {
    let pdf_p = Path::new(pdf_path);
    if !pdf_p.exists() {
        return false;
    }
    let page_count = get_pdf_page_count(pdf_path).unwrap_or(1);
    let cache_dir = match get_pdf_cache_dir(pdf_path, dpi) {
        Some(p) => p,
        None => return false,
    };
    if !cache_dir.exists() {
        return false;
    }
    let pdf_mtime = fs::metadata(pdf_p).and_then(|m| m.modified()).ok();
    for n in 1..=page_count {
        let img_path = cache_dir.join(format!("page-{:04}.png", n));
        if !img_path.exists() {
            return false;
        }
        if let (Some(pdf_t), Ok(meta)) = (pdf_mtime, fs::metadata(&img_path)) {
            if let Ok(img_t) = meta.modified() {
                if img_t < pdf_t {
                    return false;
                }
            }
        }
        if let Ok(meta) = fs::metadata(&img_path) {
            if meta.len() == 0 {
                return false;
            }
        }
    }
    true
}
