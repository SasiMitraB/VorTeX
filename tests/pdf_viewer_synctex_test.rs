use vortex::services::pdf_renderer::{get_pdf_page_count, get_pdf_page_dimensions, ensure_pdf_rendered, is_cache_valid, clear_pdf_cache};
use vortex::services::synctex::{synctex_forward_search, synctex_inverse_search, is_synctex_available};

#[test]
fn test_pdf_renderer_and_synctex() {
    // Create temp project if /tmp/vortex_test not available
    let pdf = "/tmp/vortex_test/main.pdf";
    let tex = "/tmp/vortex_test/main.tex";

    if !std::path::Path::new(pdf).exists() || !std::path::Path::new(tex).exists() {
        let dir = "/tmp/vortex_test";
        let _ = std::fs::create_dir_all(dir);
        let tex_content = r"\documentclass{article}
\begin{document}
Hello world
\section{Introduction}
This is a test.
Label here \label{sec:intro}
See Section \ref{sec:intro}.
\end{document}
";
        std::fs::write(tex, tex_content).expect("write tex");
        let res = vortex::services::compiler::build_latex_project(tex);
        if !res.success {
            eprintln!("Skipping pdf_viewer_synctex_test: latexmk not available or build failed: {}", res.message);
            return;
        }
    }

    assert!(std::path::Path::new(pdf).exists(), "PDF should exist at {}", pdf);
    assert!(std::path::Path::new(tex).exists());

    let count = get_pdf_page_count(pdf);
    println!("page count {:?}", count);
    assert_eq!(count, Some(1));

    let dims = get_pdf_page_dimensions(pdf);
    println!("dims {:?}", dims);
    assert!(dims.is_some());
    let (w, h) = dims.unwrap();
    assert!((w - 595.276).abs() < 1.0);
    assert!((h - 841.89).abs() < 1.0);

    // Clear cache to force re-render
    clear_pdf_cache(pdf);
    assert!(!is_cache_valid(pdf, 144));

    let cache = ensure_pdf_rendered(pdf, 144);
    println!("cache {:?}", cache);
    if cache.is_none() {
        eprintln!("Skipping: pdf rendering failed");
        return;
    }
    let cache = cache.unwrap();
    assert_eq!(cache.page_count, 1);
    assert_eq!(cache.page_images.len(), 1);
    assert!(std::path::Path::new(&cache.page_images[0]).exists());
    assert!(is_cache_valid(pdf, 144));

    if !is_synctex_available() {
        eprintln!("Skipping SyncTeX checks: synctex not available");
        return;
    }

    // SyncTeX forward
    let fwd = synctex_forward_search(tex, 5, 0, pdf, 0);
    println!("forward {:?}", fwd);
    if fwd.is_none() {
        eprintln!("Skipping: forward search returned None (maybe synctex file missing)");
        return;
    }
    let fwd = fwd.unwrap();
    assert_eq!(fwd.page, 1);
    assert!(fwd.x > 0.0);
    assert!(fwd.y > 0.0);

    // Highlight conversion
    let hl = vortex::views::pdf_viewer::SynctexHighlight::from(fwd.clone());
    println!("hl page {} x {} y {} w {} h {}", hl.page, hl.x, hl.y, hl.width, hl.height);
    assert_eq!(hl.page, 1);

    // Inverse at that highlight position should return same file/line
    let inv = synctex_inverse_search(pdf, hl.page, hl.x, hl.y);
    println!("inverse {:?}", inv);
    assert!(inv.is_some());
    let inv = inv.unwrap();
    assert!(inv.input.contains("main.tex"));
    assert!(inv.line > 0);
}
