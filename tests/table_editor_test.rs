use vortex::services::table_parser::ColAlign;
use vortex::views::table_editor::TableSpreadsheet;

#[test]
fn test_spreadsheet_add_and_remove_dimensions() {
    let mut sheet = TableSpreadsheet::default();
    assert_eq!(sheet.model.num_rows(), 4);
    assert_eq!(sheet.model.num_cols(), 3);

    // Add 10 rows and 5 columns (unbounded)
    for _ in 0..10 {
        sheet.add_row();
    }
    for _ in 0..5 {
        sheet.add_col();
    }
    assert_eq!(sheet.model.num_rows(), 14);
    assert_eq!(sheet.model.num_cols(), 8);

    // Remove row
    sheet.active_cell = Some((2, 1));
    sheet.remove_row();
    assert_eq!(sheet.model.num_rows(), 13);

    // Remove col
    sheet.active_cell = Some((2, 3));
    sheet.remove_col();
    assert_eq!(sheet.model.num_cols(), 7);
}

#[test]
fn test_spreadsheet_merge_and_unmerge_cells() {
    let mut sheet = TableSpreadsheet::default();
    sheet.set_cell(1, 0, "SpanCell".to_string());

    // Merge (1, 0) to (1, 1) -> col_span = 2
    sheet.merge_selection(1, 0, 1, 1);
    let cell_0 = sheet.model.get_cell(1, 0).unwrap();
    let cell_1 = sheet.model.get_cell(1, 1).unwrap();

    assert_eq!(cell_0.col_span, 2);
    assert!(!cell_0.is_shadow);
    assert!(cell_1.is_shadow);

    let latex = sheet.generate_latex();
    assert!(latex.contains("\\multicolumn{2}{c}{SpanCell}"));

    // Unmerge
    sheet.unmerge_cell(1, 0);
    let cell_0_after = sheet.model.get_cell(1, 0).unwrap();
    let cell_1_after = sheet.model.get_cell(1, 1).unwrap();
    assert_eq!(cell_0_after.col_span, 1);
    assert!(!cell_0_after.is_shadow);
    assert!(!cell_1_after.is_shadow);
}

#[test]
fn test_spreadsheet_clipboard_multi_cell_paste() {
    let mut sheet = TableSpreadsheet::default();
    sheet.active_cell = Some((1, 1));

    let clipboard_tsv = "A1\tB1\tC1\nA2\tB2\tC2";
    sheet.paste_clipboard(clipboard_tsv);

    assert_eq!(sheet.model.get_cell(1, 1).unwrap().content, "A1");
    assert_eq!(sheet.model.get_cell(1, 2).unwrap().content, "B1");
    assert_eq!(sheet.model.get_cell(1, 3).unwrap().content, "C1");
    assert_eq!(sheet.model.get_cell(2, 1).unwrap().content, "A2");
    assert_eq!(sheet.model.get_cell(2, 2).unwrap().content, "B2");
    assert_eq!(sheet.model.get_cell(2, 3).unwrap().content, "C2");
}

#[test]
fn test_spreadsheet_column_alignment_and_booktabs() {
    let mut sheet = TableSpreadsheet::default();
    assert_eq!(sheet.model.columns[0].alignment, ColAlign::Left);

    sheet.cycle_col_align(0);
    assert_eq!(sheet.model.columns[0].alignment, ColAlign::Center);

    sheet.cycle_col_align(0);
    assert_eq!(sheet.model.columns[0].alignment, ColAlign::Right);

    sheet.cycle_col_align(0);
    assert_eq!(sheet.model.columns[0].alignment, ColAlign::Left);

    // Booktabs toggle
    assert!(sheet.model.booktabs);
    let bt_latex = sheet.generate_latex();
    assert!(bt_latex.contains("\\toprule"));
    assert!(bt_latex.contains("\\bottomrule"));

    sheet.toggle_booktabs();
    assert!(!sheet.model.booktabs);
    let hline_latex = sheet.generate_latex();
    assert!(hline_latex.contains("\\hline"));
    assert!(!hline_latex.contains("\\toprule"));
}

#[test]
fn test_latex_table_bidirectional_roundtrip() {
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

    let mut sheet = TableSpreadsheet::default();
    sheet.load_from_latex(source, Some(0..source.len()), Some("doc.tex".to_string()));

    assert_eq!(sheet.model.caption.as_deref(), Some("Measurement Data"));
    assert_eq!(sheet.model.label.as_deref(), Some("tbl:measurements"));
    assert_eq!(sheet.model.num_rows(), 3);
    assert_eq!(sheet.model.num_cols(), 3);
    assert_eq!(sheet.model.get_cell(1, 0).unwrap().content, "Voltage");
    assert_eq!(sheet.model.get_cell(2, 1).unwrap().content, "5");

    // Modify a cell in spreadsheet
    sheet.set_cell(2, 1, "10".to_string());

    let output = sheet.generate_latex();
    assert!(output.contains("\\caption{Measurement Data}"));
    assert!(output.contains("\\label{tbl:measurements}"));
    assert!(output.contains("Current & 10"));
}
