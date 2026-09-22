use std::fs;
use tempfile::tempdir;
use vortex_core::fs_utils::{find_project_pdf, get_or_create_pdf_thumbnail, scan_projects};

#[test]
fn test_find_project_pdf_root_and_subdirectories() {
    let dir = tempdir().unwrap();
    let proj_dir = dir.path().join("my_paper");
    fs::create_dir(&proj_dir).unwrap();

    // Initially no PDF
    assert_eq!(find_project_pdf(&proj_dir), None);

    // Add build/main.pdf
    let build_dir = proj_dir.join("build");
    fs::create_dir(&build_dir).unwrap();
    let pdf_path = build_dir.join("main.pdf");
    fs::write(&pdf_path, b"%PDF-1.4 dummy pdf content").unwrap();

    let found = find_project_pdf(&proj_dir);
    assert_eq!(found, Some(pdf_path.to_string_lossy().to_string()));

    // Root main.pdf should take priority
    let root_pdf = proj_dir.join("main.pdf");
    fs::write(&root_pdf, b"%PDF-1.4 root main pdf").unwrap();
    let found_root = find_project_pdf(&proj_dir);
    assert_eq!(found_root, Some(root_pdf.to_string_lossy().to_string()));
}

#[test]
fn test_scan_projects_finds_project_items() {
    let dir = tempdir().unwrap();
    let proj_a = dir.path().join("ProjectA");
    let proj_b = dir.path().join("ProjectB");
    fs::create_dir(&proj_a).unwrap();
    fs::create_dir(&proj_b).unwrap();

    fs::write(proj_a.join("document.pdf"), b"%PDF-1.4").unwrap();

    let projects = scan_projects(dir.path().to_str().unwrap());
    assert_eq!(projects.len(), 2);
    assert_eq!(projects[0].name, "ProjectA");
    assert!(projects[0].preview_pdf.is_some());
    assert_eq!(projects[1].name, "ProjectB");
    assert!(projects[1].preview_pdf.is_none());
}

#[test]
fn test_get_or_create_pdf_thumbnail_nonexistent() {
    assert_eq!(get_or_create_pdf_thumbnail("/path/to/nonexistent/file.pdf"), None);
}

#[test]
fn tree_hides_latex_build_artifacts() {
    let dir = tempdir().unwrap();
    for f in ["main.tex", "main.pdf", "main.aux", "main.fdb_latexmk", "main.synctex.gz", "main.log", "refs.bib", "fig.png"] {
        std::fs::write(dir.path().join(f), "").unwrap();
    }
    let names: Vec<String> = vortex_core::fs_utils::build_tree(dir.path().to_str().unwrap()).into_iter().map(|n| n.name).collect();
    assert_eq!(names, ["fig.png", "main.pdf", "main.tex", "refs.bib"]);
}
