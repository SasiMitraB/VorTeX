use vortex_core::latex_parser::{get_section_level, parse_latex_file};

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

#[test]
fn test_extracts_tables_correctly() {
    let content = r#"
\documentclass{article}
\begin{document}
\section{Results}

\begin{table}[h!]
  \centering
  \caption{Test Results Table}
  \label{tbl:results}
  \begin{tabular}{|l|c|r|}
    \toprule
    Name & Score & Rank \\
    \midrule
    Alice & 98 & 1 \\
    Bob & 85 & 2 \\
    \bottomrule
  \end{tabular}
\end{table}

Some text in between.

\begin{tabular}{c c}
  1 & 2 \\
  3 & 4 \\
\end{tabular}

\end{document}
    "#;

    let result = parse_latex_file("/test/doc_with_tables.tex", content);
    assert_eq!(result.tables.len(), 2);

    assert_eq!(result.tables[0].caption.as_deref(), Some("Test Results Table"));
    assert_eq!(result.tables[0].label.as_deref(), Some("tbl:results"));
    assert!(result.tables[0].byte_end > result.tables[0].byte_start);

    // Standalone tabular
    assert_eq!(result.tables[1].caption, None);
    assert_eq!(result.tables[1].label, None);
    assert!(result.tables[1].byte_end > result.tables[1].byte_start);
}


#[test]
fn label_kinds_from_prefixes() {
    use vortex_core::latex_parser::{label_kind, LabelKind};
    assert_eq!(label_kind("fig:plot"), LabelKind::Figure);
    assert_eq!(label_kind("Eq:energy"), LabelKind::Equation);
    assert_eq!(label_kind("subsec:a"), LabelKind::Section);
    assert_eq!(label_kind("tbl:x"), LabelKind::Table);
    assert_eq!(label_kind("thm:main"), LabelKind::Other);
    assert_eq!(label_kind("nocolon"), LabelKind::Other);
}
