use std::fs;
use tempfile::tempdir;
use vortex_core::compiler::{build_latex_project, get_latex_path_env};

#[test]
fn test_latex_path_env_contains_standard_paths() {
    let path = get_latex_path_env();
    assert!(path.contains("/Library/TeX/texbin"));
    assert!(path.contains("/usr/local/bin"));
    assert!(path.contains("/opt/homebrew/bin"));
}

#[test]
fn test_build_nonexistent_file() {
    let result = build_latex_project("/path/to/nonexistent/file.tex");
    assert!(!result.success);
    assert!(result.message.contains("not found"));
}

#[test]
fn test_build_simple_tex_file() {
    let dir = tempdir().unwrap();
    let tex_path = dir.path().join("main.tex");
    let tex_content = r#"\documentclass{article}
\begin{document}
Hello VorTeX Build Test!
\end{document}
"#;
    fs::write(&tex_path, tex_content).unwrap();

    let result = build_latex_project(tex_path.to_str().unwrap());

    // If latexmk is installed in /Library/TeX/texbin, this will succeed and produce a PDF!
    if result.success {
        assert!(result.message.contains("✓ Build complete: main.pdf"));
        assert!(result.pdf_path.is_some());
        let pdf_p = result.pdf_path.unwrap();
        assert!(std::path::Path::new(&pdf_p).exists());
    } else {
        // If latexmk is not present or failed, it returns failure without panicking
        assert!(result.message.contains("✗ Build failed") || result.message.contains("Error"));
    }
}

fn latexmk_available() -> bool {
    std::process::Command::new("latexmk")
        .arg("-v")
        .env("PATH", get_latex_path_env())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn build_reports_diagnostics_duration_and_keeps_aux_files() {
    if !latexmk_available() {
        eprintln!("latexmk not installed; skipping");
        return;
    }
    let dir = tempdir().unwrap();
    let dir_path = dir.path().canonicalize().unwrap();
    let tex = dir_path.join("paper.tex");
    fs::write(&tex, "\\documentclass{article}\n\\begin{document}\nSee \\ref{nowhere}.\n\\undefinedmacro\n\\end{document}\n").unwrap();

    let result = build_latex_project(tex.to_str().unwrap());
    assert!(!result.success);
    assert_eq!(result.engine, vortex_core::project::Engine::Pdf);
    assert!(result.message.contains("1 error"), "{}", result.message);
    let error = result.diagnostics.iter().find(|d| d.severity == vortex_core::build_log::Severity::Error).unwrap();
    assert_eq!(error.file.as_deref(), Some(tex.to_str().unwrap()));
    assert_eq!(error.line, Some(4));
    assert!(result.raw_log.contains("Undefined control sequence"));
    // TeX still wrote a PDF despite the error.
    assert!(result.pdf_path.is_some());
    assert!(result.duration_ms > 0);

    // Rebuilding unchanged sources: latexmk doesn't rerun TeX, but the errors are still reported.
    let again = build_latex_project(tex.to_str().unwrap());
    assert!(!again.success);
    assert_eq!(again.diagnostics, result.diagnostics);
    assert!(again.pdf_path.is_none(), "no new PDF was written");

    // Aux files survive so the next build is incremental.
    fs::write(&tex, "\\documentclass{article}\n\\begin{document}\nFixed.\n\\end{document}\n").unwrap();
    let result = build_latex_project(tex.to_str().unwrap());
    assert!(result.success, "{}", result.raw_log);
    assert!(dir_path.join("paper.aux").exists());
    assert!(dir_path.join("paper.fdb_latexmk").exists());

    vortex_core::compiler::clean(tex.to_str().unwrap()).unwrap();
    assert!(!dir_path.join("paper.aux").exists());
    assert!(dir_path.join("paper.pdf").exists());
}

#[test]
fn build_uses_xelatex_for_fontspec_documents() {
    if !latexmk_available() {
        eprintln!("latexmk not installed; skipping");
        return;
    }
    let dir = tempdir().unwrap();
    let tex = dir.path().join("main.tex");
    fs::write(&tex, "\\documentclass{article}\n\\usepackage{fontspec}\n\\begin{document}\nUnicode: é ß\n\\end{document}\n").unwrap();
    let result = build_latex_project(tex.to_str().unwrap());
    assert_eq!(result.engine, vortex_core::project::Engine::Xe);
    assert!(result.success, "{}", result.raw_log);
    assert!(result.raw_log.contains("XeTeX"));
}
