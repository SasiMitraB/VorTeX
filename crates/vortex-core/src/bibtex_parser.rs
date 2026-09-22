use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct BibFields {
    pub title: Option<String>,
    pub author: Option<String>,
    pub year: Option<String>,
    pub journal: Option<String>,
    pub booktitle: Option<String>,
    pub publisher: Option<String>,
    pub doi: Option<String>,
    pub url: Option<String>,
    pub keywords: Option<String>,
    pub abstract_text: Option<String>,
    pub note: Option<String>,
    pub volume: Option<String>,
    pub number: Option<String>,
    pub pages: Option<String>,
    pub eprint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct BibEntryItem {
    #[serde(rename = "type")]
    pub item_type: String, // "bibentry"
    pub key: String,
    #[serde(rename = "entryType")]
    pub entry_type: Option<String>,
    pub file: String,
    pub filename: String,
    #[serde(default)]
    pub fields: Option<BibFields>,
}

pub fn clean_field(val: &str) -> String {
    let mut s = val.trim();

    // Strip outer quotes or braces
    while (s.starts_with('{') && s.ends_with('}')) || (s.starts_with('"') && s.ends_with('"')) {
        if s.len() >= 2 {
            s = s[1..s.len() - 1].trim();
        } else {
            break;
        }
    }

    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&next_c) = chars.peek() {
                if next_c == '\''
                    || next_c == '`'
                    || next_c == '"'
                    || next_c == '^'
                    || next_c == '~'
                    || next_c == 'c'
                    || next_c == 'v'
                    || next_c == 'u'
                    || next_c == 'H'
                    || next_c == 'r'
                {
                    chars.next(); // consume accent char
                    if let Some(&accented) = chars.peek() {
                        if accented == '{' {
                            chars.next();
                            if let Some(inner) = chars.next() {
                                if inner != '}' {
                                    result.push(inner);
                                }
                            }
                            if let Some(&'}') = chars.peek() {
                                chars.next();
                            }
                        } else {
                            result.push(chars.next().unwrap());
                        }
                    }
                    continue;
                } else if next_c == '&'
                    || next_c == '%'
                    || next_c == '$'
                    || next_c == '#'
                    || next_c == '_'
                {
                    result.push(chars.next().unwrap());
                    continue;
                } else if next_c.is_alphabetic() {
                    let mut cmd = String::new();
                    while let Some(&cmd_c) = chars.peek() {
                        if cmd_c.is_alphabetic() {
                            cmd.push(cmd_c);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    if cmd == "LaTeX" {
                        result.push_str("LaTeX");
                    } else if cmd == "TeX" {
                        result.push_str("TeX");
                    } else if cmd == "aa" {
                        result.push('å');
                    } else if cmd == "AA" {
                        result.push('Å');
                    } else if cmd == "ae" {
                        result.push('æ');
                    } else if cmd == "AE" {
                        result.push('Æ');
                    } else if cmd == "o" {
                        result.push('ø');
                    } else if cmd == "O" {
                        result.push('Ø');
                    } else if cmd == "ss" {
                        result.push('ß');
                    } else if cmd == "L" {
                        result.push('Ł');
                    } else if cmd == "l" {
                        result.push('ł');
                    }
                    continue;
                }
            }
        }

        if c != '{' && c != '}' && c != '\\' {
            result.push(c);
        }
    }

    // Collapse whitespace and newlines
    let words: Vec<&str> = result.split_whitespace().collect();
    words.join(" ")
}

pub fn parse_bib_file(file_path: &str, content: &str) -> Vec<BibEntryItem> {
    let mut result = Vec::new();
    let filename = Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    let mut chars = content.char_indices().peekable();

    while let Some((_, c)) = chars.next() {
        if c == '%' {
            for (_, ch) in chars.by_ref() {
                if ch == '\n' || ch == '\r' {
                    break;
                }
            }
            continue;
        }

        if c == '@' {
            let mut entry_type = String::new();
            while let Some(&(_, ch)) = chars.peek() {
                if ch.is_alphanumeric() || ch == '_' || ch == '-' {
                    entry_type.push(ch);
                    chars.next();
                } else {
                    break;
                }
            }

            if entry_type.is_empty() {
                continue;
            }

            let mut open_delim = None;
            while let Some(&(_, ch)) = chars.peek() {
                if ch == '{' || ch == '(' {
                    open_delim = Some(ch);
                    chars.next();
                    break;
                } else if ch.is_whitespace() {
                    chars.next();
                } else {
                    break;
                }
            }

            let open_char = match open_delim {
                Some(c) => c,
                None => continue,
            };
            let close_char = if open_char == '{' { '}' } else { ')' };

            let entry_type_lower = entry_type.to_lowercase();
            if entry_type_lower == "comment" {
                let mut depth = 1;
                while let Some((_, ch)) = chars.next() {
                    if ch == open_char {
                        depth += 1;
                    } else if ch == close_char {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                }
                continue;
            }

            let mut depth = 1;
            let mut in_quotes = false;
            let mut escaped = false;
            let body_start = chars.peek().map(|&(i, _)| i).unwrap_or(0);
            let mut body_end = body_start;

            while let Some((b_idx, ch)) = chars.next() {
                if escaped {
                    escaped = false;
                    continue;
                }
                if ch == '\\' {
                    escaped = true;
                    continue;
                }

                if ch == '"' {
                    in_quotes = !in_quotes;
                } else if !in_quotes {
                    if ch == open_char {
                        depth += 1;
                    } else if ch == close_char {
                        depth -= 1;
                        if depth == 0 {
                            body_end = b_idx;
                            break;
                        }
                    }
                }
            }

            if body_end > body_start && body_end <= content.len() {
                let body = &content[body_start..body_end];
                if entry_type_lower == "string" || entry_type_lower == "preamble" {
                    continue;
                }
                if let Some(entry) =
                    parse_single_entry(&entry_type_lower, body, file_path, &filename)
                {
                    result.push(entry);
                }
            }
        }
    }

    result
}

fn parse_single_entry(
    entry_type: &str,
    body: &str,
    file_path: &str,
    filename: &str,
) -> Option<BibEntryItem> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return None;
    }

    let comma_idx = body.find(',')?;
    let key = body[..comma_idx].trim().to_string();
    if key.is_empty() {
        return None;
    }

    let fields_str = &body[comma_idx + 1..];
    let mut field_map: HashMap<String, String> = HashMap::new();

    let mut chars = fields_str.char_indices().peekable();

    while let Some((_, ch)) = chars.next() {
        if ch.is_alphanumeric() || ch == '_' || ch == '-' || ch == ':' {
            let mut field_name = String::new();
            field_name.push(ch);

            while let Some(&(_, next_c)) = chars.peek() {
                if next_c.is_alphanumeric() || next_c == '_' || next_c == '-' || next_c == ':' {
                    field_name.push(next_c);
                    chars.next();
                } else {
                    break;
                }
            }

            // Skip whitespace to '='
            let mut found_eq = false;
            while let Some(&(_, eq_c)) = chars.peek() {
                if eq_c == '=' {
                    found_eq = true;
                    chars.next();
                    break;
                } else if eq_c.is_whitespace() {
                    chars.next();
                } else {
                    break;
                }
            }

            if !found_eq {
                continue;
            }

            // Skip whitespace to value
            while let Some(&(_, val_c)) = chars.peek() {
                if val_c.is_whitespace() {
                    chars.next();
                } else {
                    break;
                }
            }

            // Parse value
            let mut field_value = String::new();
            if let Some(&(_, start_c)) = chars.peek() {
                if start_c == '{' {
                    chars.next();
                    let mut b_depth = 1;
                    let mut escaped = false;
                    while let Some((_, v_c)) = chars.next() {
                        if escaped {
                            field_value.push('\\');
                            field_value.push(v_c);
                            escaped = false;
                            continue;
                        }
                        if v_c == '\\' {
                            escaped = true;
                            continue;
                        }
                        if v_c == '{' {
                            b_depth += 1;
                            field_value.push(v_c);
                        } else if v_c == '}' {
                            b_depth -= 1;
                            if b_depth == 0 {
                                break;
                            }
                            field_value.push(v_c);
                        } else {
                            field_value.push(v_c);
                        }
                    }
                } else if start_c == '"' {
                    chars.next();
                    let mut escaped = false;
                    while let Some((_, v_c)) = chars.next() {
                        if escaped {
                            if v_c != '"' {
                                field_value.push('\\');
                            }
                            field_value.push(v_c);
                            escaped = false;
                            continue;
                        }
                        if v_c == '\\' {
                            escaped = true;
                            continue;
                        }
                        if v_c == '"' {
                            break;
                        }
                        field_value.push(v_c);
                    }
                } else {
                    while let Some(&(_, v_c)) = chars.peek() {
                        if v_c == ',' || v_c == '\n' || v_c == '\r' || v_c == '}' || v_c == ')' {
                            break;
                        }
                        field_value.push(v_c);
                        chars.next();
                    }
                }

                field_map.insert(field_name.to_lowercase(), clean_field(&field_value));
            }
        }
    }

    let title = field_map.get("title").cloned();
    let author = field_map.get("author").cloned();
    let year = field_map
        .get("year")
        .cloned()
        .or_else(|| field_map.get("date").cloned());
    let journal = field_map
        .get("journal")
        .cloned()
        .or_else(|| field_map.get("journaltitle").cloned());
    let booktitle = field_map.get("booktitle").cloned();
    let publisher = field_map
        .get("publisher")
        .cloned()
        .or_else(|| field_map.get("organization").cloned())
        .or_else(|| field_map.get("institution").cloned())
        .or_else(|| field_map.get("school").cloned());
    let doi = field_map.get("doi").cloned();
    let url = field_map.get("url").cloned();
    let keywords = field_map
        .get("keywords")
        .cloned()
        .or_else(|| field_map.get("keyword").cloned());
    let abstract_text = field_map.get("abstract").cloned();
    let note = field_map
        .get("note")
        .cloned()
        .or_else(|| field_map.get("howpublished").cloned());
    let volume = field_map.get("volume").cloned();
    let number = field_map
        .get("number")
        .cloned()
        .or_else(|| field_map.get("issue").cloned());
    let pages = field_map.get("pages").cloned();
    let eprint = field_map
        .get("eprint")
        .cloned()
        .or_else(|| field_map.get("archiveprefix").cloned());

    Some(BibEntryItem {
        item_type: "bibentry".to_string(),
        key,
        entry_type: Some(entry_type.to_string()),
        file: file_path.to_string(),
        filename: filename.to_string(),
        fields: Some(BibFields {
            title,
            author,
            year,
            journal,
            booktitle,
            publisher,
            doi,
            url,
            keywords,
            abstract_text,
            note,
            volume,
            number,
            pages,
            eprint,
        }),
    })
}

