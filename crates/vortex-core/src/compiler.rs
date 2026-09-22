//! Builds a document with latexmk and reports diagnostics from its log.
//!
//! Auxiliary files are kept between builds so latexmk can skip passes that
//! aren't needed; `clean` removes them when a build gets stuck.

use crate::build_log::{self, Diagnostic};
use crate::project::{detect_engine_for, pdf_for, Engine};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Instant, SystemTime};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct BuildResult {
    pub success: bool,
    /// Short summary for the status bar.
    pub message: String,
    /// The main document that was compiled.
    pub main_file: String,
    pub engine: Engine,
    /// Set when this build wrote the PDF (also on failed builds that still produced one).
    pub pdf_path: Option<String>,
    pub diagnostics: Vec<Diagnostic>,
    /// The TeX `.log`, or latexmk's own output when TeX never ran.
    pub raw_log: String,
    pub duration_ms: u32,
}

pub fn get_latex_path_env() -> String {
    let current_path = std::env::var("PATH").unwrap_or_default();
    let extra_paths = [
        "/Library/TeX/texbin",
        "/usr/local/texlive/2026/bin/universal-darwin",
        "/usr/local/texlive/2025/bin/universal-darwin",
        "/usr/local/texlive/2024/bin/universal-darwin",
        "/usr/local/texlive/2023/bin/universal-darwin",
        "/opt/homebrew/bin",
        "/usr/local/bin",
        "/usr/bin",
        "/bin",
    ];

    let mut paths: Vec<&str> = extra_paths.iter().copied().collect();
    let current_entries: Vec<&str> = current_path.split(':').collect();
    for entry in current_entries {
        if !paths.contains(&entry) && !entry.is_empty() {
            paths.push(entry);
        }
    }
    paths.join(":")
}

/// latexmk with the TeX Live PATH and unwrapped log lines (so paths in the log stay whole).
fn latexmk(dir: &Path) -> Command {
    let mut cmd = Command::new("latexmk");
    cmd.env("PATH", get_latex_path_env())
        .env("max_print_line", "10000")
        .env("error_line", "254")
        .env("half_error_line", "238")
        .current_dir(dir)
        .stdin(Stdio::null());
    cmd
}

fn split(tex: &Path) -> Result<(&Path, &str), String> {
    let dir = match tex.parent() {
        Some(d) if d.as_os_str().is_empty() => Path::new("."),
        Some(d) => d,
        None => Path::new("."),
    };
    let name = tex.file_name().and_then(|n| n.to_str()).ok_or("Error: invalid file name")?;
    Ok((dir, name))
}

/// Compiles `tex_file_path` (the main document) with the engine it asks for.
pub fn build_latex_project(tex_file_path: &str) -> BuildResult {
    let started = Instant::now();
    let path = Path::new(tex_file_path);
    let engine = detect_engine_for(path);
    let failed = |message: String, raw_log: String| BuildResult {
        success: false,
        message,
        main_file: tex_file_path.to_string(),
        engine,
        pdf_path: None,
        diagnostics: Vec::new(),
        raw_log,
        duration_ms: started.elapsed().as_millis() as u32,
    };
    if !path.is_file() {
        return failed(format!("Error: file '{tex_file_path}' not found"), String::new());
    }
    let (dir, file_name) = match split(path) {
        Ok(v) => v,
        Err(e) => return failed(e, String::new()),
    };

    let build_start = SystemTime::now();
    let output = latexmk(dir)
        .args([engine.latexmk_flag(), "-interaction=nonstopmode", "-file-line-error", "-synctex=1"])
        .arg(file_name)
        .output();
    let output = match output {
        Ok(o) => o,
        Err(e) => return failed(format!("Error executing latexmk: {e}"), e.to_string()),
    };

    // When nothing changed since a failed run, latexmk exits with an error without rerunning
    // TeX; the existing log still describes these sources, so it is parsed either way.
    let log = std::fs::read(path.with_extension("log")).map(|b| String::from_utf8_lossy(&b).to_string()).ok();
    let diagnostics = log.as_deref().map(|log| build_log::parse(log, dir)).unwrap_or_default();
    let raw_log = log.unwrap_or_else(|| {
        format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr))
    });

    let pdf = pdf_for(path);
    let pdf_written = std::fs::metadata(&pdf)
        .and_then(|m| m.modified())
        .map(|t| t >= build_start)
        .unwrap_or(false);
    let pdf_name = pdf.file_name().and_then(|n| n.to_str()).unwrap_or("output.pdf").to_string();
    let success = output.status.success();
    let errors = diagnostics.iter().filter(|d| d.severity == build_log::Severity::Error).count();
    let message = if success {
        format!("✓ Build complete: {pdf_name}")
    } else if errors > 0 {
        format!("✗ Build failed: {errors} error{}", if errors == 1 { "" } else { "s" })
    } else {
        "✗ Build failed".to_string()
    };

    BuildResult {
        success,
        message,
        main_file: tex_file_path.to_string(),
        engine,
        pdf_path: pdf_written.then(|| pdf.to_string_lossy().to_string()),
        diagnostics,
        raw_log,
        duration_ms: started.elapsed().as_millis() as u32,
    }
}

/// Removes auxiliary files (`latexmk -c`); the PDF is kept so the viewer stays populated.
pub fn clean(tex_file_path: &str) -> Result<(), String> {
    let (dir, file_name) = split(Path::new(tex_file_path))?;
    let out = latexmk(dir)
        .arg("-c")
        .arg(file_name)
        .output()
        .map_err(|e| format!("Error executing latexmk: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}
