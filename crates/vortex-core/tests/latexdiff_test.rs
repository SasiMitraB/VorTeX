use std::path::Path;
use std::process::Command;
use vortex_core::git;
use vortex_core::latexdiff::*;
use vortex_core::project::{detect_engine, main_document as find_main_document, Engine};

#[test]
fn output_name_uses_short_hashes() {
    assert_eq!(output_name("paper/main.tex", "1234567890abcdef", None), "main-diff-1234567-working.pdf");
    assert_eq!(output_name("main.tex", "aaaaaaaaaa", Some("bbbbbbbbbb")), "main-diff-aaaaaaa-bbbbbbb.pdf");
}

#[test]
fn engine_detection() {
    assert_eq!(detect_engine("\\documentclass{article}\n\\usepackage{amsmath}"), Engine::Pdf);
    assert_eq!(detect_engine("\\usepackage{fontspec}"), Engine::Xe);
    assert_eq!(detect_engine("% \\usepackage{fontspec}\n"), Engine::Pdf);
    assert_eq!(detect_engine("% !TEX program = lualatex\n\\usepackage{fontspec}"), Engine::Lua);
    assert_eq!(detect_engine("% !TeX TS-program = xelatex\n"), Engine::Xe);
}

#[test]
fn main_document_resolution() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    std::fs::create_dir(p.join("chapters")).unwrap();
    std::fs::write(p.join("paper.tex"), "\\documentclass{article}\n\\begin{document}\\input{chapters/one}\\end{document}\n").unwrap();
    std::fs::write(p.join("chapters/one.tex"), "%!TEX root = ../paper.tex\nHello\n").unwrap();
    std::fs::write(p.join("chapters/two.tex"), "Just a fragment\n").unwrap();

    let canon = |x: Option<std::path::PathBuf>| x.map(|x| x.canonicalize().unwrap());
    let paper = Some(p.join("paper.tex").canonicalize().unwrap());
    // Magic root comment
    assert_eq!(canon(find_main_document(Some(&p.join("chapters/one.tex")), p)), paper);
    // Fragment without a root: fall back to the root-level document
    assert_eq!(canon(find_main_document(Some(&p.join("chapters/two.tex")), p)), paper);
    assert_eq!(canon(find_main_document(None, p)), paper);
    // A main.tex wins over other root files
    std::fs::write(p.join("main.tex"), "\\documentclass{article}\n").unwrap();
    assert_eq!(canon(find_main_document(None, p)), Some(p.join("main.tex").canonicalize().unwrap()));
}

#[test]
fn extracts_latex_errors_with_line_context() {
    let log = "(./x.tex\n! Undefined control sequence.\n<recently read> \\foo\n                  \nl.12 \\foo\n\n! Undefined control sequence.\n<recently read> \\foo\nl.12 \\foo\n";
    assert_eq!(log_errors(log, 3), vec!["! Undefined control sequence. — l.12 \\foo".to_string()]);
}

fn run(dir: &Path, args: &[&str]) {
    let ok = Command::new("git").args(args).current_dir(dir).output().unwrap().status.success();
    assert!(ok, "git {args:?} failed");
}

fn tool_available(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .env("PATH", vortex_core::compiler::get_latex_path_env())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn generates_diff_pdf_between_commit_and_working_copy() {
    if !tool_available("latexdiff") || !tool_available("latexmk") {
        eprintln!("latexdiff/latexmk not installed; skipping");
        return;
    }
    let repo = tempfile::tempdir().unwrap();
    let root = repo.path().canonicalize().unwrap();
    let project = root.join("paper");
    std::fs::create_dir_all(project.join("sections")).unwrap();
    std::fs::write(
        project.join("main.tex"),
        "\\documentclass{article}\n\\begin{document}\n\\input{sections/intro}\n\\end{document}\n",
    )
    .unwrap();
    std::fs::write(project.join("sections/intro.tex"), "The quick brown fox.\n").unwrap();
    run(&root, &["init", "-q"]);
    run(&root, &["-c", "user.name=T", "-c", "user.email=t@t", "add", "."]);
    run(&root, &["-c", "user.name=T", "-c", "user.email=t@t", "commit", "-qm", "first"]);

    // Uncommitted edit in an \input file
    std::fs::write(project.join("sections/intro.tex"), "The quick red fox jumps.\n").unwrap();

    let commits = git::log(&root, "paper", 10);
    assert_eq!(commits.len(), 1);
    let out = project.join(output_name("paper/main.tex", &commits[0].hash, None));
    let req = LatexDiffRequest {
        repo_root: root.clone(),
        scope: "paper".to_string(),
        main_rel: "paper/main.tex".to_string(),
        old_rev: commits[0].hash.clone(),
        new_rev: None,
        output_pdf: out.clone(),
    };
    let pdf = generate(&req).expect("diff PDF");
    assert_eq!(pdf, out);
    let bytes = std::fs::read(&pdf).unwrap();
    assert!(bytes.starts_with(b"%PDF"));

    // Only the PDF lands in the project: no intermediate .tex or aux files.
    let status = Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=all"])
        .current_dir(&root)
        .output()
        .unwrap();
    let status = String::from_utf8_lossy(&status.stdout);
    let untracked: Vec<&str> = status.lines().filter(|l| l.starts_with("??")).collect();
    assert_eq!(untracked, vec![format!("?? paper/{}", out.file_name().unwrap().to_string_lossy()).as_str()]);

    // Missing main file in the old version is reported, not a panic.
    let bad = LatexDiffRequest { main_rel: "paper/nope.tex".to_string(), ..req };
    assert!(generate(&bad).unwrap_err().contains("does not exist"));
}
