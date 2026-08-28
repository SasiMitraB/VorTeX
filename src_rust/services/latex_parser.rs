use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::LazyLock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LabelItem {
    pub key: String,
    pub file: String,
    pub filename: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CitationItem {
    #[serde(rename = "type")]
    pub item_type: String, // "citation"
    pub keys: Vec<String>,
    pub file: String,
    pub filename: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RefItem {
    #[serde(rename = "type")]
    pub item_type: String, // "ref"
    pub key: String,
    pub file: String,
    pub filename: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BibRef {
    #[serde(rename = "type")]
    pub item_type: String, // "bibliography"
    pub file: String,
    #[serde(rename = "sourceFile")]
    pub source_file: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SectionItem {
    #[serde(rename = "type")]
    pub section_type: Option<String>,
    pub level: usize,
    pub title: String,
    pub file: String,
    pub line: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParsedLatex {
    pub labels: Vec<LabelItem>,
    pub citations: Vec<CitationItem>,
    pub refs: Vec<RefItem>,
    pub bibliographies: Vec<BibRef>,
    pub sections: Vec<SectionItem>,
}

static SECTION_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\\(part|chapter|section|subsection|subsubsection|paragraph|subparagraph)\*?\s*\{([^}]+)\}").unwrap()
});

static LABEL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\\label\s*\{([^}]+)\}").unwrap()
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
    }

    result
}

