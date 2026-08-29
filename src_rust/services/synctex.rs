use std::path::Path;
use std::process::Command;

use crate::services::compiler::get_latex_path_env;

/// Result of a SyncTeX forward search (editor -> PDF)
#[derive(Debug, Clone, PartialEq)]
pub struct SynctexForwardResult {
    /// 1-based page number
    pub page: usize,
    /// x coordinate in PDF points (72 dpi, from top-left)
    pub x: f32,
    /// y coordinate in PDF points (72 dpi, from top-left)
    pub y: f32,
    /// box origin h (often same as x, but more accurate for highlighting)
    pub h: f32,
    /// box origin v
    pub v: f32,
    /// width of the enclosing box in points
    pub width: f32,
    /// height of the enclosing box in points
    pub height: f32,
}

/// Result of a SyncTeX inverse search (PDF -> editor)
#[derive(Debug, Clone, PartialEq)]
pub struct SynctexInverseResult {
    /// Source file path as reported by SyncTeX (may be absolute)
    pub input: String,
    /// 1-based line number
    pub line: usize,
    /// 0-based column (-1 if unknown) - we normalize to 0 if -1
    pub column: usize,
}

fn parse_synctex_output(stdout: &str) -> Vec<std::collections::HashMap<String, String>> {
    let mut results = Vec::new();
    let mut current: Option<std::collections::HashMap<String, String>> = None;

    for line in stdout.lines() {
        let line = line.trim();
        if line == "SyncTeX result begin" {
            current = Some(std::collections::HashMap::new());
        } else if line == "SyncTeX result end" {
            if let Some(map) = current.take() {
                if !map.is_empty() {
                    results.push(map);
                }
            }
        } else if let Some(ref mut map) = current {
            if let Some(colon) = line.find(':') {
                let key = line[..colon].trim().to_string();
                let value = line[colon + 1..].trim().to_string();
                map.insert(key, value);
            }
        }
    }
    results
}

/// Forward search: given a source location, find the corresponding PDF position.
/// `tex_file` should be the exact path as understood by TeX (prefer absolute).
/// `line` and `column` are 1-based (column 0 means ignore column).
/// `pdf_path` must exist.
/// `page_hint` is optional currently displayed page (0 for unknown).
pub fn synctex_forward_search(
    tex_file: &str,
    line: usize,
    column: usize,
    pdf_path: &str,
    page_hint: usize,
) -> Option<SynctexForwardResult> {
    let pdf_p = Path::new(pdf_path);
    if !pdf_p.exists() {
        return None;
    }

    // Check that synctex file exists
    if find_synctex_file(pdf_path).is_none() {
        return None;
    }

    let path_env = get_latex_path_env();
    let input_arg = if page_hint > 0 {
        format!("{}:{}:{}:{}", line, column, page_hint, tex_file)
    } else {
        format!("{}:{}:{}", line, column, tex_file)
    };

    let output = Command::new("synctex")
        .env("PATH", &path_env)
        .args(["view", "-i", &input_arg, "-o", pdf_path])
        .output()
        .ok()?;

    if !output.status.success() {
        // synctex still outputs results on failure? Try parsing anyway
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("SyncTeX ERROR") {
            // Could be missing file, still try stdout
        }
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let results = parse_synctex_output(&stdout);
    if results.is_empty() {
        return None;
    }

    // Take first result (most accurate)
    let first = &results[0];
    let page = first.get("Page")?.parse::<usize>().ok()?;
    let x = first.get("x")?.parse::<f32>().ok()?;
    let y = first.get("y")?.parse::<f32>().ok()?;
    let h = first
        .get("h")
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(x);
    let v = first
        .get("v")
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(y);
    let width = first
        .get("W")
        .or_else(|| first.get("width"))
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(0.0);
    let height = first
        .get("H")
        .or_else(|| first.get("height"))
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(0.0);

    Some(SynctexForwardResult {
        page,
        x,
        y,
        h,
        v,
        width,
        height,
    })
}

/// Inverse search: given a PDF position (page, x, y in PDF points from top-left), find source location.
pub fn synctex_inverse_search(
    pdf_path: &str,
    page: usize,
    x: f32,
    y: f32,
) -> Option<SynctexInverseResult> {
    let pdf_p = Path::new(pdf_path);
    if !pdf_p.exists() {
        return None;
    }

    if find_synctex_file(pdf_path).is_none() {
        return None;
    }

    let path_env = get_latex_path_env();
    let o_arg = format!("{}:{}:{}:{}", page, x, y, pdf_path);

    let output = Command::new("synctex")
        .env("PATH", &path_env)
        .args(["edit", "-o", &o_arg])
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let results = parse_synctex_output(&stdout);
    if results.is_empty() {
        return None;
    }

    let first = &results[0];
    let input = first.get("Input")?.clone();
    let line = first.get("Line")?.parse::<usize>().ok()?;
    // Column may be -1 if unknown
    let column_raw = first
        .get("Column")
        .and_then(|v| v.parse::<isize>().ok())
        .unwrap_or(-1);
    let column = if column_raw < 0 { 0 } else { column_raw as usize };

    Some(SynctexInverseResult {
        input,
        line,
        column,
    })
}

/// Find the synctex file for a given pdf_path. Checks for .synctex and .synctex.gz alongside PDF.
pub fn find_synctex_file(pdf_path: &str) -> Option<String> {
    let pdf_p = Path::new(pdf_path);
    let parent = pdf_p.parent().unwrap_or(Path::new("."));
    let stem = pdf_p.file_stem()?.to_str()?;

    let gz_path = parent.join(format!("{}.synctex.gz", stem));
    if gz_path.exists() {
        return Some(gz_path.to_string_lossy().to_string());
    }
    let synctex_path = parent.join(format!("{}.synctex", stem));
    if synctex_path.exists() {
        return Some(synctex_path.to_string_lossy().to_string());
    }

    // Also check if synctex file is in parent dir of PDF? (when output dir custom)
    // Try pdf_path with .synctex.gz appended
    let alt_gz = format!("{}.synctex.gz", pdf_path);
    if Path::new(&alt_gz).exists() {
        return Some(alt_gz);
    }
    let alt = format!("{}.synctex", pdf_path);
    if Path::new(&alt).exists() {
        return Some(alt);
    }

    None
}

/// Check if synctex is available in PATH
pub fn is_synctex_available() -> bool {
    let path_env = get_latex_path_env();
    Command::new("synctex")
        .env("PATH", &path_env)
        .arg("help")
        .output()
        .map(|o| o.status.success() || !String::from_utf8_lossy(&o.stdout).is_empty())
        .unwrap_or(false)
}

/// Simple helper to convert PDF point coordinates (72 dpi, top-left origin) to image pixel coordinates
/// given render DPI and zoom factor, and known page size in points.
/// Returns (pixel_x, pixel_y) relative to page image top-left.
pub fn pdf_point_to_pixel(
    point_x: f32,
    point_y: f32,
    render_dpi: f32,
    zoom: f32,
) -> (f32, f32) {
    let scale = (render_dpi / 72.0) * zoom;
    (point_x * scale, point_y * scale)
}

/// Inverse: pixel to PDF point
pub fn pixel_to_pdf_point(
    pixel_x: f32,
    pixel_y: f32,
    render_dpi: f32,
    zoom: f32,
) -> (f32, f32) {
    let scale = (render_dpi / 72.0) * zoom;
    if scale == 0.0 {
        return (0.0, 0.0);
    }
    (pixel_x / scale, pixel_y / scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_synctex_output() {
        let sample = r#"This is SyncTeX command line utility, version 1.5
SyncTeX result begin
Output:/tmp/test.pdf
Page:1
x:133.768356
y:167.710464
h:133.768356
v:167.710464
W:343.711060
H:9.962625
SyncTeX result end
"#;
        let results = parse_synctex_output(sample);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].get("Page").unwrap(), "1");
        assert_eq!(results[0].get("x").unwrap(), "133.768356");
    }

    #[test]
    fn test_point_pixel_conversion() {
        let (px, py) = pdf_point_to_pixel(100.0, 200.0, 144.0, 1.0);
        assert!((px - 200.0).abs() < 0.01);
        assert!((py - 400.0).abs() < 0.01);

        let (x, y) = pixel_to_pdf_point(px, py, 144.0, 1.0);
        assert!((x - 100.0).abs() < 0.01);
        assert!((y - 200.0).abs() < 0.01);
    }

    #[test]
    fn test_zoom_conversion() {
        let (px, _) = pdf_point_to_pixel(100.0, 100.0, 144.0, 1.5);
        assert!((px - 300.0).abs() < 0.01);
    }
}
