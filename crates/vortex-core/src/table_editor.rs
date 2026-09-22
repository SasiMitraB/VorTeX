//! Editing operations for the visual table editor.
//!
//! The editor keeps a `TableModel`, sends one `TableOp` per user action, and
//! renders the model that comes back; `generate_latex_table` gives the code.

use crate::table_parser::{parse_latex_table, ColAlign, TableModel};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum TableOp {
    AddRow,
    /// Keeps at least one row.
    RemoveRow { row: u32 },
    AddCol,
    /// Keeps at least one column.
    RemoveCol { col: u32 },
    SetCell { row: u32, col: u32, text: String },
    /// Left → center → right → left (a `p{…}` column becomes left).
    CycleAlign { col: u32 },
    ToggleBooktabs,
    /// Merges the rectangle spanned by the two corners.
    Merge { row1: u32, col1: u32, row2: u32, col2: u32 },
    Unmerge { row: u32, col: u32 },
    /// Tab-separated rows (as copied from a spreadsheet), starting at the given cell;
    /// the table grows to fit.
    Paste { row: u32, col: u32, text: String },
    /// Empty or blank clears it.
    SetCaption { text: Option<String> },
    SetLabel { text: Option<String> },
}

/// The table in `source`, or a blank starter table when it can't be parsed.
pub fn load(source: &str) -> TableModel {
    parse_latex_table(source).unwrap_or_default()
}

pub fn apply(model: &mut TableModel, op: TableOp) {
    let clean = |t: Option<String>| t.map(|t| t.trim().to_string()).filter(|t| !t.is_empty());
    match op {
        TableOp::AddRow => model.add_row(),
        TableOp::RemoveRow { row } => model.remove_row(row as usize),
        TableOp::AddCol => model.add_col(),
        TableOp::RemoveCol { col } => model.remove_col(col as usize),
        TableOp::SetCell { row, col, text } => model.set_cell(row as usize, col as usize, text),
        TableOp::CycleAlign { col } => {
            if let Some(col) = model.columns.get_mut(col as usize) {
                col.alignment = match col.alignment {
                    ColAlign::Left => ColAlign::Center,
                    ColAlign::Center => ColAlign::Right,
                    ColAlign::Right | ColAlign::Paragraph(_) => ColAlign::Left,
                };
            }
        }
        TableOp::ToggleBooktabs => model.booktabs = !model.booktabs,
        TableOp::Merge { row1, col1, row2, col2 } => {
            model.merge_cells(row1 as usize, col1 as usize, row2 as usize, col2 as usize)
        }
        TableOp::Unmerge { row, col } => model.unmerge_cell(row as usize, col as usize),
        TableOp::Paste { row, col, text } => paste(model, row as usize, col as usize, &text),
        TableOp::SetCaption { text } => model.caption = clean(text),
        TableOp::SetLabel { text } => model.label = clean(text),
    }
}

fn paste(model: &mut TableModel, start_r: usize, start_c: usize, text: &str) {
    let rows: Vec<&str> = text.lines().filter(|s| !s.trim().is_empty()).collect();
    if rows.is_empty() {
        return;
    }
    if rows.len() == 1 && !rows[0].contains('\t') {
        model.set_cell(start_r, start_c, text.trim().to_string());
        return;
    }
    for (dr, row_str) in rows.iter().enumerate() {
        let r = start_r + dr;
        while r >= model.num_rows() {
            model.add_row();
        }
        for (dc, cell_str) in row_str.split('\t').enumerate() {
            let c = start_c + dc;
            while c >= model.num_cols() {
                model.add_col();
            }
            model.set_cell(r, c, cell_str.trim().to_string());
        }
    }
}
