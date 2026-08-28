use vortex::services::latex_parser::{get_section_level, parse_latex_file};

#[test]
fn test_extracts_document_sections_with_correct_hierarchy_levels() {
    let content = r#"
\documentclass{article}
\begin{document}
\part{Overarching Part}
\chapter{First Chapter}
\section{Introduction}
Some text
\subsection{Background}
More details
\subsubsection{Details}
Deep sub
\paragraph{Paragraph header}
Inline para
\subparagraph{Subparagraph header}
Deepest
\end{document}
    "#;

    let result = parse_latex_file("/test/sections.tex", content);
    assert_eq!(result.sections.len(), 7);

    assert_eq!(result.sections[0].title, "Overarching Part");
    assert_eq!(result.sections[0].level, 0);

    assert_eq!(result.sections[1].title, "First Chapter");
    assert_eq!(result.sections[1].level, 1);

    assert_eq!(result.sections[2].title, "Introduction");
    assert_eq!(result.sections[2].level, 2);

    assert_eq!(result.sections[3].title, "Background");
    assert_eq!(result.sections[3].level, 3);

    assert_eq!(result.sections[4].title, "Details");
    assert_eq!(result.sections[4].level, 4);

    assert_eq!(result.sections[5].title, "Paragraph header");
    assert_eq!(result.sections[5].level, 5);

    assert_eq!(result.sections[6].title, "Subparagraph header");
    assert_eq!(result.sections[6].level, 6);
}

#[test]
fn test_extracts_labels_correctly() {
    let content = r#"
\documentclass{article}
\begin{document}
\section{Intro}
\label{sec:intro}
Some equation
\label{eq:einstein}
\end{document}
    "#;

    let result = parse_latex_file("/test/labels.tex", content);
    assert_eq!(result.labels.len(), 2);
    assert_eq!(result.labels[0].key, "sec:intro");
    assert_eq!(result.labels[0].filename, "labels.tex");
    assert_eq!(result.labels[1].key, "eq:einstein");
}

#[test]
fn test_extracts_citations_refs_and_bibliographies() {
    let content = r#"
\documentclass{article}
\begin{document}
As shown by \cite{knuth1984, lamport1994} and \citep{einstein1905}.
See \ref{sec:intro} and equation \eqref{eq:einstein}.
\bibliography{references}
\addbibresource{extra.bib}
\end{document}
    "#;

    let result = parse_latex_file("/test/doc.tex", content);

    // Citations
    assert_eq!(result.citations.len(), 2);
    assert_eq!(result.citations[0].keys, vec!["knuth1984", "lamport1994"]);
    assert_eq!(result.citations[1].keys, vec!["einstein1905"]);

    // Refs
    assert_eq!(result.refs.len(), 2);
    assert_eq!(result.refs[0].key, "sec:intro");
    assert_eq!(result.refs[1].key, "eq:einstein");

    // Bibliographies
    assert_eq!(result.bibliographies.len(), 2);
    assert_eq!(result.bibliographies[0].file, "references");
    assert_eq!(result.bibliographies[1].file, "extra.bib");
}

#[test]
fn test_get_section_level_mapping() {
    assert_eq!(get_section_level("part"), 0);
    assert_eq!(get_section_level("chapter"), 1);
    assert_eq!(get_section_level("section"), 2);
    assert_eq!(get_section_level("subsection"), 3);
    assert_eq!(get_section_level("subsubsection"), 4);
    assert_eq!(get_section_level("paragraph"), 5);
    assert_eq!(get_section_level("subparagraph"), 6);
    assert_eq!(get_section_level("unknown"), 2);
}
