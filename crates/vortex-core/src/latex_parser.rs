use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::LazyLock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct LabelItem {
    pub key: String,
    pub file: String,
    pub filename: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct CitationItem {
    #[serde(rename = "type")]
    pub item_type: String, // "citation"
    pub keys: Vec<String>,
    pub file: String,
    pub filename: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct RefItem {
    #[serde(rename = "type")]
    pub item_type: String, // "ref"
    pub key: String,
    pub file: String,
    pub filename: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct BibRef {
    #[serde(rename = "type")]
    pub item_type: String, // "bibliography"
    pub file: String,
    #[serde(rename = "sourceFile")]
    pub source_file: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SectionItem {
    #[serde(rename = "type")]
    pub section_type: Option<String>,
    pub level: usize,
    pub title: String,
    pub file: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct TodoItem {
    pub tag: String,
    pub text: String,
    pub file: String,
    pub filename: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct TableItem {
    #[serde(rename = "type", default = "default_table_type")]
    pub item_type: String, // "table"
    pub file: String,
    pub filename: String,
    pub line: usize,
    pub caption: Option<String>,
    pub label: Option<String>,
    pub byte_start: usize,
    pub byte_end: usize,
}

fn default_table_type() -> String {
    "table".to_string()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ParsedLatex {
    pub labels: Vec<LabelItem>,
    pub citations: Vec<CitationItem>,
    pub refs: Vec<RefItem>,
    pub bibliographies: Vec<BibRef>,
    pub sections: Vec<SectionItem>,
    pub todos: Vec<TodoItem>,
    pub tables: Vec<TableItem>,
}

static SECTION_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\\(part|chapter|section|subsection|subsubsection|paragraph|subparagraph)\*?\s*\{([^}]+)\}").unwrap()
});

static LABEL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\\label\s*\{([^}]+)\}").unwrap()
});

static CAPTION_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\\caption\s*\{([^}]+)\}").unwrap()
});

static CITE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\\(?:cite|citep|citet|autocite|parencite|textcite|citealt|citealp|citeauthor|citeyear|nocite|footcite|fullcite)\*?(?:\[[^\]]*\])*\s*\{([^}]+)\}").unwrap()
});

static REF_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\\(?:ref|eqref|cref|autoref|pageref)\s*\{([^}]+)\}").unwrap()
});

static BIB_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\\(?:bibliography|addbibresource)\s*\{([^}]+)\}").unwrap()
});

static TODO_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:^|[^\\])(?:%|//|#|\*)\s*(\[\s*\]|\[x\]|TODO|FIXME|NOTE|BUG|HACK|XXX|IDEA|OPTIMIZE|REVIEW)\s*(?::|-)?\s*(.*)").unwrap()
});

/// What a label points at, guessed from its prefix (`fig:`, `eq:`, `sec:` ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum LabelKind {
    Section,
    Equation,
    Table,
    Figure,
    Other,
}

pub fn label_kind(key: &str) -> LabelKind {
    let prefix = key.split(':').next().unwrap_or("").to_lowercase();
    match prefix.as_str() {
        "sec" | "sub" | "subsec" | "chap" | "ch" | "part" | "app" => LabelKind::Section,
        "eq" | "eqn" => LabelKind::Equation,
        "tab" | "tbl" | "table" => LabelKind::Table,
        "fig" | "figure" | "subfig" => LabelKind::Figure,
        _ => LabelKind::Other,
    }
}

pub fn get_section_level(cmd: &str) -> usize {
    match cmd {
        "part" => 0,
        "chapter" => 1,
        "section" => 2,
        "subsection" => 3,
        "subsubsection" => 4,
        "paragraph" => 5,
        "subparagraph" => 6,
        _ => 2,
    }
}

pub fn parse_latex_file(file_path: &str, content: &str) -> ParsedLatex {
    let mut result = ParsedLatex::default();
    let filename = Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    for (index, line) in content.lines().enumerate() {
        let line_num = index + 1;

        // Extract sections
        for cap in SECTION_REGEX.captures_iter(line) {
            if let (Some(cmd), Some(title)) = (cap.get(1), cap.get(2)) {
                let level = get_section_level(cmd.as_str());
                result.sections.push(SectionItem {
                    section_type: Some("section".to_string()),
                    level,
                    title: title.as_str().trim().to_string(),
                    file: file_path.to_string(),
                    line: line_num,
                });
            }
        }

        // Extract labels
        for cap in LABEL_REGEX.captures_iter(line) {
            if let Some(key_match) = cap.get(1) {
                let key = key_match.as_str().trim().to_string();
                if !key.is_empty() && !result.labels.iter().any(|l| l.key == key) {
                    result.labels.push(LabelItem {
                        key,
                        file: file_path.to_string(),
                        filename: filename.clone(),
                        line: line_num,
                    });
                }
            }
        }

        // Extract citations
        for cap in CITE_REGEX.captures_iter(line) {
            if let Some(keys_match) = cap.get(1) {
                let keys: Vec<String> = keys_match
                    .as_str()
                    .split(',')
                    .map(|k| k.trim().to_string())
                    .filter(|k| !k.is_empty())
                    .collect();
                if !keys.is_empty() {
                    result.citations.push(CitationItem {
                        item_type: "citation".to_string(),
                        keys,
                        file: file_path.to_string(),
                        filename: filename.clone(),
                        line: line_num,
                    });
                }
            }
        }

        // Extract refs
        for cap in REF_REGEX.captures_iter(line) {
            if let Some(key_match) = cap.get(1) {
                let key = key_match.as_str().trim().to_string();
                if !key.is_empty() {
                    result.refs.push(RefItem {
                        item_type: "ref".to_string(),
                        key,
                        file: file_path.to_string(),
                        filename: filename.clone(),
                        line: line_num,
                    });
                }
            }
        }

        // Extract bibliographies
        for cap in BIB_REGEX.captures_iter(line) {
            if let Some(bib_match) = cap.get(1) {
                for bib_file in bib_match.as_str().split(',') {
                    let bib_file = bib_file.trim().to_string();
                    if !bib_file.is_empty() {
                        result.bibliographies.push(BibRef {
                            item_type: "bibliography".to_string(),
                            file: bib_file,
                            source_file: file_path.to_string(),
                            line: line_num,
                        });
                    }
                }
            }
        }

        // Extract TODO items / comments
        if let Some(cap) = TODO_REGEX.captures(line) {
            let raw_tag = cap.get(1).map(|m| m.as_str().trim()).unwrap_or("TODO");
            let tag = match raw_tag {
                s if s.starts_with('[') && s.ends_with(']') => {
                    if s.contains('x') || s.contains('X') {
                        "DONE".to_string()
                    } else {
                        "TODO".to_string()
                    }
                }
                other => other.to_uppercase(),
            };

            let text = cap
                .get(2)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();

            let final_text = if text.is_empty() {
                format!("{} (line {})", tag, line_num)
            } else {
                text
            };

            result.todos.push(TodoItem {
                tag,
                text: final_text,
                file: file_path.to_string(),
                filename: filename.clone(),
                line: line_num,
            });
        }
    }

    // Extract tables
    let mut search_idx = 0;
    let mut covered_ranges: Vec<(usize, usize)> = Vec::new();

    // 1. First search for \begin{table} ... \end{table}
    while let Some(table_pos) = content[search_idx..].find("\\begin{table") {
        let abs_start = search_idx + table_pos;
        if let Some(table_end_rel) = content[abs_start..].find("\\end{table}") {
            let abs_end = abs_start + table_end_rel + "\\end{table}".len();
            let snippet = &content[abs_start..abs_end];
            let line_num = content[..abs_start].chars().filter(|&c| c == '\n').count() + 1;

            let mut caption = None;
            let mut label = None;
            if let Some(cap_m) = CAPTION_REGEX.captures(snippet) {
                caption = cap_m.get(1).map(|m| m.as_str().trim().to_string());
            }
            if let Some(lbl_m) = LABEL_REGEX.captures(snippet) {
                label = lbl_m.get(1).map(|m| m.as_str().trim().to_string());
            }

            result.tables.push(TableItem {
                item_type: "table".to_string(),
                file: file_path.to_string(),
                filename: filename.clone(),
                line: line_num,
                caption,
                label,
                byte_start: abs_start,
                byte_end: abs_end,
            });

            covered_ranges.push((abs_start, abs_end));
            search_idx = abs_end;
        } else {
            search_idx = abs_start + "\\begin{table".len();
        }
    }

    // 2. Then search for standalone \begin{tabular} ... \end{tabular} not within \begin{table}
    search_idx = 0;
    while let Some(tab_pos) = content[search_idx..].find("\\begin{tabular") {
        let abs_start = search_idx + tab_pos;
        let is_covered = covered_ranges.iter().any(|(s, e)| abs_start >= *s && abs_start < *e);
        if is_covered {
            search_idx = abs_start + "\\begin{tabular".len();
            continue;
        }

        if let Some(tab_end_rel) = content[abs_start..].find("\\end{tabular}") {
            let abs_end = abs_start + tab_end_rel + "\\end{tabular}".len();
            let line_num = content[..abs_start].chars().filter(|&c| c == '\n').count() + 1;

            result.tables.push(TableItem {
                item_type: "table".to_string(),
                file: file_path.to_string(),
                filename: filename.clone(),
                line: line_num,
                caption: None,
                label: None,
                byte_start: abs_start,
                byte_end: abs_end,
            });

            search_idx = abs_end;
        } else {
            search_idx = abs_start + "\\begin{tabular".len();
        }
    }

    result
}

