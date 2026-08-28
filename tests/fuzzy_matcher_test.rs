use vortex::services::bibtex_parser::{BibEntryItem, BibFields};
use vortex::services::fuzzy_matcher::{extract_prefix, filter_by_prefix, Matcher};
use vortex::services::latex_parser::LabelItem;

#[test]
fn test_fuzzy_search_labels_exact_and_fuzzy() {
    let matcher = Matcher::new();

    let mock_labels = vec![
        LabelItem {
            key: "fig:performance_chart".to_string(),
            file: "/path/to/main.tex".to_string(),
            filename: "main.tex".to_string(),
            line: 12,
        },
        LabelItem {
            key: "eq:einstein_mass".to_string(),
            file: "/path/to/other.tex".to_string(),
            filename: "other.tex".to_string(),
            line: 45,
        },
        LabelItem {
            key: "sec:intro".to_string(),
            file: "/path/to/main.tex".to_string(),
            filename: "main.tex".to_string(),
            line: 5,
        },
    ];

    // Exact search
    let results = matcher.search_labels("fig:performance_chart", &mock_labels, None);
    assert!(!results.is_empty());
    assert_eq!(results[0].obj.get("key").and_then(|k| k.as_str()), Some("fig:performance_chart"));

    // Fuzzy search
    let fuzzy_results = matcher.search_labels("perf", &mock_labels, None);
    assert!(!fuzzy_results.is_empty());
    assert_eq!(fuzzy_results[0].obj.get("key").and_then(|k| k.as_str()), Some("fig:performance_chart"));

    // Active file boosting
    let boosted = matcher.search_labels("intro", &mock_labels, Some("/path/to/main.tex"));
    assert!(!boosted.is_empty());
    assert_eq!(boosted[0].obj.get("key").and_then(|k| k.as_str()), Some("sec:intro"));
}

#[test]
fn test_fuzzy_search_bib_entries() {
    let matcher = Matcher::new();

    let mock_bibs = vec![
        BibEntryItem {
            item_type: "bibentry".to_string(),
            key: "doe2023deep".to_string(),
            entry_type: Some("article".to_string()),
            file: "/path/to/refs.bib".to_string(),
            filename: "refs.bib".to_string(),
            fields: Some(BibFields {
                title: Some("Deep Learning for Compilers".to_string()),
                author: Some("John Doe and Jane Smith".to_string()),
                year: Some("2023".to_string()),
                journal: Some("ACM Transactions on Architecture".to_string()),
                keywords: Some("compilers, optimization, neural networks".to_string()),
                ..Default::default()
            }),
        },
        BibEntryItem {
            item_type: "bibentry".to_string(),
            key: "knuth1984lit".to_string(),
            entry_type: Some("article".to_string()),
            file: "/path/to/refs.bib".to_string(),
            filename: "refs.bib".to_string(),
            fields: Some(BibFields {
                title: Some("Literate Programming".to_string()),
                author: Some("Donald E. Knuth".to_string()),
                year: Some("1984".to_string()),
                journal: Some("The Computer Journal".to_string()),
                keywords: Some("documentation, pascal, algorithms".to_string()),
                ..Default::default()
            }),
        },
    ];

    // Search by title keyword
    let results = matcher.search_bib_entries("Compilers", &mock_bibs);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].obj.get("key").and_then(|k| k.as_str()), Some("doe2023deep"));

    // Search by author
    let author_results = matcher.search_bib_entries("Knuth", &mock_bibs);
    assert_eq!(author_results.len(), 1);
    assert_eq!(author_results[0].obj.get("key").and_then(|k| k.as_str()), Some("knuth1984lit"));

    // Search by keywords
    let keyword_results = matcher.search_bib_entries("optimization", &mock_bibs);
    assert_eq!(keyword_results.len(), 1);
    assert_eq!(keyword_results[0].obj.get("key").and_then(|k| k.as_str()), Some("doe2023deep"));

    // Search by year
    let year_results = matcher.search_bib_entries("1984", &mock_bibs);
    assert_eq!(year_results.len(), 1);
    assert_eq!(year_results[0].obj.get("key").and_then(|k| k.as_str()), Some("knuth1984lit"));
}


#[test]
fn test_prefix_filter_and_extraction() {
    let mock_labels = vec![
        LabelItem {
            key: "fig:1".to_string(),
            file: "a.tex".to_string(),
            filename: "a.tex".to_string(),
            line: 1,
        },
        LabelItem {
            key: "eq:1".to_string(),
            file: "a.tex".to_string(),
            filename: "a.tex".to_string(),
            line: 2,
        },
    ];

    let filtered = filter_by_prefix(&mock_labels, "fig:");
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].key, "fig:1");

    assert_eq!(extract_prefix("fig:test"), Some("fig:"));
    assert_eq!(extract_prefix("noprefix"), None);
}
