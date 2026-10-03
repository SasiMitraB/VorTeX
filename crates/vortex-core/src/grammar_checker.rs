/// Grammar checker service.
///
/// Runs `harper-core` on preprocessed LaTeX text and maps the resulting lint
/// spans back to (row, col) positions in the original `.tex` buffer.
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex, OnceLock};

use harper_core::{
    linting::{LintGroup, Linter},
    parsers::PlainEnglish,
    spell::FstDictionary,
    Dialect, Document,
};

use crate::bibtex_parser::BibEntryItem;
use crate::grammar_preprocess::{inline_math_ranges, preprocess_latex};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// English spelling conventions to check against.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum GrammarDialect {
    American,
    #[default]
    British,
    Canadian,
    Australian,
    Indian,
}

impl From<GrammarDialect> for Dialect {
    fn from(d: GrammarDialect) -> Self {
        match d {
            GrammarDialect::American => Dialect::American,
            GrammarDialect::British => Dialect::British,
            GrammarDialect::Canadian => Dialect::Canadian,
            GrammarDialect::Australian => Dialect::Australian,
            GrammarDialect::Indian => Dialect::Indian,
        }
    }
}

/// A grammar issue found in the original `.tex` file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
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
///
/// The cleaned text is linted paragraph by paragraph and each paragraph's lints are
/// cached by content, so after an edit only the paragraphs that changed are linted again.
pub fn run_grammar_check(
    orig_content: &str,
    bib: &HashMap<String, BibEntryItem>,
    dialect: GrammarDialect,
) -> Vec<GrammarDiagnostic> {
    // 1. Pre-process LaTeX → clean English + source map
    let (cleaned, source_map) = preprocess_latex(orig_content, bib);
    if cleaned.trim().is_empty() {
        return Vec::new();
    }

    // 2. Lint each paragraph of the cleaned text, reusing cached results
    let mut checker = checker().lock().unwrap_or_else(|e| e.into_inner());
    let lints = checker.lint(&cleaned, dialect);
    if lints.is_empty() {
        return Vec::new();
    }

    // 3. Map each lint span back to original positions
    let lines = LineIndex::new(orig_content);
    let math = inline_math_ranges(orig_content);
    let mut diagnostics = Vec::with_capacity(lints.len());

    for (clean_byte_start, clean_byte_end, message) in lints {
        let orig_byte_start = source_map.to_orig(clean_byte_start);
        let orig_byte_end = source_map.to_orig(clean_byte_end).max(orig_byte_start + 1);
        let orig_byte_end = orig_byte_end.min(orig_content.len());
        if math.iter().any(|m| m.contains(&orig_byte_start)) {
            continue;
        }

        // Convert orig byte offsets to (row, col_char)
        let (row_start, col_start) = lines.row_col(orig_byte_start);
        let (row_end, col_end) = lines.row_col(orig_byte_end);

        // Only emit single-line diagnostics for now (multi-line is rare for grammar):
        // clamp to the end of the starting line.
        let col_end = if row_start == row_end { col_end } else { lines.line_chars(row_start) };

        diagnostics.push(GrammarDiagnostic {
            row: row_start,
            col_start,
            col_end,
            message: message.to_string(),
        });
    }

    diagnostics
}

// ---------------------------------------------------------------------------
// Incremental linting
// ---------------------------------------------------------------------------

/// Paragraph caches are trimmed once they hold more entries than this…
const CACHE_LIMIT: usize = 4096;
/// …down to the paragraphs seen in this many recent checks.
const CACHE_KEEP_RUNS: u64 = 16;

/// A lint in one paragraph: byte span relative to the paragraph, and its message.
type ParagraphLint = (usize, usize, Arc<str>);

struct CachedParagraph {
    lints: Arc<[ParagraphLint]>,
    last_used: u64,
}

/// One linter per dialect, kept for the life of the process: Harper loads its dictionary and
/// rules once, and its own sentence caches stay warm between checks.
struct Checker {
    linters: HashMap<GrammarDialect, LintGroup>,
    paragraphs: HashMap<(u64, GrammarDialect), CachedParagraph>,
    run: u64,
}

fn checker() -> &'static Mutex<Checker> {
    static CHECKER: OnceLock<Mutex<Checker>> = OnceLock::new();
    CHECKER.get_or_init(|| {
        Mutex::new(Checker { linters: HashMap::new(), paragraphs: HashMap::new(), run: 0 })
    })
}

impl Checker {
    /// Lints of `cleaned` as (start byte, end byte, message), in document order.
    fn lint(&mut self, cleaned: &str, dialect: GrammarDialect) -> Vec<(usize, usize, Arc<str>)> {
        self.run += 1;
        let run = self.run;
        let mut out = Vec::new();

        for (offset, paragraph) in paragraphs(cleaned) {
            let key = (hash(paragraph), dialect);
            let lints = match self.paragraphs.get_mut(&key) {
                Some(cached) => {
                    cached.last_used = run;
                    cached.lints.clone()
                }
                None => {
                    let linter = self.linters.entry(dialect).or_insert_with(|| new_linter(dialect));
                    let lints = lint_paragraph(linter, paragraph);
                    self.paragraphs.insert(key, CachedParagraph { lints: lints.clone(), last_used: run });
                    lints
                }
            };
            out.extend(lints.iter().map(|(s, e, m)| (offset + s, offset + e, m.clone())));
        }

        if self.paragraphs.len() > CACHE_LIMIT {
            self.paragraphs.retain(|_, p| run - p.last_used < CACHE_KEEP_RUNS);
        }
        out
    }
}

fn new_linter(dialect: GrammarDialect) -> LintGroup {
    let mut linter = LintGroup::new_curated(FstDictionary::curated(), dialect.into());
    // Whitespace in LaTeX source (indentation, alignment) is not prose.
    linter.config.set_rule_enabled("Spaces", false);
    linter.config.set_rule_enabled("NoFrenchSpaces", false);
    linter
}

fn lint_paragraph(linter: &mut LintGroup, paragraph: &str) -> Arc<[ParagraphLint]> {
    let document = Document::new_curated(paragraph, &PlainEnglish);
    let lints = linter.lint(&document);
    if lints.is_empty() {
        return Arc::new([]);
    }
    // Harper spans count chars; the source map works in bytes.
    let char_bytes: Vec<usize> = paragraph.char_indices().map(|(b, _)| b).collect();
    let to_byte = |c: usize| char_bytes.get(c).copied().unwrap_or(paragraph.len());
    lints
        .into_iter()
        .map(|l| (to_byte(l.span.start), to_byte(l.span.end), Arc::from(l.message)))
        .collect()
}

/// Non-blank paragraphs of `text` (split at blank lines) with their byte offsets.
fn paragraphs(text: &str) -> impl Iterator<Item = (usize, &str)> {
    let mut start = 0;
    let ends = text.match_indices("\n\n").map(|(i, _)| i).chain(std::iter::once(text.len()));
    ends.filter_map(move |end| {
        let paragraph = (start, &text[start..end]);
        start = end + 2;
        (!paragraph.1.trim().is_empty()).then_some(paragraph)
    })
}

fn hash(text: &str) -> u64 {
    let mut h = DefaultHasher::new();
    text.hash(&mut h);
    h.finish()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Byte offsets of line starts, to turn byte offsets into (row, char column).
struct LineIndex<'a> {
    text: &'a str,
    starts: Vec<usize>,
}

impl<'a> LineIndex<'a> {
    fn new(text: &'a str) -> Self {
        let starts = std::iter::once(0).chain(text.match_indices('\n').map(|(i, _)| i + 1)).collect();
        Self { text, starts }
    }

    /// 0-indexed row and char column of `byte_offset`.
    fn row_col(&self, byte_offset: usize) -> (usize, usize) {
        let byte_offset = byte_offset.min(self.text.len());
        let row = self.starts.partition_point(|&s| s <= byte_offset) - 1;
        let col_bytes = &self.text.as_bytes()[self.starts[row]..byte_offset];
        let col_chars = std::str::from_utf8(col_bytes)
            .map(|s| s.chars().count())
            .unwrap_or(col_bytes.len());
        (row, col_chars)
    }

    /// Length of `row` in chars, without its line break.
    fn line_chars(&self, row: usize) -> usize {
        let start = self.starts[row];
        let end = self.starts.get(row + 1).map_or(self.text.len(), |&s| s - 1);
        self.text[start..end].trim_end_matches('\r').chars().count()
    }
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
        let diags = run_grammar_check(tex, &HashMap::new(), GrammarDialect::British);
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
        let diags = run_grammar_check(tex, &HashMap::new(), GrammarDialect::British);
        for d in diags {
            assert!(
                !d.message.contains("x + y = z"),
                "Equation environment should not generate diagnostics"
            );
        }
    }

    #[test]
    fn dialect_changes_spelling_rules() {
        let tex = "The colour of the sky is blue.\n";
        let flagged = |d| run_grammar_check(tex, &HashMap::new(), d).iter().any(|x| x.col_start == 4);
        assert!(!flagged(GrammarDialect::British));
        assert!(flagged(GrammarDialect::American));
    }

    #[test]
    fn paragraphs_split_at_blank_lines() {
        let text = "One two.\n\nThree\nfour.\n\n\n\nFive.\n\n";
        let got: Vec<_> = paragraphs(text).collect();
        assert_eq!(got, vec![(0, "One two."), (10, "Three\nfour."), (25, "Five.")]);
        for (offset, p) in got {
            assert_eq!(&text[offset..offset + p.len()], p);
        }
    }

    #[test]
    fn cached_paragraphs_keep_their_positions_after_an_edit() {
        let first = "Intro text is fine.\n\nWe recieve teh data.\n";
        let edited = "Intro text is fine and longer now.\nAnother line.\n\nWe recieve teh data.\n";
        let at = |tex| {
            run_grammar_check(tex, &HashMap::new(), GrammarDialect::British)
                .into_iter()
                .filter(|d| d.row >= 2)
                .map(|d| (d.row, d.col_start, d.col_end))
                .collect::<Vec<_>>()
        };
        let before = at(first);
        assert!(!before.is_empty());
        // The second paragraph is unchanged (served from the cache) but moved down a line.
        let after = at(edited);
        assert_eq!(after, before.iter().map(|&(r, s, e)| (r + 1, s, e)).collect::<Vec<_>>());
    }

    #[test]
    fn latex_layout_and_math_are_not_flagged() {
        let tex = "\\begin{tabular}{l c r}\n    Method & Score & Time \\\\\n\\end{tabular}\nEnergy is $E = mc^2$ here. See Section~\\ref{sec:a} and more.\n";
        let diags = run_grammar_check(tex, &HashMap::new(), GrammarDialect::British);
        assert!(diags.is_empty(), "{diags:#?}");
    }
}
