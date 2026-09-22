use vortex_core::table_editor::{apply, load, TableOp};
use vortex_core::table_parser::{generate_latex_table, ColAlign, TableModel};

#[test]
fn add_and_remove_rows_and_columns() {
    let mut m = TableModel::default();
    assert_eq!((m.num_rows(), m.num_cols()), (4, 3));
    for _ in 0..10 {
        apply(&mut m, TableOp::AddRow);
    }
    for _ in 0..5 {
        apply(&mut m, TableOp::AddCol);
    }
    assert_eq!((m.num_rows(), m.num_cols()), (14, 8));
    apply(&mut m, TableOp::RemoveRow { row: 2 });
    apply(&mut m, TableOp::RemoveCol { col: 3 });
    assert_eq!((m.num_rows(), m.num_cols()), (13, 7));

    // The last row and column are never removed.
    let mut tiny = load("\\begin{tabular}{l}\nA \\\\\n\\end{tabular}");
    apply(&mut tiny, TableOp::RemoveRow { row: 0 });
    apply(&mut tiny, TableOp::RemoveCol { col: 0 });
    assert_eq!((tiny.num_rows(), tiny.num_cols()), (1, 1));
}

#[test]
fn merge_and_unmerge_cells() {
    let mut m = TableModel::default();
    apply(&mut m, TableOp::SetCell { row: 1, col: 0, text: "SpanCell".into() });
    apply(&mut m, TableOp::Merge { row1: 1, col1: 0, row2: 1, col2: 1 });
    assert_eq!(m.get_cell(1, 0).unwrap().col_span, 2);
    assert!(m.get_cell(1, 1).unwrap().is_shadow);
    assert!(generate_latex_table(&m).contains("\\multicolumn{2}{c}{SpanCell}"));

    apply(&mut m, TableOp::Unmerge { row: 1, col: 0 });
    assert_eq!(m.get_cell(1, 0).unwrap().col_span, 1);
    assert!(!m.get_cell(1, 1).unwrap().is_shadow);
}

#[test]
fn paste_tab_separated_grows_the_table() {
    let mut m = TableModel::default();
    apply(&mut m, TableOp::Paste { row: 3, col: 1, text: "A1\tB1\tC1\nA2\tB2\tC2".into() });
    assert_eq!((m.num_rows(), m.num_cols()), (5, 4));
    assert_eq!(m.get_cell(3, 1).unwrap().content, "A1");
    assert_eq!(m.get_cell(3, 3).unwrap().content, "C1");
    assert_eq!(m.get_cell(4, 2).unwrap().content, "B2");

    // A single value goes into one cell.
    apply(&mut m, TableOp::Paste { row: 0, col: 0, text: "  solo \n".into() });
    assert_eq!(m.get_cell(0, 0).unwrap().content, "solo");
}

#[test]
fn column_alignment_and_booktabs() {
    let mut m = TableModel::default();
    assert_eq!(m.columns[0].alignment, ColAlign::Left);
    for expected in [ColAlign::Center, ColAlign::Right, ColAlign::Left] {
        apply(&mut m, TableOp::CycleAlign { col: 0 });
        assert_eq!(m.columns[0].alignment, expected);
    }

    assert!(generate_latex_table(&m).contains("\\toprule"));
    apply(&mut m, TableOp::ToggleBooktabs);
    let latex = generate_latex_table(&m);
    assert!(latex.contains("\\hline") && !latex.contains("\\toprule"));
}

#[test]
fn caption_and_label_are_trimmed_or_cleared() {
    let mut m = TableModel::default();
    apply(&mut m, TableOp::SetCaption { text: Some("  Results ".into()) });
    apply(&mut m, TableOp::SetLabel { text: Some("tab:res".into()) });
    assert_eq!(m.caption.as_deref(), Some("Results"));
    apply(&mut m, TableOp::SetLabel { text: Some("   ".into()) });
    assert_eq!(m.label, None);
}

#[test]
fn latex_roundtrip() {
    let source = r#"\begin{table}[h!]
  \centering
  \caption{Measurement Data}
  \label{tbl:measurements}
  \begin{tabular}{l c r}
    \toprule
    Item & Value & Unit \\
    \midrule
    Voltage & 220 & V \\
    Current & 5 & A \\
    \bottomrule
  \end{tabular}
\end{table}"#;

    let mut m = load(source);
    assert_eq!(m.caption.as_deref(), Some("Measurement Data"));
    assert_eq!(m.label.as_deref(), Some("tbl:measurements"));
    assert_eq!((m.num_rows(), m.num_cols()), (3, 3));
    assert_eq!(m.get_cell(1, 0).unwrap().content, "Voltage");

    apply(&mut m, TableOp::SetCell { row: 2, col: 1, text: "10".into() });
    let out = generate_latex_table(&m);
    assert!(out.contains("\\caption{Measurement Data}"));
    assert!(out.contains("\\label{tbl:measurements}"));
    assert!(out.contains("Current & 10"));

    // Unparseable input gives the starter table.
    assert_eq!(load("not a table"), TableModel::default());
}

#[test]
fn ops_serialize_with_an_op_tag() {
    let op: TableOp = serde_json::from_str(r#"{"op":"setCell","row":1,"col":2,"text":"x"}"#).unwrap();
    assert_eq!(op, TableOp::SetCell { row: 1, col: 2, text: "x".into() });
    assert_eq!(serde_json::to_string(&TableOp::AddRow).unwrap(), r#"{"op":"addRow"}"#);
}
