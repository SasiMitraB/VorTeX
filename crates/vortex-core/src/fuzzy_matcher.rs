use crate::bibtex_parser::BibEntryItem;
use crate::latex_parser::LabelItem;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzyMatchResult {
    pub obj: serde_json::Value,
    #[serde(default)]
    pub score: f64,
}

pub struct Matcher {
    matcher: SkimMatcherV2,
}

impl Default for Matcher {
    fn default() -> Self {
        Self {
            matcher: SkimMatcherV2::default().smart_case(),
        }
    }
}

impl Matcher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn search_labels(
        &self,
        query: &str,
        labels: &[LabelItem],
        current_file: Option<&str>,
    ) -> Vec<FuzzyMatchResult> {
        if query.trim().is_empty() {
            let mut results: Vec<FuzzyMatchResult> = labels
                .iter()
                .take(15)
                .map(|l| FuzzyMatchResult {
                    obj: serde_json::to_value(l).unwrap_or(serde_json::Value::Null),
                    score: if current_file.map_or(false, |f| f == l.file) { 100.0 } else { 0.0 },
                })
                .collect();

            if let Some(cf) = current_file {
                results.sort_by(|a, b| {
                    let a_cf = a.obj.get("file").and_then(|f| f.as_str()) == Some(cf);
                    let b_cf = b.obj.get("file").and_then(|f| f.as_str()) == Some(cf);
                    match (a_cf, b_cf) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => std::cmp::Ordering::Equal,
                    }
                });
            }

            return results.into_iter().take(10).collect();
        }

        let mut scored: Vec<(LabelItem, i64)> = Vec::new();

        for label in labels {
            // Match against key first
            let mut score = self.matcher.fuzzy_match(&label.key, query);

            // If no match on key, try filename
            if score.is_none() {
                score = self.matcher.fuzzy_match(&label.filename, query).map(|s| s / 2);
            }

            if let Some(s) = score {
                // Boost score if it is in the active file
                let final_score = if current_file.map_or(false, |cf| cf == label.file) {
                    s + 500
                } else {
                    s
                };
                scored.push((label.clone(), final_score));
            }
        }

        scored.sort_by(|a, b| b.1.cmp(&a.1));

        scored
            .into_iter()
            .take(10)
            .map(|(l, s)| FuzzyMatchResult {
                obj: serde_json::to_value(&l).unwrap_or(serde_json::Value::Null),
                score: s as f64,
            })
            .collect()
    }

    pub fn search_bib_entries(
        &self,
        query: &str,
        bibentries: &[BibEntryItem],
    ) -> Vec<FuzzyMatchResult> {
        let query_clean = query.trim();

        if query_clean.is_empty() {
            return bibentries
                .iter()
                .take(50)
                .map(|b| FuzzyMatchResult {
                    obj: serde_json::to_value(b).unwrap_or(serde_json::Value::Null),
                    score: 0.0,
                })
                .collect();
        }

        let mut scored: Vec<(BibEntryItem, i64)> = Vec::new();

        for entry in bibentries {
            let mut best_score: Option<i64> = None;

            // 1. Key match (highest priority, +200 bonus)
            if let Some(s) = self.matcher.fuzzy_match(&entry.key, query_clean) {
                best_score = Some(s + 200);
            }

            if let Some(ref fields) = entry.fields {
                // 2. Title match (+100)
                if let Some(ref title) = fields.title {
                    if let Some(s) = self.matcher.fuzzy_match(title, query_clean) {
                        best_score = Some(best_score.map_or(s + 100, |curr| curr.max(s + 100)));
                    }
                }
                // 3. Author match (+100)
                if let Some(ref author) = fields.author {
                    if let Some(s) = self.matcher.fuzzy_match(author, query_clean) {
                        best_score = Some(best_score.map_or(s + 100, |curr| curr.max(s + 100)));
                    }
                }
                // 4. Year match (+60)
                if let Some(ref year) = fields.year {
                    if let Some(s) = self.matcher.fuzzy_match(year, query_clean) {
                        best_score = Some(best_score.map_or(s + 60, |curr| curr.max(s + 60)));
                    }
                }
                // 5. Keywords match (+80)
                if let Some(ref keywords) = fields.keywords {
                    if let Some(s) = self.matcher.fuzzy_match(keywords, query_clean) {
                        best_score = Some(best_score.map_or(s + 80, |curr| curr.max(s + 80)));
                    }
                }
                // 6. Journal / Booktitle match (+50)
                if let Some(ref journal) = fields.journal {
                    if let Some(s) = self.matcher.fuzzy_match(journal, query_clean) {
                        best_score = Some(best_score.map_or(s + 50, |curr| curr.max(s + 50)));
                    }
                }
                if let Some(ref booktitle) = fields.booktitle {
                    if let Some(s) = self.matcher.fuzzy_match(booktitle, query_clean) {
                        best_score = Some(best_score.map_or(s + 50, |curr| curr.max(s + 50)));
                    }
                }
                // 7. Eprint / Note / Publisher (+30)
                if let Some(ref eprint) = fields.eprint {
                    if let Some(s) = self.matcher.fuzzy_match(eprint, query_clean) {
                        best_score = Some(best_score.map_or(s + 30, |curr| curr.max(s + 30)));
                    }
                }
                if let Some(ref note) = fields.note {
                    if let Some(s) = self.matcher.fuzzy_match(note, query_clean) {
                        best_score = Some(best_score.map_or(s + 30, |curr| curr.max(s + 30)));
                    }
                }
            }

            if let Some(s) = best_score {
                scored.push((entry.clone(), s));
            }
        }

        scored.sort_by(|a, b| b.1.cmp(&a.1));

        scored
            .into_iter()
            .take(50)
            .map(|(b, s)| FuzzyMatchResult {
                obj: serde_json::to_value(&b).unwrap_or(serde_json::Value::Null),
                score: s as f64,
            })
            .collect()
    }
}

pub fn filter_by_prefix<'a>(labels: &'a [LabelItem], prefix: &str) -> Vec<&'a LabelItem> {
    if prefix.is_empty() {
        labels.iter().collect()
    } else {
        labels.iter().filter(|l| l.key.starts_with(prefix)).collect()
    }
}

pub fn extract_prefix(query: &str) -> Option<&str> {
    if let Some(idx) = query.find(':') {
        Some(&query[..=idx])
    } else {
        None
    }
}
