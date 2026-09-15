/// Grammar checker service.
///
/// Runs `harper-core` on preprocessed LaTeX text and maps the resulting lint
/// spans back to (row, col) positions in the original `.tex` buffer.
use std::collections::HashMap;

use harper_core::{
    linting::{LintGroup, Linter},
    parsers::PlainEnglish,
    spell::FstDictionary,
    Dialect, Document,
};

use crate::services::bibtex_parser::BibEntryItem;
use crate::services::grammar_preprocess::preprocess_latex;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A grammar issue found in the original `.tex` file.
#[derive(Debug, Clone)]
pub struct GrammarDiagnostic {
    /// 0-indexed row in the editor's line array
    pub row: usize,
    /// 0-indexed char column where the underline starts
    pub col_start: usize,
    /// 0-indexed char column where the underline ends (exclusive)
    pub col_end: usize,
    /// Human-readable description from Harper
    pub message: String,
}

// ---------------------------------------------------------------------------
// Run
// ---------------------------------------------------------------------------

/// Check `orig_content` (raw `.tex`) for grammar issues.
///
/// `bib` should be the project's full BibTeX entry map (key → BibEntryItem),
/// used to resolve `\cite` commands into author-year text.
pub fn run_grammar_check(
    orig_content: &str,
    bib: &HashMap<String, BibEntryItem>,
) -> Vec<GrammarDiagnostic> {
    // 1. Pre-process LaTeX → clean English + source map
    let (cleaned, source_map) = preprocess_latex(orig_content, bib);
    if cleaned.trim().is_empty() {
        return Vec::new();
    }

    // 2. Curated dictionary is an Arc<FstDictionary>
    let dict = FstDictionary::curated();

    // 3. Create document
    let parser = PlainEnglish;
    let document = Document::new_curated(&cleaned, &parser);

    // 4. Create linter with British English
    let mut linter = LintGroup::new_curated(dict, Dialect::British);

    // 5. Lint
    let lints = linter.lint(&document);
    if lints.is_empty() {
        return Vec::new();
    }

    // 6. Map each lint span back to original positions
    let orig_bytes = orig_content.as_bytes();
    let mut diagnostics = Vec::with_capacity(lints.len());

    for lint in lints {
        let span = lint.span;
        // Convert char-index span to byte offsets in the cleaned string
        let clean_byte_start = char_index_to_byte(&cleaned, span.start);
        let clean_byte_end = char_index_to_byte(&cleaned, span.end);

        let orig_byte_start = source_map.to_orig(clean_byte_start);
        let orig_byte_end = source_map.to_orig(clean_byte_end).max(orig_byte_start + 1);
        let orig_byte_end = orig_byte_end.min(orig_content.len());

        // Convert orig byte offsets to (row, col_char)
        let (row_start, col_start) = byte_to_row_col(orig_bytes, orig_byte_start);
        let (row_end, col_end) = byte_to_row_col(orig_bytes, orig_byte_end);

        // Only emit single-line diagnostics for now (multi-line is rare for grammar)
        let (final_row, final_col_start, final_col_end) = if row_start == row_end {
            (row_start, col_start, col_end)
        } else {
            // Clamp to end of starting line
            let line_end = orig_content
                .lines()
                .nth(row_start)
                .map(|l| l.chars().count())
                .unwrap_or(col_end);
            (row_start, col_start, line_end)
        };

        let message = lint.message.to_string();

        diagnostics.push(GrammarDiagnostic {
            row: final_row,
            col_start: final_col_start,
            col_end: final_col_end,
            message,
        });
    }

    diagnostics
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Convert a char index (as produced by harper's span) to a byte offset
/// in a UTF-8 string.
fn char_index_to_byte(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(b, _)| b)
        .unwrap_or(s.len())
}

/// Convert a byte offset in a UTF-8 buffer to (0-indexed row, 0-indexed char col).
fn byte_to_row_col(bytes: &[u8], byte_offset: usize) -> (usize, usize) {
    let byte_offset = byte_offset.min(bytes.len());
    let prefix = &bytes[..byte_offset];
    let row = prefix.iter().filter(|&&b| b == b'\n').count();
    let line_start = prefix
        .iter()
        .rposition(|&b| b == b'\n')
        .map(|p| p + 1)
        .unwrap_or(0);
    let col_bytes = &bytes[line_start..byte_offset];
    let col_chars = std::str::from_utf8(col_bytes)
        .map(|s| s.chars().count())
        .unwrap_or(col_bytes.len());
    (row, col_chars)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grammar_error_detected() {
        let tex = r#"\documentclass{article}
\begin{document}
ew feature here.
dis is a newass feature where i can just give it asentence and it checks if there's any errors or anything in the grammer or
\end{document}"#;
        let diags = run_grammar_check(tex, &HashMap::new());
        let lines: Vec<&str> = tex.lines().collect();
        for d in &diags {
            let line = lines.get(d.row).unwrap_or(&"");
            let flagged: String = line.chars().skip(d.col_start).take(d.col_end - d.col_start).collect();
            println!("Diagnostic: row={}, col_start={}, col_end={}, msg={}", d.row, d.col_start, d.col_end, d.message);
            println!("Line text: {:?}", line);
            println!("Flagged text: {:?}", flagged);
        }
        assert!(!diags.is_empty());
    }

    #[test]
    fn test_math_and_equation_ignored() {
        let tex = r#"
Here is an equation:
\begin{equation}
x + y = z
\end{equation}
And inline $a^2 + b^2 = c^2$ holds.
"#;
        let diags = run_grammar_check(tex, &HashMap::new());
        for d in diags {
            assert!(
                !d.message.contains("x + y = z"),
                "Equation environment should not generate diagnostics"
            );
        }
    }
}
