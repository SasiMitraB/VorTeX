use vortex::services::bibtex_parser::{clean_field, parse_bib_file};

#[test]
fn test_parses_bibtex_entries_into_structured_objects() {
    let content = r#"
@article{knuth1984,
  author = {Donald E. Knuth},
  title = {Literate Programming},
  journal = {The Computer Journal},
  year = {1984},
  doi = {10.1093/comjnl/27.2.97}
}

@book{lamport1994,
  author = {Leslie Lamport},
  title = {LaTeX: A Document Preparation System},
  publisher = {Addison-Wesley},
  year = {1994}
}
    "#;

    let entries = parse_bib_file("/test/refs.bib", content);
    assert_eq!(entries.len(), 2);

    let knuth = &entries[0];
    assert_eq!(knuth.key.to_lowercase(), "knuth1984");
    assert_eq!(knuth.entry_type.as_deref(), Some("article"));
    let fields = knuth.fields.as_ref().unwrap();
    assert_eq!(fields.author.as_deref(), Some("Donald E. Knuth"));
    assert_eq!(fields.title.as_deref(), Some("Literate Programming"));
    assert_eq!(fields.journal.as_deref(), Some("The Computer Journal"));
    assert_eq!(fields.year.as_deref(), Some("1984"));
    assert_eq!(fields.doi.as_deref(), Some("10.1093/comjnl/27.2.97"));

    let lamport = &entries[1];
    assert_eq!(lamport.key.to_lowercase(), "lamport1994");
    assert_eq!(lamport.entry_type.as_deref(), Some("book"));
    let l_fields = lamport.fields.as_ref().unwrap();
    assert_eq!(l_fields.author.as_deref(), Some("Leslie Lamport"));
    assert_eq!(l_fields.title.as_deref(), Some("LaTeX: A Document Preparation System"));
    assert_eq!(l_fields.publisher.as_deref(), Some("Addison-Wesley"));
    assert_eq!(l_fields.year.as_deref(), Some("1994"));
}

#[test]
fn test_bibtex_parentheses_and_quoted_fields() {
    let content = r#"
% Sample bibliography comment
@article{vaswani2017attention,
  author    = {Vaswani, Ashish and Shazeer, Noam and Parmar, Niki (Core Dev)},
  title     = {Attention is All you Need (NeurIPS 2017)},
  journal   = {Advances in Neural Information Processing Systems (NeurIPS)},
  volume    = {30},
  year      = 2017,
  keywords  = {deep learning, transformers, attention mechanism}
}

@inproceedings{goodfellow2014gan,
  title="Generative Adversarial \"Nets\"",
  author="Ian Goodfellow and Jean Pouget-Abadie",
  booktitle="NIPS (Conference)",
  year=2014,
  eprint="1406.2661"
}
    "#;

    let entries = parse_bib_file("/test/refs.bib", content);
    assert_eq!(entries.len(), 2);

    let vaswani = &entries[0];
    assert_eq!(vaswani.key, "vaswani2017attention");
    let v_fields = vaswani.fields.as_ref().unwrap();
    assert_eq!(v_fields.title.as_deref(), Some("Attention is All you Need (NeurIPS 2017)"));
    assert_eq!(v_fields.journal.as_deref(), Some("Advances in Neural Information Processing Systems (NeurIPS)"));
    assert_eq!(v_fields.year.as_deref(), Some("2017"));
    assert_eq!(v_fields.keywords.as_deref(), Some("deep learning, transformers, attention mechanism"));

    let gan = &entries[1];
    assert_eq!(gan.key, "goodfellow2014gan");
    let g_fields = gan.fields.as_ref().unwrap();
    assert_eq!(g_fields.title.as_deref(), Some("Generative Adversarial \"Nets\""));
    assert_eq!(g_fields.booktitle.as_deref(), Some("NIPS (Conference)"));
    assert_eq!(g_fields.year.as_deref(), Some("2014"));
    assert_eq!(g_fields.eprint.as_deref(), Some("1406.2661"));
}

#[test]
fn test_clean_field_accents_and_braces() {
    assert_eq!(clean_field(r#"{\"{o}}liver"#), "oliver");
    assert_eq!(clean_field(r#"{\'E}tienne"#), "Etienne");
    assert_eq!(clean_field(r#"Simple Text"#), "Simple Text");
    assert_eq!(clean_field(r#"{\textbf{Bold} Title}"#), "Bold Title");
    assert_eq!(clean_field(r#"The {BERT} model for {NLP}"#), "The BERT model for NLP");
    assert_eq!(clean_field(r#"Tom \& Jerry"#), "Tom & Jerry");
}
