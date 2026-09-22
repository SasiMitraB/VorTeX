use vortex_core::backend::BackendClient;
use vortex_core::latex_parser::parse_latex_file;
use vortex_core::semantic_index::SemanticIndex;

#[test]
fn test_parse_latex_todo_comments() {
    let content = r#"\documentclass{article}
\title{Sample Paper}
% TODO: write abstract section
\begin{document}
\section{Introduction}
Some text here.
% FIXME: check formula (2.1)
\subsection{Background}
More background details.
% NOTE: verify reference to Turing1936
% [ ] Add concluding remarks
% [x] Proofread section 1
\end{document}
"#;

    let parsed = parse_latex_file("/fake/path/paper.tex", content);

    assert_eq!(parsed.sections.len(), 2);
    assert_eq!(parsed.sections[0].title, "Introduction");
    assert_eq!(parsed.sections[0].line, 5);
    assert_eq!(parsed.sections[1].title, "Background");
    assert_eq!(parsed.sections[1].line, 8);

    assert_eq!(parsed.todos.len(), 5);

    assert_eq!(parsed.todos[0].tag, "TODO");
    assert_eq!(parsed.todos[0].text, "write abstract section");
    assert_eq!(parsed.todos[0].line, 3);
    assert_eq!(parsed.todos[0].filename, "paper.tex");

    assert_eq!(parsed.todos[1].tag, "FIXME");
    assert_eq!(parsed.todos[1].text, "check formula (2.1)");
    assert_eq!(parsed.todos[1].line, 7);

    assert_eq!(parsed.todos[2].tag, "NOTE");
    assert_eq!(parsed.todos[2].text, "verify reference to Turing1936");
    assert_eq!(parsed.todos[2].line, 10);

    assert_eq!(parsed.todos[3].tag, "TODO");
    assert_eq!(parsed.todos[3].text, "Add concluding remarks");
    assert_eq!(parsed.todos[3].line, 11);

    assert_eq!(parsed.todos[4].tag, "DONE");
    assert_eq!(parsed.todos[4].text, "Proofread section 1");
    assert_eq!(parsed.todos[4].line, 12);
}

#[test]
fn test_semantic_index_todos_and_sections() {
    let index = SemanticIndex::new();

    let doc1 = r#"\section{Intro}
% TODO: intro todo
Text"#;
    let doc2 = r#"\section{Methods}
% FIXME: method fixme
More text"#;

    index.update_file("/proj/doc1.tex", doc1);
    index.update_file("/proj/doc2.tex", doc2);

    let all_sections = index.get_all_sections();
    assert_eq!(all_sections.len(), 2);
    assert_eq!(all_sections[0].title, "Intro");
    assert_eq!(all_sections[1].title, "Methods");

    let all_todos = index.get_all_todos();
    assert_eq!(all_todos.len(), 2);
    assert_eq!(all_todos[0].tag, "TODO");
    assert_eq!(all_todos[0].text, "intro todo");
    assert_eq!(all_todos[1].tag, "FIXME");
    assert_eq!(all_todos[1].text, "method fixme");

    let doc1_todos = index.get_todos_for_file("/proj/doc1.tex");
    assert_eq!(doc1_todos.len(), 1);
    assert_eq!(doc1_todos[0].text, "intro todo");

    // Test removing file
    index.remove_file("/proj/doc1.tex");
    let remaining_todos = index.get_all_todos();
    assert_eq!(remaining_todos.len(), 1);
    assert_eq!(remaining_todos[0].file, "/proj/doc2.tex");
}

#[test]
fn test_backend_client_todo_and_section_api() {
    let backend = BackendClient::new().unwrap();

    let temp_dir = std::env::temp_dir().join(format!("vortex_todo_test_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&temp_dir).unwrap();

    let main_tex = temp_dir.join("main.tex");
    std::fs::write(&main_tex, r#"\section{Main Section}
% TODO: finalize intro
\subsection{Details}
% NOTE: check bibliography
"#).unwrap();

    backend.init_project(temp_dir.to_str().unwrap()).unwrap();

    let sections = backend.get_all_sections().unwrap();
    assert_eq!(sections.len(), 2);
    assert_eq!(sections[0].title, "Main Section");
    assert_eq!(sections[1].title, "Details");

    let todos = backend.get_todos(None).unwrap();
    assert_eq!(todos.len(), 2);
    assert_eq!(todos[0].tag, "TODO");
    assert_eq!(todos[0].text, "finalize intro");
    assert_eq!(todos[1].tag, "NOTE");
    assert_eq!(todos[1].text, "check bibliography");

    let _ = std::fs::remove_dir_all(temp_dir);
}
