//! Autocomplete: works out what is being completed at the cursor and builds the
//! edit for each suggestion, so the editor only has to apply it.
//!
//! Columns are UTF-16 code units (JavaScript string offsets) on a single line.

use crate::backend::BackendClient;
use crate::text::{byte_at_utf16, utf16_len};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum CompletionKind {
    Reference,
    Citation,
    Environment,
    Command,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionKind,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    /// Replaces `from..to` on the line; may span several lines (already indented).
    pub insert_text: String,
    /// Where the cursor goes, in UTF-16 units from the start of `insert_text`.
    pub cursor_offset: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Completions {
    /// Replacement range on the line, in UTF-16 units.
    pub from: u32,
    pub to: u32,
    /// The text being completed (what the matcher ranked against).
    pub query: String,
    pub items: Vec<CompletionItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionContext {
    /// Inside `\cite{…}` and friends; `query` is the key being typed.
    Citation { query: String },
    /// Inside `\ref{…}` and friends.
    Reference { query: String },
    /// Inside `\begin{…}`.
    Environment { query: String },
    /// After a backslash: `\sec` → `sec`.
    Command { query: String },
}

const CITE_COMMANDS: &[&str] = &[
    "\\cite", "\\citep", "\\citet", "\\citealt", "\\citealp", "\\citeauthor", "\\citeyear", "\\citeyearpar",
    "\\citetext", "\\parencite", "\\textcite", "\\autocite", "\\footcite", "\\nocite", "\\fullcite", "\\cites",
    "\\citeurl",
];
const REF_COMMANDS: &[&str] = &["\\ref", "\\eqref", "\\pageref", "\\autoref", "\\cref", "\\Cref"];

/// What is being completed, given the line's text before the cursor.
pub fn context_at(prefix: &str) -> Option<CompletionContext> {
    // 1. Inside an unclosed '{'
    if let Some(open_brace_idx) = prefix.rfind('{') {
        let inside = &prefix[open_brace_idx + 1..];
        if !inside.contains('}') {
            let before_brace = prefix[..open_brace_idx].trim_end();

            // Strip optional arguments like `[see][p.~10]` before the `{`
            let mut cmd_end = before_brace;
            while cmd_end.ends_with(']') {
                match cmd_end.rfind('[') {
                    Some(open_bracket) => cmd_end = cmd_end[..open_bracket].trim_end(),
                    None => break,
                }
            }

            if let Some(slash_idx) = cmd_end.rfind('\\') {
                let cmd = &cmd_end[slash_idx..];
                let cmd = cmd.strip_suffix('*').unwrap_or(cmd);
                let last_key = || inside.rsplit(',').next().unwrap_or(inside).trim_start().to_string();
                if CITE_COMMANDS.contains(&cmd) {
                    return Some(CompletionContext::Citation { query: last_key() });
                }
                if REF_COMMANDS.contains(&cmd) {
                    return Some(CompletionContext::Reference { query: last_key() });
                }
                if cmd == "\\begin" {
                    return Some(CompletionContext::Environment { query: inside.trim().to_string() });
                }
            }
        }
    }

    // 2. A command name after a backslash: \alpha, \sub, ...
    if let Some(pos) = prefix.rfind('\\') {
        let query = &prefix[pos + 1..];
        if !query.is_empty() && query.chars().all(|c| c.is_alphabetic()) {
            return Some(CompletionContext::Command { query: query.to_string() });
        }
    }
    None
}

/// Suggestions for the cursor at UTF-16 column `cursor` on `line`.
pub fn complete(backend: &BackendClient, line: &str, cursor: u32, current_file: Option<&str>) -> Option<Completions> {
    let split = byte_at_utf16(line, cursor);
    let (prefix, suffix) = line.split_at(split);
    let cursor = utf16_len(prefix);
    let ctx = context_at(prefix)?;
    let result = match &ctx {
        CompletionContext::Citation { query } | CompletionContext::Reference { query } => {
            key_completions(backend, &ctx, query, suffix, cursor, current_file)
        }
        CompletionContext::Environment { query } => environment_completions(query, prefix, suffix, cursor),
        CompletionContext::Command { query } => command_completions(query, cursor),
    };
    (!result.items.is_empty()).then_some(result)
}

/// `\ref{…}` / `\cite{…}`: replaces the whole key under the cursor and closes the brace if needed.
fn key_completions(
    backend: &BackendClient,
    ctx: &CompletionContext,
    query: &str,
    suffix: &str,
    cursor: u32,
    current_file: Option<&str>,
) -> Completions {
    // The rest of the key after the cursor is replaced too.
    let rest_len = suffix.find(|c: char| c == ',' || c == '}' || c.is_whitespace()).unwrap_or(suffix.len());
    let closer = suffix[rest_len..].chars().next();
    let from = cursor - utf16_len(query);
    let mut to = cursor + utf16_len(&suffix[..rest_len]);
    let brace = match closer {
        Some(',') => "",
        Some('}') => {
            to += 1;
            "}"
        }
        _ => "}",
    };
    let finish = |key: String, kind, detail, documentation| {
        let insert_text = format!("{key}{brace}");
        CompletionItem { cursor_offset: utf16_len(&insert_text), label: key, kind, detail, documentation, insert_text }
    };

    let items = match ctx {
        CompletionContext::Reference { .. } => backend
            .fuzzy_search_labels(query, current_file)
            .unwrap_or_default()
            .into_iter()
            .map(|m| {
                let str_field = |k: &str| m.obj.get(k).and_then(|v| v.as_str());
                let name = str_field("key").or_else(|| str_field("name")).unwrap_or("").to_string();
                let detail = str_field("file").map(|s| {
                    let base = std::path::Path::new(s).file_name().and_then(|n| n.to_str()).unwrap_or(s);
                    let line = m.obj.get("line").and_then(|l| l.as_u64()).unwrap_or(0);
                    format!("{base}:{line}")
                });
                finish(name, CompletionKind::Reference, detail, None)
            })
            .collect(),
        _ => backend
            .fuzzy_search_citations(query, current_file)
            .unwrap_or_default()
            .into_iter()
            .map(|m| {
                let key = m.obj.get("key").and_then(|k| k.as_str()).unwrap_or("").to_string();
                let field = |k: &str| m.obj.get("fields").and_then(|f| f.get(k)).and_then(|v| v.as_str());
                let (title, author, year) = (field("title"), field("author"), field("year"));
                let venue = field("journal").or_else(|| field("booktitle"));
                let detail = match (author, year) {
                    (Some(a), Some(y)) => {
                        let first = a.split(" and ").next().unwrap_or(a);
                        let short = if a.contains(" and ") { format!("{first} et al.") } else { first.to_string() };
                        Some(format!("{short}, {y}"))
                    }
                    (Some(a), None) => Some(a.to_string()),
                    _ => title.map(str::to_string),
                };
                let documentation = match (title, venue, year) {
                    (Some(t), Some(j), Some(y)) => Some(format!("{t}\n{j} ({y})")),
                    (Some(t), Some(j), None) => Some(format!("{t}\n{j}")),
                    (Some(t), None, Some(y)) => Some(format!("{t} ({y})")),
                    (Some(t), None, None) => Some(t.to_string()),
                    _ => None,
                };
                finish(key, CompletionKind::Citation, detail, documentation)
            })
            .collect(),
    };
    Completions { from, to, query: query.to_string(), items }
}

const ENVIRONMENTS: &[(&str, &str)] = &[
    ("figure", "Figure environment with caption and label"),
    ("table", "Table environment"),
    ("tabular", "Tabular data grid"),
    ("equation", "Numbered single equation"),
    ("align", "Multi-line aligned equations"),
    ("itemize", "Bullet point list"),
    ("enumerate", "Numbered list"),
    ("matrix", "Matrix environment"),
    ("pmatrix", "Matrix with parentheses"),
    ("bmatrix", "Matrix with brackets"),
    ("proof", "Mathematical proof"),
    ("theorem", "Theorem environment"),
    ("lemma", "Lemma environment"),
    ("definition", "Definition environment"),
];

/// `\begin{fig` → `\begin{figure}`, an indented blank line for the cursor, and `\end{figure}`.
fn environment_completions(query: &str, prefix: &str, suffix: &str, cursor: u32) -> Completions {
    let indent: String = prefix.chars().take_while(|c| c.is_whitespace()).collect();
    let from = cursor - utf16_len(query);
    let to = cursor + u32::from(suffix.starts_with('}'));
    let q = query.to_lowercase();
    let items = ENVIRONMENTS
        .iter()
        .filter(|(env, _)| env.contains(q.as_str()))
        .map(|(env, desc)| {
            let body_line = format!("{env}}}\n{indent}  ");
            CompletionItem {
                label: env.to_string(),
                kind: CompletionKind::Environment,
                detail: Some(desc.to_string()),
                documentation: None,
                cursor_offset: utf16_len(&body_line),
                insert_text: format!("{body_line}\n{indent}\\end{{{env}}}"),
            }
        })
        .collect();
    Completions { from, to, query: query.to_string(), items }
}

/// (name, description, text inserted after the backslash)
const COMMANDS: &[(&str, &str, &str)] = &[
    ("section", "\\section{title}", "section{}"),
    ("subsection", "\\subsection{title}", "subsection{}"),
    ("subsubsection", "\\subsubsection{title}", "subsubsection{}"),
    ("textbf", "\\textbf{bold text}", "textbf{}"),
    ("textit", "\\textit{italic text}", "textit{}"),
    ("texttt", "\\texttt{monospace text}", "texttt{}"),
    ("underline", "\\underline{text}", "underline{}"),
    ("label", "\\label{marker}", "label{}"),
    ("ref", "\\ref{marker}", "ref{}"),
    ("cite", "\\cite{key}", "cite{}"),
    ("frac", "\\frac{num}{den}", "frac{}{}"),
    ("sqrt", "\\sqrt{x}", "sqrt{}"),
    ("int", "\\int_{a}^{b}", "int_{}^{}"),
    ("sum", "\\sum_{i=1}^{n}", "sum_{}^{}"),
    ("includegraphics", "\\includegraphics[width=\\linewidth]{file}", "includegraphics[]{}"),
];

fn command_completions(query: &str, cursor: u32) -> Completions {
    let q = query.to_lowercase();
    let items = COMMANDS
        .iter()
        .filter(|(cmd, _, _)| cmd.starts_with(q.as_str()))
        .map(|(cmd, desc, ins)| {
            // The cursor goes into the first empty argument.
            let first_arg = [ins.find("{}"), ins.find("[]")].into_iter().flatten().min();
            let cursor_offset = first_arg.map(|i| i + 1).unwrap_or(ins.len()) as u32;
            CompletionItem {
                label: format!("\\{cmd}"),
                kind: CompletionKind::Command,
                detail: Some(desc.to_string()),
                documentation: None,
                insert_text: ins.to_string(),
                cursor_offset,
            }
        })
        .collect();
    Completions { from: cursor - utf16_len(query), to: cursor, query: query.to_string(), items }
}
