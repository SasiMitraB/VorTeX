//! Turns a TeX `.log` file into file/line diagnostics.
//!
//! TeX announces every file it reads with `(path` and closes it with `)`, so the
//! file a message belongs to is the top of that parenthesis stack. The compiler
//! runs TeX with `max_print_line` raised, so log lines are not wrapped at 79
//! columns; wrapped logs still parse, but long paths may be cut off.

use crate::project::normalize_path as normalize;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    /// Overfull/underfull boxes: typesetting quality, not correctness.
    BadBox,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Diagnostic {
    pub severity: Severity,
    /// Absolute path of the source file, when the log says which one.
    pub file: Option<String>,
    /// 1-based line in `file`.
    pub line: Option<u32>,
    pub message: String,
}

struct Patterns {
    file_line_error: Regex,
    warning: Regex,
    input_line: Regex,
    bad_box: Regex,
    context_line: Regex,
}

fn patterns() -> &'static Patterns {
    static P: OnceLock<Patterns> = OnceLock::new();
    P.get_or_init(|| Patterns {
        file_line_error: Regex::new(r"^(.*?\.[A-Za-z0-9]+):(\d+): (.*)$").unwrap(),
        warning: Regex::new(r"^(?:(?:La|pdf|Xe|Lua)?TeX(?: Font)?|Package \S+|Class \S+|Module \S+) Warning: (.*)$").unwrap(),
        input_line: Regex::new(r"\s*on input line (\d+)\.?").unwrap(),
        bad_box: Regex::new(r"^(?:Over|Under)full \\[hv]box \([^)]*\) (?:in paragraph at lines (\d+)--\d+|detected at line (\d+)|in alignment at lines (\d+)--\d+|has occurred while \\output is active)").unwrap(),
        context_line: Regex::new(r"^l\.(\d+)").unwrap(),
    })
}

enum Frame {
    File(PathBuf),
    Other,
}

/// Parses the log of a build run in `build_dir` (relative paths in the log resolve against it).
pub fn parse(log: &str, build_dir: &Path) -> Vec<Diagnostic> {
    let p = patterns();
    let lines: Vec<&str> = log.lines().collect();
    let mut stack: Vec<Frame> = Vec::new();
    let mut out: Vec<Diagnostic> = Vec::new();
    // Lines that echo document source (unbalanced parens there are not file markers).
    let mut skip_parens_until = 0usize;
    let mut i = 0;

    let current_file = |stack: &[Frame]| {
        stack.iter().rev().find_map(|f| match f {
            Frame::File(p) => Some(p.to_string_lossy().to_string()),
            Frame::Other => None,
        })
    };

    while i < lines.len() {
        let line = lines[i];

        if let Some(c) = p.file_line_error.captures(line) {
            let file = resolve(build_dir, &c[1]);
            let line_no = c[2].parse().ok();
            let msg = c[3].trim();
            if msg.starts_with("==> Fatal error occurred") {
                // Summary of the error reported just before.
            } else if msg == "Emergency stop." {
                // "! LaTeX Error: File `x' not found." has no location; this line supplies it.
                match out.last_mut() {
                    Some(prev) if prev.severity == Severity::Error && prev.line.is_none() => {
                        prev.file = Some(file);
                        prev.line = line_no;
                    }
                    _ => push(&mut out, Severity::Error, Some(file), line_no, msg.to_string()),
                }
            } else {
                push(&mut out, Severity::Error, Some(file), line_no, strip_error_prefix(msg));
            }
            skip_parens_until = skip_error_context(&lines, i, p);
            i += 1;
            continue;
        }

        if let Some(msg) = line.strip_prefix("! ") {
            let ctx = lines.iter().enumerate().skip(i + 1).take(8).find_map(|(j, l)| {
                p.context_line.captures(l).map(|c| (j, c[1].parse::<u32>().ok()))
            });
            let line_no = ctx.and_then(|(_, n)| n);
            let mut msg = strip_error_prefix(msg.trim());
            if let Some((j, _)) = ctx {
                // "l.12 \foo" shows the offending token.
                let token = lines[j].split_once(' ').map(|(_, t)| t.trim()).unwrap_or("");
                if !token.is_empty() && msg.contains("Undefined control sequence") {
                    msg = format!("{msg} {token}");
                }
            }
            let file = if line_no.is_some() { current_file(&stack) } else { None };
            push(&mut out, Severity::Error, file, line_no, msg);
            skip_parens_until = skip_error_context(&lines, i, p);
            i += 1;
            continue;
        }

        if let Some(c) = p.warning.captures(line) {
            let mut text = c[1].trim().to_string();
            // Continuation lines: "(natbib)   ..." for packages, plain lines for LaTeX, up to a blank line.
            let mut j = i + 1;
            while j < lines.len() && !lines[j].trim().is_empty() && !starts_new_message(lines[j], p) {
                let cont = lines[j].trim();
                let cont = cont.strip_prefix('(').and_then(|r| r.split_once(')')).map(|(_, r)| r.trim()).unwrap_or(cont);
                text.push(' ');
                text.push_str(cont);
                j += 1;
            }
            let line_no = p.input_line.captures(&text).and_then(|c| c[1].parse().ok());
            let message = p.input_line.replace(&text, "").trim().trim_end_matches('.').to_string() + ".";
            push(&mut out, Severity::Warning, current_file(&stack), line_no, message);
            i = j;
            continue;
        }

        if let Some(c) = p.bad_box.captures(line) {
            let line_no = (1..=3).find_map(|g| c.get(g)).and_then(|m| m.as_str().parse().ok());
            let message = line.split(" in paragraph").next().unwrap_or(line);
            let message = message.split(" detected at").next().unwrap_or(message);
            let message = message.split(" in alignment").next().unwrap_or(message).to_string();
            push(&mut out, Severity::BadBox, current_file(&stack), line_no, message);
            // The following lines show the box contents (document text) up to a blank line.
            let mut j = i + 1;
            while j < lines.len() && !lines[j].trim().is_empty() {
                j += 1;
            }
            skip_parens_until = j;
            i += 1;
            continue;
        }

        if i >= skip_parens_until {
            track_files(line, build_dir, &mut stack);
        }
        i += 1;
    }
    out
}

/// True when any diagnostic is an error.
pub fn has_errors(diagnostics: &[Diagnostic]) -> bool {
    diagnostics.iter().any(|d| d.severity == Severity::Error)
}

fn push(out: &mut Vec<Diagnostic>, severity: Severity, file: Option<String>, line: Option<u32>, message: String) {
    let d = Diagnostic { severity, file, line, message };
    if !out.contains(&d) {
        out.push(d);
    }
}

fn strip_error_prefix(msg: &str) -> String {
    msg.strip_prefix("LaTeX Error: ").unwrap_or(msg).trim().to_string()
}

fn starts_new_message(line: &str, p: &Patterns) -> bool {
    line.starts_with("! ") || p.warning.is_match(line) || p.bad_box.is_match(line) || p.file_line_error.is_match(line)
}

/// After an error, the `l.<n> <source>` line and the line after it echo document source.
fn skip_error_context(lines: &[&str], i: usize, p: &Patterns) -> usize {
    lines
        .iter()
        .enumerate()
        .skip(i + 1)
        .take(8)
        .find(|(_, l)| p.context_line.is_match(l))
        .map(|(j, _)| j + 2)
        .unwrap_or(i + 1)
}

fn resolve(build_dir: &Path, path: &str) -> String {
    let p = Path::new(path);
    let abs = if p.is_absolute() { p.to_path_buf() } else { build_dir.join(p) };
    normalize(&abs).to_string_lossy().to_string()
}

fn looks_like_file(token: &str) -> bool {
    if token.is_empty() {
        return false;
    }
    if token.starts_with('/') || token.starts_with("./") || token.starts_with("../") {
        return true;
    }
    // Bare "name.ext" or "dir/name.ext", as TeX prints files found via kpathsea.
    let name = token.rsplit('/').next().unwrap_or(token);
    match name.rsplit_once('.') {
        Some((stem, ext)) => {
            !stem.is_empty()
                && (1..=8).contains(&ext.len())
                && ext.chars().all(|c| c.is_ascii_alphanumeric())
                && token.chars().all(|c| c.is_alphanumeric() || "/._-+".contains(c))
        }
        None => false,
    }
}

/// Updates the file stack for every `(` and `)` on the line.
fn track_files(line: &str, build_dir: &Path, stack: &mut Vec<Frame>) {
    let mut rest = line;
    while let Some(pos) = rest.find(['(', ')']) {
        let (ch, after) = (rest.as_bytes()[pos], &rest[pos + 1..]);
        if ch == b')' {
            stack.pop();
            rest = after;
            continue;
        }
        // Everything up to the next paren could be a path that contains spaces.
        let span_end = after.find(['(', ')']).unwrap_or(after.len());
        let span = &after[..span_end];
        let token_end = span.find(char::is_whitespace).unwrap_or(span.len());
        let token = &span[..token_end];
        if looks_like_file(token) {
            let path = longest_existing_path(span, build_dir).unwrap_or_else(|| token.to_string());
            stack.push(Frame::File(PathBuf::from(resolve(build_dir, &path))));
            rest = &after[path.len().min(after.len())..];
        } else {
            stack.push(Frame::Other);
            rest = after;
        }
    }
}

/// The longest whitespace-delimited prefix of `span` that is an existing file.
fn longest_existing_path(span: &str, build_dir: &Path) -> Option<String> {
    let mut ends: Vec<usize> = span.match_indices(char::is_whitespace).map(|(i, _)| i).collect();
    ends.push(span.len());
    ends.into_iter().rev().map(|e| &span[..e]).find(|cand| {
        let p = Path::new(cand);
        let abs = if p.is_absolute() { p.to_path_buf() } else { build_dir.join(p) };
        abs.is_file()
    }).map(str::to_string)
}
