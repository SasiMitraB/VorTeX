#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColAlign {
    Left,
    Center,
    Right,
    Paragraph(String),
}

impl Default for ColAlign {
    fn default() -> Self {
        ColAlign::Center
    }
}

impl ColAlign {
    pub fn to_spec_char(&self) -> String {
        match self {
            ColAlign::Left => "l".to_string(),
            ColAlign::Center => "c".to_string(),
            ColAlign::Right => "r".to_string(),
            ColAlign::Paragraph(w) => format!("p{{{}}}", w),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColumnSpec {
    pub alignment: ColAlign,
    pub border_left: bool,
    pub border_right: bool,
}

impl Default for ColumnSpec {
    fn default() -> Self {
        Self {
            alignment: ColAlign::Center,
            border_left: false,
            border_right: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleKind {
    Toprule,
    Midrule,
    Bottomrule,
    Hline,
    Cline(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableCell {
    pub content: String,
    pub col_span: usize,
    pub row_span: usize,
    pub col_spec_override: Option<String>,
    pub is_shadow: bool,
}

impl Default for TableCell {
    fn default() -> Self {
        Self {
            content: String::new(),
            col_span: 1,
            row_span: 1,
            col_spec_override: None,
            is_shadow: false,
        }
    }
}

impl TableCell {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            col_span: 1,
            row_span: 1,
            col_spec_override: None,
            is_shadow: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
    pub rule_above: Option<RuleKind>,
    pub rule_below: Option<RuleKind>,
}

impl Default for TableRow {
    fn default() -> Self {
        Self {
            cells: Vec::new(),
            rule_above: None,
            rule_below: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableModel {
    pub columns: Vec<ColumnSpec>,
    pub rows: Vec<TableRow>,
    pub booktabs: bool,
    pub caption: Option<String>,
    pub label: Option<String>,
    pub source_span: Option<Range<usize>>,
    pub source_file: Option<String>,
}

impl Default for TableModel {
    fn default() -> Self {
        let cols = vec![
            ColumnSpec { alignment: ColAlign::Left, border_left: false, border_right: false },
            ColumnSpec { alignment: ColAlign::Center, border_left: false, border_right: false },
            ColumnSpec { alignment: ColAlign::Right, border_left: false, border_right: false },
        ];

        let mut rows = Vec::new();
        for r in 0..4 {
            let mut cells = Vec::new();
            for c in 0..3 {
                if r == 0 {
                    cells.push(TableCell::new(format!("Header {}", c + 1)));
                } else {
                    cells.push(TableCell::new(format!("Data {},{}", r, c + 1)));
                }
            }
            rows.push(TableRow {
                cells,
                rule_above: if r == 0 { Some(RuleKind::Toprule) } else { None },
                rule_below: if r == 0 { Some(RuleKind::Midrule) } else if r == 3 { Some(RuleKind::Bottomrule) } else { None },
            });
        }

        Self {
            columns: cols,
            rows,
            booktabs: true,
            caption: None,
            label: None,
            source_span: None,
            source_file: None,
        }
    }
}

impl TableModel {
    pub fn num_rows(&self) -> usize {
        self.rows.len()
    }

    pub fn num_cols(&self) -> usize {
        self.columns.len()
    }

    pub fn add_row(&mut self) {
        let cols = self.columns.len().max(1);
        let cells = vec![TableCell::default(); cols];
        self.rows.push(TableRow {
            cells,
            rule_above: None,
            rule_below: None,
        });
        self.recompute_shadow_cells();
    }

    pub fn remove_row(&mut self, idx: usize) {
        if self.rows.len() > 1 && idx < self.rows.len() {
            self.rows.remove(idx);
            self.recompute_shadow_cells();
        }
    }

    pub fn add_col(&mut self) {
        self.columns.push(ColumnSpec {
            alignment: ColAlign::Center,
            border_left: false,
            border_right: false,
        });
        for row in self.rows.iter_mut() {
            row.cells.push(TableCell::default());
        }
        self.recompute_shadow_cells();
    }

    pub fn remove_col(&mut self, idx: usize) {
        if self.columns.len() > 1 && idx < self.columns.len() {
            self.columns.remove(idx);
            for row in self.rows.iter_mut() {
                if idx < row.cells.len() {
                    row.cells.remove(idx);
                }
            }
            self.recompute_shadow_cells();
        }
    }

    pub fn set_cell(&mut self, r: usize, c: usize, text: String) {
        if r < self.rows.len() && c < self.rows[r].cells.len() {
            self.rows[r].cells[c].content = text;
        }
    }

    pub fn get_cell(&self, r: usize, c: usize) -> Option<&TableCell> {
        self.rows.get(r).and_then(|row| row.cells.get(c))
    }

    pub fn get_cell_mut(&mut self, r: usize, c: usize) -> Option<&mut TableCell> {
        self.rows.get_mut(r).and_then(|row| row.cells.get_mut(c))
    }

    pub fn merge_cells(&mut self, r1: usize, c1: usize, r2: usize, c2: usize) {
        let min_r = r1.min(r2);
        let max_r = r2.max(r1);
        let min_c = c1.min(c2);
        let max_c = c2.max(c1);

        if max_r >= self.rows.len() || max_c >= self.columns.len() {
            return;
        }

        let col_span = max_c - min_c + 1;
        let row_span = max_r - min_r + 1;

        if let Some(top_left) = self.get_cell_mut(min_r, min_c) {
            top_left.col_span = col_span;
            top_left.row_span = row_span;
            top_left.is_shadow = false;
        }

        self.recompute_shadow_cells();
    }

    pub fn unmerge_cell(&mut self, r: usize, c: usize) {
        if let Some(cell) = self.get_cell_mut(r, c) {
            cell.col_span = 1;
            cell.row_span = 1;
            cell.is_shadow = false;
            cell.col_spec_override = None;
        }
        self.recompute_shadow_cells();
    }

    pub fn recompute_shadow_cells(&mut self) {
        // Reset all shadows first
        for row in self.rows.iter_mut() {
            for cell in row.cells.iter_mut() {
                cell.is_shadow = false;
            }
        }

        let num_r = self.rows.len();
        let num_c = self.columns.len();

        for r in 0..num_r {
            for c in 0..num_c {
                if r < self.rows.len() && c < self.rows[r].cells.len() {
                    let col_span = self.rows[r].cells[c].col_span;
                    let row_span = self.rows[r].cells[c].row_span;

                    if col_span > 1 || row_span > 1 {
                        for dr in 0..row_span {
                            for dc in 0..col_span {
                                if dr == 0 && dc == 0 {
                                    continue;
                                }
                                let target_r = r + dr;
                                let target_c = c + dc;
                                if target_r < self.rows.len() && target_c < self.rows[target_r].cells.len() {
                                    self.rows[target_r].cells[target_c].is_shadow = true;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Parse column specifier like `|l|c|r|` or `l c p{3cm}` into Vec<ColumnSpec>
pub fn parse_col_specs(spec_str: &str) -> Vec<ColumnSpec> {
    let mut specs: Vec<ColumnSpec> = Vec::new();
    let chars: Vec<char> = spec_str.chars().collect();
    let mut i = 0;
    let mut pending_left_border = false;

    while i < chars.len() {
        let ch = chars[i];
        if ch.is_whitespace() {
            i += 1;
            continue;
        }

        if ch == '|' {
            pending_left_border = true;
            if let Some(last) = specs.last_mut() {
                last.border_right = true;
            }
            i += 1;
            continue;
        }

        if ch == 'l' || ch == 'c' || ch == 'r' {
            let alignment = match ch {
                'l' => ColAlign::Left,
                'c' => ColAlign::Center,
                'r' => ColAlign::Right,
                _ => unreachable!(),
            };
            specs.push(ColumnSpec {
                alignment,
                border_left: pending_left_border,
                border_right: false,
            });
            pending_left_border = false;
            i += 1;
            continue;
        }

        if ch == 'p' || ch == 'm' || ch == 'b' {
            // Check for width in braces e.g. p{3cm}
            let mut width = String::new();
            i += 1;
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            if i < chars.len() && chars[i] == '{' {
                i += 1;
                let mut depth = 1;
                while i < chars.len() && depth > 0 {
                    if chars[i] == '{' {
                        depth += 1;
                    } else if chars[i] == '}' {
                        depth -= 1;
                        if depth == 0 {
                            i += 1;
                            break;
                        }
                    }
                    width.push(chars[i]);
                    i += 1;
                }
            }
            specs.push(ColumnSpec {
                alignment: ColAlign::Paragraph(width),
                border_left: pending_left_border,
                border_right: false,
            });
            pending_left_border = false;
            continue;
        }

        // Other column specs like @{}, *{...}, etc.
        if ch == '@' || ch == '!' {
            // Skip parameter in braces
            i += 1;
            if i < chars.len() && chars[i] == '{' {
                i += 1;
                let mut depth = 1;
                while i < chars.len() && depth > 0 {
                    if chars[i] == '{' { depth += 1; }
                    else if chars[i] == '}' { depth -= 1; }
                    i += 1;
                }
            }
            continue;
        }

        // Unrecognized specifier, default to center
        specs.push(ColumnSpec {
            alignment: ColAlign::Center,
            border_left: pending_left_border,
            border_right: false,
        });
        pending_left_border = false;
        i += 1;
    }

    if specs.is_empty() {
        specs.push(ColumnSpec::default());
    }

    specs
}

/// Tokenize and split a LaTeX table body into rows and cells, respecting nested braces and math mode.
pub fn parse_table_body(body: &str, expected_cols: usize) -> (Vec<TableRow>, bool) {
    let mut rows: Vec<TableRow> = Vec::new();
    let mut has_booktabs = false;

    // Split body into rows by `\\`
    let raw_rows = split_by_token(body, "\\\\");

    for raw_row in raw_rows {
        let trimmed_row = raw_row.trim();
        if trimmed_row.is_empty() {
            continue;
        }

        let mut row_rule_above: Option<RuleKind> = None;
        let mut row_rule_below: Option<RuleKind> = None;
        let mut cleaned_row = String::new();

        // Process line by line or commands in raw_row
        let lines: Vec<&str> = trimmed_row.lines().collect();
        for line in lines {
            let l = line.trim();
            if l == "\\toprule" {
                row_rule_above = Some(RuleKind::Toprule);
                has_booktabs = true;
            } else if l == "\\midrule" {
                row_rule_above = Some(RuleKind::Midrule);
                has_booktabs = true;
            } else if l == "\\bottomrule" {
                row_rule_below = Some(RuleKind::Bottomrule);
                has_booktabs = true;
            } else if l == "\\hline" {
                row_rule_above = Some(RuleKind::Hline);
            } else if l.starts_with("\\cline{") {
                if let Some(end) = l.find('}') {
                    let cline_arg = l[7..end].to_string();
                    row_rule_below = Some(RuleKind::Cline(cline_arg));
                }
            } else {
                if !cleaned_row.is_empty() {
                    cleaned_row.push(' ');
                }
                cleaned_row.push_str(l);
            }
        }

        // Split cleaned_row by `&` respecting braces
        if cleaned_row.is_empty() && (row_rule_above.is_some() || row_rule_below.is_some()) {
            if let Some(last_row) = rows.last_mut() {
                if row_rule_above.is_some() {
                    last_row.rule_below = row_rule_above;
                } else if row_rule_below.is_some() {
                    last_row.rule_below = row_rule_below;
                }
            }
            continue;
        }

        let raw_cells = split_by_token(&cleaned_row, "&");
        let mut cells = Vec::new();

        for cell_str in raw_cells {
            let c_trimmed = cell_str.trim();
            let cell = parse_cell_content(c_trimmed);
            cells.push(cell);
        }

        // Pad or truncate cells to match expected_cols
        while cells.len() < expected_cols {
            cells.push(TableCell::default());
        }

        rows.push(TableRow {
            cells,
            rule_above: row_rule_above,
            rule_below: row_rule_below,
        });
    }

    (rows, has_booktabs)
}

fn parse_cell_content(raw: &str) -> TableCell {
    let mut cell = TableCell::default();

    // Check for \multicolumn{n}{spec}{content}
    if raw.starts_with("\\multicolumn{") {
        if let Some((n, spec, inner)) = parse_multicolumn(raw) {
            cell.col_span = n;
            cell.col_spec_override = Some(spec);
            cell.content = inner;
            return cell;
        }
    }

    // Check for \multirow{n}{width}{content}
    if raw.starts_with("\\multirow{") {
        if let Some((n, inner)) = parse_multirow(raw) {
            cell.row_span = n;
            cell.content = inner;
            return cell;
        }
    }

    cell.content = raw.to_string();
    cell
}

fn parse_multicolumn(raw: &str) -> Option<(usize, String, String)> {
    // \multicolumn{n}{spec}{content}
    let rest = raw.strip_prefix("\\multicolumn{")?;
    let close1 = rest.find('}')?;
    let n: usize = rest[..close1].trim().parse().ok()?;

    let rest2 = rest[close1 + 1..].trim_start();
    if !rest2.starts_with('{') {
        return None;
    }
    let rest2 = &rest2[1..];
    let close2 = rest2.find('}')?;
    let spec = rest2[..close2].trim().to_string();

    let rest3 = rest2[close2 + 1..].trim_start();
    if !rest3.starts_with('{') {
        return None;
    }
    let rest3 = &rest3[1..];
    let close3 = find_matching_brace(rest3)?;
    let content = rest3[..close3].to_string();

    Some((n, spec, content))
}

fn parse_multirow(raw: &str) -> Option<(usize, String)> {
    // \multirow{n}{width}{content}
    let rest = raw.strip_prefix("\\multirow{")?;
    let close1 = rest.find('}')?;
    let n: usize = rest[..close1].trim().parse().ok()?;

    let rest2 = rest[close1 + 1..].trim_start();
    if !rest2.starts_with('{') {
        return None;
    }
    let rest2 = &rest2[1..];
    let close2 = rest2.find('}')?;

    let rest3 = rest2[close2 + 1..].trim_start();
    if !rest3.starts_with('{') {
        return None;
    }
    let rest3 = &rest3[1..];
    let close3 = find_matching_brace(rest3)?;
    let content = rest3[..close3].to_string();

    Some((n, content))
}

fn find_matching_brace(s: &str) -> Option<usize> {
    let mut depth = 1;
    for (i, c) in s.char_indices() {
        if c == '{' {
            depth += 1;
        } else if c == '}' {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

/// Splits a string by a delimiter token (`\\` or `&`), ignoring occurrences inside `{...}` or `$...$`
fn split_by_token(s: &str, delim: &str) -> Vec<String> {
    let mut results = Vec::new();
    let mut cur = String::new();
    let mut brace_depth = 0;
    let mut in_math = false;
    let chars: Vec<char> = s.chars().collect();
    let delim_chars: Vec<char> = delim.chars().collect();
    let delim_len = delim_chars.len();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

        if ch == '$' && (i == 0 || chars[i - 1] != '\\') {
            in_math = !in_math;
            cur.push(ch);
            i += 1;
            continue;
        }

        if !in_math {
            if ch == '{' && (i == 0 || chars[i - 1] != '\\') {
                brace_depth += 1;
            } else if ch == '}' && (i == 0 || chars[i - 1] != '\\') && brace_depth > 0 {
                brace_depth -= 1;
            }
        }

        // Check if matching delimiter at top level
        if brace_depth == 0 && !in_math {
            let mut matches_delim = true;
            if i + delim_len <= chars.len() {
                for d in 0..delim_len {
                    if chars[i + d] != delim_chars[d] {
                        matches_delim = false;
                        break;
                    }
                }
            } else {
                matches_delim = false;
            }

            if matches_delim {
                results.push(cur.clone());
                cur.clear();
                i += delim_len;
                continue;
            }
        }

        cur.push(ch);
        i += 1;
    }

    if !cur.is_empty() || !results.is_empty() {
        results.push(cur);
    }

    results
}

/// Parse a full LaTeX table snippet (either `\begin{table}...\end{table}` or `\begin{tabular}...\end{tabular}`)
pub fn parse_latex_table(source: &str) -> Option<TableModel> {
    let mut caption = None;
    let mut label = None;

    // Check for \caption{...}
    if let Some(cap_idx) = source.find("\\caption{") {
        let rest = &source[cap_idx + 9..];
        if let Some(close) = find_matching_brace(rest) {
            caption = Some(rest[..close].trim().to_string());
        }
    }

    // Check for \label{...}
    if let Some(lbl_idx) = source.find("\\label{") {
        let rest = &source[lbl_idx + 7..];
        if let Some(close) = find_matching_brace(rest) {
            label = Some(rest[..close].trim().to_string());
        }
    }

    // Find \begin{tabular}{...} or \begin{tabularx}{...}
    let tabular_tag = "\\begin{tabular}";
    let begin_pos = source.find(tabular_tag)?;
    let after_begin = &source[begin_pos + tabular_tag.len()..];

    // Find specifier {spec}
    let spec_start = after_begin.find('{')?;
    let rest_spec = &after_begin[spec_start + 1..];
    let spec_close = find_matching_brace(rest_spec)?;
    let spec_str = &rest_spec[..spec_close];

    let columns = parse_col_specs(spec_str);
    let expected_cols = columns.len();

    // Find body up to \end{tabular}
    let body_start = spec_start + 1 + spec_close + 1;
    let rest_body = &after_begin[body_start..];
    let end_pos = rest_body.find("\\end{tabular}")?;
    let body = &rest_body[..end_pos];

    let (rows, has_booktabs) = parse_table_body(body, expected_cols);

    let mut model = TableModel {
        columns,
        rows,
        booktabs: has_booktabs,
        caption,
        label,
        source_span: Some(0..source.len()),
        source_file: None,
    };

    model.recompute_shadow_cells();
    Some(model)
}

/// Generate formatted LaTeX code from a `TableModel`
pub fn generate_latex_table(model: &TableModel) -> String {
    let mut col_spec = String::new();
    for (i, col) in model.columns.iter().enumerate() {
        if col.border_left {
            col_spec.push('|');
        }
        col_spec.push_str(&col.alignment.to_spec_char());
        if col.border_right && i == model.columns.len() - 1 {
            col_spec.push('|');
        } else if !col_spec.ends_with(' ') && i + 1 < model.columns.len() {
            col_spec.push(' ');
        }
    }

    let mut tabular = format!("\\begin{{tabular}}{{{}}}\n", col_spec);

    if model.booktabs {
        tabular.push_str("  \\toprule\n");
    } else {
        tabular.push_str("  \\hline\n");
    }

    // Compute column widths for pretty indentation
    let num_cols = model.columns.len();
    let mut col_widths = vec![4usize; num_cols];
    for row in &model.rows {
        for (c, cell) in row.cells.iter().enumerate() {
            if c < num_cols && !cell.is_shadow {
                let cell_len = cell.content.len();
                if cell_len > col_widths[c] {
                    col_widths[c] = cell_len;
                }
            }
        }
    }

    for (r, row) in model.rows.iter().enumerate() {
        let mut row_str = String::from("  ");
        let mut cell_tokens = Vec::new();

        for (c, cell) in row.cells.iter().enumerate() {
            if cell.is_shadow {
                continue;
            }

            let cell_repr = if cell.col_span > 1 {
                let align_spec = cell.col_spec_override.as_deref().unwrap_or("c");
                format!("\\multicolumn{{{}}}{{{}}}{{{}}}", cell.col_span, align_spec, cell.content)
            } else if cell.row_span > 1 {
                format!("\\multirow{{{}}}{{*}}{{{}}}", cell.row_span, cell.content)
            } else {
                cell.content.clone()
            };

            let padded = if c < num_cols {
                let pad_len = col_widths[c].saturating_sub(cell_repr.len());
                format!("{}{}", cell_repr, " ".repeat(pad_len))
            } else {
                cell_repr
            };

            cell_tokens.push(padded);
        }

        row_str.push_str(&cell_tokens.join(" & "));
        row_str.push_str(" \\\\\n");
        tabular.push_str(&row_str);

        if r == 0 && model.rows.len() > 1 {
            if model.booktabs {
                tabular.push_str("  \\midrule\n");
            } else {
                tabular.push_str("  \\hline\n");
            }
        }
    }

    if model.booktabs {
        tabular.push_str("  \\bottomrule\n\\end{tabular}");
    } else {
        tabular.push_str("  \\hline\n\\end{tabular}");
    }

    if model.caption.is_some() || model.label.is_some() {
        let mut wrapped = String::from("\\begin{table}[h!]\n  \\centering\n");
        if let Some(ref cap) = model.caption {
            wrapped.push_str(&format!("  \\caption{{{}}}\n", cap));
        }
        if let Some(ref lbl) = model.label {
            wrapped.push_str(&format!("  \\label{{{}}}\n", lbl));
        }
        for line in tabular.lines() {
            wrapped.push_str(&format!("  {}\n", line));
        }
        wrapped.push_str("\\end{table}");
        wrapped
    } else {
        tabular
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_col_specs() {
        let specs = parse_col_specs("|l|c|r|");
        assert_eq!(specs.len(), 3);
        assert_eq!(specs[0].alignment, ColAlign::Left);
        assert!(specs[0].border_left);
        assert_eq!(specs[1].alignment, ColAlign::Center);
        assert_eq!(specs[2].alignment, ColAlign::Right);
        assert!(specs[2].border_right);
    }

    #[test]
    fn test_parse_and_generate_roundtrip() {
        let latex = "\\begin{tabular}{l c r}\n  \\toprule\n  Name & Age & City \\\\\n  \\midrule\n  Alice & 25 & NYC \\\\\n  Bob & 30 & LA \\\\\n  \\bottomrule\n\\end{tabular}";
        let model = parse_latex_table(latex).expect("Failed to parse table");
        assert_eq!(model.columns.len(), 3);
        assert_eq!(model.rows.len(), 3);
        assert_eq!(model.rows[0].cells[0].content, "Name");
        assert_eq!(model.rows[1].cells[0].content, "Alice");
        assert_eq!(model.rows[2].cells[2].content, "LA");
        assert!(model.booktabs);

        let generated = generate_latex_table(&model);
        assert!(generated.contains("\\begin{tabular}{l c r}"));
        assert!(generated.contains("Alice"));
        assert!(generated.contains("\\bottomrule"));
    }

    #[test]
    fn test_parse_multicolumn() {
        let latex = "\\begin{tabular}{c c}\n  \\toprule\n  \\multicolumn{2}{c}{Header Merged} \\\\\n  \\midrule\n  A & B \\\\\n  \\bottomrule\n\\end{tabular}";
        let model = parse_latex_table(latex).expect("Failed to parse table");
        assert_eq!(model.rows[0].cells[0].col_span, 2);
        assert_eq!(model.rows[0].cells[0].content, "Header Merged");
        assert!(model.rows[0].cells[1].is_shadow);
    }

    #[test]
    fn test_table_with_caption_and_label() {
        let latex = "\\begin{table}[h!]\n  \\centering\n  \\caption{Experiment Results}\n  \\label{tbl:results}\n  \\begin{tabular}{l c}\n    \\toprule\n    Test & Score \\\\\n    \\midrule\n    1 & 100 \\\\\n    \\bottomrule\n  \\end{tabular}\n\\end{table}";
        let model = parse_latex_table(latex).expect("Failed to parse table with caption");
        assert_eq!(model.caption.as_deref(), Some("Experiment Results"));
        assert_eq!(model.label.as_deref(), Some("tbl:results"));

        let out = generate_latex_table(&model);
        assert!(out.contains("\\caption{Experiment Results}"));
        assert!(out.contains("\\label{tbl:results}"));
    }
}
