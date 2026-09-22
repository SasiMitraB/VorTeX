use std::fs;
use tempfile::tempdir;
use vortex_core::backend::BackendClient;
use vortex_core::completion::{complete, context_at, CompletionContext, CompletionKind};

fn ctx(prefix: &str) -> Option<CompletionContext> {
    context_at(prefix)
}

#[test]
fn detects_what_is_being_completed() {
    assert_eq!(ctx("See \\ref{sec:in"), Some(CompletionContext::Reference { query: "sec:in".into() }));
    assert_eq!(ctx("\\cref{a, b"), Some(CompletionContext::Reference { query: "b".into() }));
    assert_eq!(ctx("\\citep[see][p.~10]{knu"), Some(CompletionContext::Citation { query: "knu".into() }));
    assert_eq!(ctx("\\cite*{"), Some(CompletionContext::Citation { query: "".into() }));
    assert_eq!(ctx("  \\begin{fig"), Some(CompletionContext::Environment { query: "fig".into() }));
    assert_eq!(ctx("\\subs"), Some(CompletionContext::Command { query: "subs".into() }));
    assert_eq!(ctx("\\ref{done} more"), None);
    assert_eq!(ctx("plain text"), None);
    assert_eq!(ctx("\\"), None);
}

fn backend_with_project() -> (tempfile::TempDir, BackendClient) {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("main.tex"),
        "\\section{Intro}\\label{sec:intro}\n\\begin{equation}\\label{eq:euler}\\end{equation}\n\\bibliography{refs}\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("refs.bib"),
        "@book{knuth1984, title={The TeXbook}, author={Donald Knuth and Someone Else}, year={1984}}\n",
    )
    .unwrap();
    let backend = BackendClient::new().unwrap();
    backend.init_project(dir.path().to_str().unwrap()).unwrap();
    (dir, backend)
}

#[test]
fn reference_completion_replaces_the_key_and_keeps_one_brace() {
    let (_dir, backend) = backend_with_project();

    // Cursor after "sec:" with an auto-closed brace: the brace is consumed and re-added.
    let line = "See \\ref{sec:}.";
    let c = complete(&backend, line, 13, None).expect("completions");
    assert_eq!((c.from, c.to), (9, 14));
    let item = c.items.iter().find(|i| i.label == "sec:intro").expect("label suggested");
    assert_eq!(item.kind, CompletionKind::Reference);
    assert_eq!(item.insert_text, "sec:intro}");
    assert_eq!(item.cursor_offset, 10);

    // Mid-key: the rest of the key is replaced too, and no brace is added before a comma.
    let line = "\\cref{se,eq:euler}";
    let c = complete(&backend, line, 8, None).unwrap();
    assert_eq!((c.from, c.to), (6, 8));
    assert!(c.items.iter().all(|i| !i.insert_text.ends_with('}')));

    // No closing brace yet: one is added.
    let c = complete(&backend, "\\ref{eq", 7, None).unwrap();
    assert!(c.items.iter().any(|i| i.insert_text == "eq:euler}"));
}

#[test]
fn citation_completion_describes_entries() {
    let (_dir, backend) = backend_with_project();
    let c = complete(&backend, "\\cite{knu", 9, None).unwrap();
    let item = &c.items[0];
    assert_eq!(item.label, "knuth1984");
    assert_eq!(item.detail.as_deref(), Some("Donald Knuth et al., 1984"));
    assert_eq!(item.documentation.as_deref(), Some("The TeXbook (1984)"));
}

#[test]
fn environment_completion_inserts_an_indented_block() {
    let backend = BackendClient::new().unwrap();
    let c = complete(&backend, "  \\begin{fig}", 12, None).unwrap();
    assert_eq!((c.from, c.to), (9, 13));
    let item = &c.items[0];
    assert_eq!(item.label, "figure");
    assert_eq!(item.insert_text, "figure}\n    \n  \\end{figure}");
    // Cursor at the end of the indented blank line.
    assert_eq!(item.cursor_offset, "figure}\n    ".len() as u32);
}

#[test]
fn command_completion_puts_cursor_in_first_argument() {
    let backend = BackendClient::new().unwrap();
    let offset = |line: &str, label: &str| {
        let c = complete(&backend, line, line.encode_utf16().count() as u32, None).unwrap();
        let i = c.items.into_iter().find(|i| i.label == label).unwrap();
        (i.insert_text.clone(), i.cursor_offset)
    };
    assert_eq!(offset("\\sec", "\\section"), ("section{}".into(), 8));
    assert_eq!(offset("\\fr", "\\frac"), ("frac{}{}".into(), 5));
    assert_eq!(offset("\\includeg", "\\includegraphics"), ("includegraphics[]{}".into(), 16));
    assert!(complete(&backend, "\\zzz", 4, None).is_none());
}

#[test]
fn columns_are_utf16_units() {
    let backend = BackendClient::new().unwrap();
    // "𝛼" is two UTF-16 units; the command starts at unit 3.
    let line = "𝛼 \\sec";
    let c = complete(&backend, line, 7, None).unwrap();
    assert_eq!((c.from, c.to), (4, 7));
}
