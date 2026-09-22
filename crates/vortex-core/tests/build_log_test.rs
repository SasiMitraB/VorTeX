//! Fixtures are real pdfTeX logs (TeX Live 2025, `max_print_line=10000`) from
//! building small documents in a folder with a `chapters/` subfolder.

use std::path::Path;
use vortex_core::build_log::{parse, Diagnostic, Severity};

const DIR: &str = "/proj";

fn fixture(name: &str) -> Vec<Diagnostic> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/logs").join(name);
    parse(&std::fs::read_to_string(path).unwrap(), Path::new(DIR))
}

fn d(severity: Severity, file: &str, line: Option<u32>, message: &str) -> Diagnostic {
    Diagnostic { severity, file: Some(format!("{DIR}/{file}")), line, message: message.to_string() }
}

#[test]
fn undefined_control_sequences_in_main_and_included_files() {
    assert_eq!(
        fixture("undefined.log"),
        vec![
            d(Severity::Error, "chapters/intro.tex", Some(2), "Undefined control sequence."),
            d(Severity::Error, "main.tex", Some(5), "Undefined control sequence."),
        ]
    );
}

#[test]
fn missing_package_is_located_by_the_emergency_stop() {
    assert_eq!(
        fixture("missing.log"),
        vec![d(Severity::Error, "main.tex", Some(3), "File `doesnotexist.sty' not found.")]
    );
}

#[test]
fn missing_input_file() {
    assert_eq!(
        fixture("missing2.log"),
        vec![d(Severity::Error, "main.tex", Some(4), "File `chapters/nope.tex' not found.")]
    );
}

#[test]
fn bad_boxes_with_their_lines() {
    let diags = fixture("boxes.log");
    assert!(diags.iter().all(|x| x.severity == Severity::BadBox), "{diags:#?}");
    assert_eq!(diags[0], d(Severity::BadBox, "main.tex", Some(3), "Overfull \\hbox (156.1588pt too wide)"));
    assert_eq!(diags[1], d(Severity::BadBox, "main.tex", Some(5), "Overfull \\hbox (84.92818pt too wide)"));
    assert!(diags.contains(&d(Severity::BadBox, "main.tex", Some(5), "Underfull \\hbox (badness 10000)")));
    // The box inside the \input file is attributed to it.
    assert!(diags.contains(&d(Severity::BadBox, "chapters/c.tex", Some(2), "Underfull \\hbox (badness 10000)")));
}

#[test]
fn undefined_references_and_citations() {
    let diags = fixture("refs.log");
    let w = |file, line, msg| d(Severity::Warning, file, line, msg);
    assert!(diags.contains(&w("main.tex", Some(4), "Reference `sec:missing' on page 1 undefined.")), "{diags:#?}");
    assert!(diags.contains(&w("main.tex", Some(4), "Citation `nokey' on page 1 undefined.")));
    assert!(diags.contains(&w("chapters/r.tex", Some(1), "Citation `other' on page 1 undefined.")));
    assert!(diags.contains(&d(Severity::Error, "chapters/r.tex", Some(1), "Undefined control sequence.")));
    // Summary warnings after the \input closed belong to main.tex and have no line.
    assert!(diags.contains(&w("main.tex", None, "There were undefined references.")));
    assert!(diags.contains(&w("main.tex", None, "Label(s) may have changed. Rerun to get cross-references right.")));
}

#[test]
fn classic_error_format_uses_the_file_stack_and_context_line() {
    let log = "(./main.tex (./chapters/a.tex\n! Undefined control sequence.\nl.7 \\foo (unbalanced\n\n)\n! Missing $ inserted.\n<inserted text>\n$\nl.9 x^2\n";
    assert_eq!(
        parse(log, Path::new(DIR)),
        vec![
            d(Severity::Error, "chapters/a.tex", Some(7), "Undefined control sequence. \\foo (unbalanced"),
            // The "(unbalanced" in echoed source did not open a file frame.
            d(Severity::Error, "main.tex", Some(9), "Missing $ inserted."),
        ]
    );
}

#[test]
fn multi_line_package_warnings_are_joined() {
    let log = "(./main.tex\nPackage hyperref Warning: Token not allowed in a PDF string (Unicode):\n(hyperref)                removing `\\alpha' on input line 12.\n\n)";
    assert_eq!(
        parse(log, Path::new(DIR)),
        vec![d(Severity::Warning, "main.tex", Some(12), "Token not allowed in a PDF string (Unicode): removing `\\alpha'.")]
    );
}

#[test]
fn paths_with_spaces_resolve_against_the_disk() {
    let dir = tempfile::tempdir().unwrap();
    let chapter_dir = dir.path().join("My Chapters");
    std::fs::create_dir(&chapter_dir).unwrap();
    std::fs::write(chapter_dir.join("one.tex"), "").unwrap();
    let log = "(./main.tex (./My Chapters/one.tex\n./My Chapters/one.tex:3: Undefined control sequence.\n))";
    let diags = parse(log, dir.path());
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].file.as_deref(), Some(chapter_dir.join("one.tex").to_str().unwrap()));
    // Warnings (no file:line prefix) rely on the stack getting the full path.
    let log = "(./main.tex (./My Chapters/one.tex\nLaTeX Warning: Reference `x' on page 1 undefined on input line 2.\n))";
    assert_eq!(parse(log, dir.path())[0].file.as_deref(), Some(chapter_dir.join("one.tex").to_str().unwrap()));
}
