use std::fs;
use tempfile::tempdir;
use vortex::services::compiler::{build_latex_project, get_latex_path_env};

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
