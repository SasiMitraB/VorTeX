use vortex_core::semantic_index::SemanticIndex;

#[test]
fn test_semantic_index_update_and_query() {
    let index = SemanticIndex::new();
    index.clear();

    let tex_content = r#"
\documentclass{article}
\begin{document}
\section{First Section}
\label{sec:first}
\cite{einstein1905}
\ref{fig:arch}
\end{document}
    "#;

    index.update_file("/path/to/main.tex", tex_content);

    let labels = index.get_all_labels();
    assert_eq!(labels.len(), 1);
    assert_eq!(labels[0].key, "sec:first");

    let sections = index.get_sections_for_file("/path/to/main.tex");
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].title, "First Section");

    let cited = index.get_cited_keys("/path/to/main.tex");
    assert!(cited.contains("einstein1905"));

    let referenced = index.get_referenced_labels("/path/to/main.tex");
    assert!(referenced.contains("fig:arch"));

    let stats = index.get_stats();
    assert_eq!(stats.labels, 1);
    assert_eq!(stats.files_indexed, 1);

    // Remove file
    index.remove_file("/path/to/main.tex");
    assert_eq!(index.get_all_labels().len(), 0);
    assert_eq!(index.get_stats().files_indexed, 0);
}

#[test]
fn test_semantic_index_bib_file() {
    let index = SemanticIndex::new();
    index.clear();

    let bib_content = r#"
@article{turing1936,
  author = {Alan M. Turing},
  title = {On Computable Numbers},
  year = {1936}
}
    "#;

    index.update_file("/path/to/refs.bib", bib_content);

    let citations = index.get_all_citations();
    assert_eq!(citations.len(), 1);
    assert_eq!(citations[0].key, "turing1936");
}
