//! Hover previews for LaTeX math.
//!
//! `find_math_spans` locates inline (`$..$`, `\(..\)`), display (`$$..$$`, `\[..\]`)
//! and environment (`equation`, `align`, ...) math in a buffer. `render_math`
//! compiles a single span with `pdflatex` into a tiny `standalone` document
//! (reusing the document's own macros and math packages) and rasterizes it with
//! MuPDF into a transparent PNG, cached on disk by source hash.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::compiler::get_latex_path_env;

const MATH_ENVS: &[&str] = &[
    "equation", "equation*", "align", "align*", "gather", "gather*", "multline", "multline*",
    "flalign", "flalign*", "alignat", "alignat*", "eqnarray", "eqnarray*", "displaymath", "math",
];

/// Environments whose contents are never math (so `$` inside them is literal).
const VERBATIM_ENVS: &[&str] = &["verbatim", "verbatim*", "lstlisting", "minted", "comment"];

/// Packages from the user's preamble that are safe to load in a math-only snippet.
const PACKAGE_WHITELIST: &[&str] = &[
    "amsmath", "amssymb", "amsfonts", "mathtools", "bm", "physics", "siunitx", "mathrsfs",
    "esint", "upgreek", "cancel", "braket", "dsfont", "bbm", "units", "nicefrac", "xfrac",
    "commath", "derivative", "diffcoeff", "tensor", "mhchem", "fix-cm",
    // Fonts, so the preview matches the document's typeface
    "newtxtext", "newtxmath", "mathptmx", "lmodern", "fourier", "libertine", "mathpazo",
    "txfonts", "pxfonts", "eulervm", "stix", "stix2", "newpxtext", "newpxmath", "kpfonts",
];

/// Packages that provide their own math symbol fonts (and clash with `amssymb`).
const MATH_FONT_PACKAGES: &[&str] = &[
    "newtxmath", "mathptmx", "fourier", "mathpazo", "txfonts", "pxfonts", "stix", "stix2",
    "newpxmath", "kpfonts",
];

const MACRO_COMMANDS: &[&str] = &[
    "newcommand", "renewcommand", "providecommand", "DeclareMathOperator",
    "DeclareRobustCommand", "DeclarePairedDelimiter", "def", "let",
];

/// Pixels per PDF point in the rasterized PNG (288 dpi: crisp on Retina displays).
const RASTER_SCALE: f32 = 4.0;
const COMPILE_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct MathSpan {
    /// (row, col) of the first character of the opening delimiter.
    pub start: (usize, usize),
    /// (row, col) of the last character of the closing delimiter.
    pub end: (usize, usize),
    /// Math content between the delimiters.
    pub body: String,
    pub kind: MathKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum MathKind {
    Inline,
    Display,
    Environment(String),
}

impl MathSpan {
    /// LaTeX source that typesets this span on its own. Everything is set as
    /// `$\displaystyle ...$` (multi-line environments via their `aligned`-style
    /// inner forms) so `standalone` crops the output to the math's natural size.
    pub fn snippet(&self) -> String {
        let body = strip_numbering(&self.body);
        let inner = match &self.kind {
            MathKind::Inline | MathKind::Display => body.trim().to_string(),
            MathKind::Environment(env) => {
                let inner_env = match env.trim_end_matches('*') {
                    "align" | "flalign" | "eqnarray" => Some("aligned"),
                    "alignat" => Some("alignedat"),
                    "gather" | "multline" => Some("gathered"),
                    _ => None,
                };
                match inner_env {
                    Some(e) => format!("\\begin{{{e}}}{body}\\end{{{e}}}"),
                    None => body.trim().to_string(),
                }
            }
        };
        format!("$\\displaystyle {inner}$")
    }

    /// Short label for the preview header.
    pub fn label(&self) -> String {
        match &self.kind {
            MathKind::Inline => "inline math".to_string(),
            MathKind::Display => "display math".to_string(),
            MathKind::Environment(env) => env.clone(),
        }
    }
}

/// Removes numbering/labelling commands that are invalid in inline math, and
/// turns `split` (only valid inside `equation`) into `aligned`.
fn strip_numbering(body: &str) -> String {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        regex::Regex::new(r"\\label\{[^}]*\}|\\tag\*?\{[^}]*\}|\\(?:nonumber|notag)\b").unwrap()
    });
    re.replace_all(body, "")
        .replace("\\begin{split}", "\\begin{aligned}")
        .replace("\\end{split}", "\\end{aligned}")
}

/// Flattened view of the buffer so delimiters can span lines.
struct Text {
    chars: Vec<char>,
    line_starts: Vec<usize>,
}

impl Text {
    fn new(lines: &[String]) -> Self {
        let mut chars = Vec::new();
        let mut line_starts = Vec::with_capacity(lines.len());
        for line in lines {
            line_starts.push(chars.len());
            chars.extend(line.chars());
            chars.push('\n');
        }
        Self { chars, line_starts }
    }

    fn pos(&self, idx: usize) -> (usize, usize) {
        let row = match self.line_starts.binary_search(&idx) {
            Ok(r) => r,
            Err(r) => r - 1,
        };
        (row, idx - self.line_starts[row])
    }

    fn starts_with(&self, idx: usize, pat: &[char]) -> bool {
        self.chars.get(idx..idx + pat.len()) == Some(pat)
    }

    fn skip_comment(&self, mut i: usize) -> usize {
        while i < self.chars.len() && self.chars[i] != '\n' {
            i += 1;
        }
        i
    }

    /// Index of the next unescaped, uncommented `pat` at or after `from`.
    /// With `stop_at_paragraph`, gives up at a blank line (unterminated `$`).
    fn find_closing(&self, from: usize, pat: &[char], stop_at_paragraph: bool) -> Option<usize> {
        let mut i = from;
        while i < self.chars.len() {
            if self.starts_with(i, pat) {
                return Some(i);
            }
            match self.chars[i] {
                '\\' => i += 2,
                '%' => i = self.skip_comment(i),
                '\n' if stop_at_paragraph => {
                    let mut j = i + 1;
                    while j < self.chars.len() && (self.chars[j] == ' ' || self.chars[j] == '\t') {
                        j += 1;
                    }
                    if j >= self.chars.len() || self.chars[j] == '\n' {
                        return None;
                    }
                    i += 1;
                }
                _ => i += 1,
            }
        }
        None
    }

    /// Reads `{name}` starting at `idx`, returning (name, index after `}`).
    fn read_braced_name(&self, idx: usize) -> Option<(String, usize)> {
        if self.chars.get(idx) != Some(&'{') {
            return None;
        }
        let mut j = idx + 1;
        let mut name = String::new();
        while j < self.chars.len() && self.chars[j] != '}' && self.chars[j] != '\n' {
            name.push(self.chars[j]);
            j += 1;
        }
        (self.chars.get(j) == Some(&'}')).then(|| (name.trim().to_string(), j + 1))
    }

    fn span(&self, open: usize, body_start: usize, body_end: usize, close_end: usize, kind: MathKind) -> MathSpan {
        MathSpan {
            start: self.pos(open),
            end: self.pos(close_end - 1),
            body: self.chars[body_start..body_end].iter().collect(),
            kind,
        }
    }
}

/// All math regions in the buffer, in document order.
pub fn find_math_spans(lines: &[String]) -> Vec<MathSpan> {
    let text = Text::new(lines);
    let chars = &text.chars;
    let mut spans = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        match chars[i] {
            '%' => i = text.skip_comment(i),
            '\\' => {
                let next = chars.get(i + 1).copied();
                if next == Some('(') || next == Some('[') {
                    let (close, kind) = if next == Some('(') {
                        (['\\', ')'], MathKind::Inline)
                    } else {
                        (['\\', ']'], MathKind::Display)
                    };
                    if let Some(end) = text.find_closing(i + 2, &close, false) {
                        spans.push(text.span(i, i + 2, end, end + 2, kind));
                        i = end + 2;
                        continue;
                    }
                    i += 2;
                } else if text.starts_with(i + 1, &['b', 'e', 'g', 'i', 'n']) {
                    let Some((env, after)) = text.read_braced_name(i + 6) else {
                        i += 6;
                        continue;
                    };
                    let is_math = MATH_ENVS.contains(&env.as_str());
                    if is_math || VERBATIM_ENVS.contains(&env.as_str()) {
                        let end_pat: Vec<char> = format!("\\end{{{env}}}").chars().collect();
                        // Verbatim content has no escapes or comments, so search literally.
                        let found = if is_math {
                            text.find_closing(after, &end_pat, false)
                        } else {
                            (after..chars.len()).find(|&k| text.starts_with(k, &end_pat))
                        };
                        match found {
                            Some(end) => {
                                if is_math {
                                    spans.push(text.span(i, after, end, end + end_pat.len(), MathKind::Environment(env)));
                                }
                                i = end + end_pat.len();
                            }
                            None => i = after,
                        }
                    } else {
                        i = after;
                    }
                } else {
                    // Control word or escaped symbol such as `\$` / `\%`
                    i += 2;
                }
            }
            '$' => {
                if chars.get(i + 1) == Some(&'$') {
                    match text.find_closing(i + 2, &['$', '$'], false) {
                        Some(end) => {
                            spans.push(text.span(i, i + 2, end, end + 2, MathKind::Display));
                            i = end + 2;
                        }
                        None => i += 2,
                    }
                } else {
                    match text.find_closing(i + 1, &['$'], true) {
                        Some(end) => {
                            spans.push(text.span(i, i + 1, end, end + 1, MathKind::Inline));
                            i = end + 1;
                        }
                        None => i += 1,
                    }
                }
            }
            _ => i += 1,
        }
    }
    spans
}

/// The math region containing buffer position (row, col), if any.
pub fn math_span_at(spans: &[MathSpan], row: usize, col: usize) -> Option<&MathSpan> {
    spans.iter().find(|s| (row, col) >= s.start && (row, col) <= s.end)
}

// ─────────────────────────────────────────────────────────────────────────────
// Preamble: user macros and math packages
// ─────────────────────────────────────────────────────────────────────────────

/// Index just past the balanced `{...}` or `[...]` group opening at `idx`.
fn skip_group(chars: &[char], idx: usize) -> Option<usize> {
    let (open, close) = match chars.get(idx)? {
        '{' => ('{', '}'),
        '[' => ('[', ']'),
        _ => return None,
    };
    let mut depth = 0usize;
    let mut i = idx;
    while i < chars.len() {
        match chars[i] {
            '\\' => i += 1,
            c if c == open => depth += 1,
            c if c == close => {
                depth -= 1;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn skip_whitespace(chars: &[char], mut i: usize) -> usize {
    while i < chars.len() && chars[i].is_whitespace() {
        i += 1;
    }
    i
}

/// Index past a control sequence (`\name` or `\@`) starting at `idx`.
fn skip_control_sequence(chars: &[char], idx: usize) -> usize {
    let mut i = idx + 1;
    if i < chars.len() && chars[i].is_ascii_alphabetic() {
        while i < chars.len() && (chars[i].is_ascii_alphabetic() || chars[i] == '@') {
            i += 1;
        }
        i
    } else {
        (i + 1).min(chars.len())
    }
}

/// Extracts macro definitions (`\newcommand`, `\def`, `\DeclareMathOperator`, ...).
pub fn extract_macros(content: &str) -> Vec<String> {
    let chars: Vec<char> = content.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '%' => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            '\\' => {
                let name_end = skip_control_sequence(&chars, i);
                let name: String = chars[i + 1..name_end].iter().collect();
                if !MACRO_COMMANDS.contains(&name.as_str()) {
                    i = name_end;
                    continue;
                }
                let mut j = name_end;
                if chars.get(j) == Some(&'*') {
                    j += 1;
                }
                let end = match name.as_str() {
                    "let" => {
                        // \let\a\b  or  \let\a=\b
                        let mut k = skip_whitespace(&chars, j);
                        if chars.get(k) == Some(&'\\') {
                            k = skip_control_sequence(&chars, k);
                        }
                        k = skip_whitespace(&chars, k);
                        if chars.get(k) == Some(&'=') {
                            k = skip_whitespace(&chars, k + 1);
                        }
                        (chars.get(k) == Some(&'\\')).then(|| skip_control_sequence(&chars, k))
                    }
                    "def" => {
                        // \def\name<params>{body}
                        let k = skip_whitespace(&chars, j);
                        if chars.get(k) != Some(&'\\') {
                            None
                        } else {
                            let mut k = skip_control_sequence(&chars, k);
                            while k < chars.len() && chars[k] != '{' && chars[k] != '\n' {
                                k += 1;
                            }
                            skip_group(&chars, k)
                        }
                    }
                    _ => {
                        // \newcommand{\name}[n][default]{body}, also \newcommand\name...
                        let mut k = skip_whitespace(&chars, j);
                        let mut groups = 0;
                        loop {
                            let next = skip_whitespace(&chars, k);
                            match chars.get(next) {
                                Some('{') | Some('[') => match skip_group(&chars, next) {
                                    Some(e) => {
                                        k = e;
                                        groups += 1;
                                    }
                                    None => break,
                                },
                                Some('\\') if groups == 0 => {
                                    k = skip_control_sequence(&chars, next);
                                    groups += 1;
                                }
                                _ => break,
                            }
                        }
                        (groups >= 2).then_some(k)
                    }
                };
                match end {
                    Some(e) => {
                        out.push(chars[i..e].iter().collect());
                        i = e;
                    }
                    None => i = name_end,
                }
            }
            _ => i += 1,
        }
    }
    out
}

/// Whitelisted `\usepackage` lines, in document order.
pub fn extract_math_packages(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in content.lines() {
        let line = line.split('%').next().unwrap_or("").trim();
        let Some(rest) = line.strip_prefix("\\usepackage") else {
            continue;
        };
        let (options, rest) = match rest.strip_prefix('[').and_then(|r| r.split_once(']')) {
            Some((opts, r)) => (Some(opts), r),
            None => (None, rest),
        };
        let Some(names) = rest.trim_start().strip_prefix('{').and_then(|r| r.split_once('}')).map(|(n, _)| n) else {
            continue;
        };
        let kept: Vec<&str> = names
            .split(',')
            .map(str::trim)
            .filter(|n| PACKAGE_WHITELIST.contains(n))
            .collect();
        if kept.is_empty() {
            continue;
        }
        let opts = match options {
            Some(o) if kept.len() == 1 => format!("[{o}]"),
            _ => String::new(),
        };
        out.push(format!("\\usepackage{opts}{{{}}}", kept.join(",")));
    }
    out
}

/// The current buffer plus `main.tex` and any `\input`/`\include`d files next to it.
fn gather_sources(buffer: &str, file_path: Option<&str>) -> Vec<String> {
    let mut sources = vec![buffer.to_string()];
    let Some(dir) = file_path.and_then(|p| Path::new(p).parent()) else {
        return sources;
    };
    let mut seen: Vec<PathBuf> = file_path.map(PathBuf::from).into_iter().collect();
    let mut add = |path: PathBuf, sources: &mut Vec<String>| {
        if seen.contains(&path) || seen.len() > 16 {
            return;
        }
        seen.push(path.clone());
        if let Ok(content) = std::fs::read_to_string(&path) {
            sources.push(content);
        }
    };

    add(dir.join("main.tex"), &mut sources);
    static INPUT_RE: OnceLock<regex::Regex> = OnceLock::new();
    let input_re = INPUT_RE.get_or_init(|| regex::Regex::new(r"\\(?:input|include)\{([^}]+)\}").unwrap());
    let mut idx = 0;
    while idx < sources.len() && idx < 4 {
        let found: Vec<PathBuf> = input_re
            .captures_iter(&sources[idx])
            .map(|c| {
                let name = c[1].trim();
                let mut p = dir.join(name);
                if p.extension().is_none() {
                    p.set_extension("tex");
                }
                p
            })
            .collect();
        for p in found {
            add(p, &mut sources);
        }
        idx += 1;
    }
    sources
}

/// Preamble lines (packages + macros) for rendering math from this document.
pub fn build_preamble(buffer: &str, file_path: Option<&str>) -> String {
    let sources = gather_sources(buffer, file_path);
    let mut packages: Vec<String> = Vec::new();
    let mut macros: Vec<String> = Vec::new();
    for src in &sources {
        for p in extract_math_packages(src) {
            if !packages.contains(&p) {
                packages.push(p);
            }
        }
        for m in extract_macros(src) {
            if !macros.contains(&m) {
                macros.push(m);
            }
        }
    }
    let has_math_font = packages.iter().any(|p| MATH_FONT_PACKAGES.iter().any(|f| p.contains(f)));
    let has_amssymb = packages.iter().any(|p| p.contains("amssymb"));
    let mut lines = vec!["\\usepackage{amsmath}".to_string()];
    lines.extend(packages);
    if !has_math_font && !has_amssymb {
        lines.push("\\usepackage{amssymb}".to_string());
    }
    lines.extend(macros.iter().map(|m| make_redefinable(m)));
    lines.join("\n")
}

/// `\newcommand`/`\renewcommand` fail if the command is already / not yet defined,
/// which depends on the document class and packages we don't load. Declaring it
/// with `\providecommand` first makes `\renewcommand` work either way.
fn make_redefinable(def: &str) -> String {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        regex::Regex::new(r"^\\(?:new|renew)command(\*?)\s*(?:\{\s*(\\[A-Za-z@]+)\s*\}|(\\[A-Za-z@]+))").unwrap()
    });
    let Some(c) = re.captures(def) else {
        return def.to_string();
    };
    let star = &c[1];
    let name = c.get(2).or_else(|| c.get(3)).unwrap().as_str();
    let rest = &def[c.get(0).unwrap().end()..];
    format!("\\providecommand{{{name}}}{{}}\\renewcommand{star}{{{name}}}{rest}")
}

// ─────────────────────────────────────────────────────────────────────────────
// Rendering
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct RenderedMath {
    pub png_path: PathBuf,
    /// Size of the typeset math in PDF points.
    pub width_pt: f32,
    pub height_pt: f32,
}

pub struct MathRenderRequest {
    pub snippet: String,
    pub preamble: String,
    /// Text color as `RRGGBB`.
    pub color_hex: String,
}

fn cache_root() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".vortex-editor").join("math_cache"))
}

fn fnv1a(s: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in s.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Preambles that failed to compile, so later renders skip straight to the minimal one.
static BROKEN_PREAMBLES: Mutex<Option<HashMap<u64, ()>>> = Mutex::new(None);

fn document(preamble: &str, color_hex: &str, snippet: &str) -> String {
    format!(
        "\\documentclass[border=1pt]{{standalone}}\n{preamble}\n\\usepackage{{xcolor}}\n\\begin{{document}}\n\\color[HTML]{{{color_hex}}}\n{snippet}\n\\end{{document}}\n"
    )
}

/// Renders the snippet, trying the document's preamble first and falling back
/// to a minimal one if the document's macros/packages fail to compile.
pub fn render_math(req: &MathRenderRequest) -> Result<RenderedMath, String> {
    let preamble_key = fnv1a(&req.preamble);
    let preamble_broken = BROKEN_PREAMBLES
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|m| m.contains_key(&preamble_key)))
        .unwrap_or(false);

    if !preamble_broken {
        match compile_to_png(&document(&req.preamble, &req.color_hex, &req.snippet)) {
            Ok(r) => return Ok(r),
            Err(_) => {
                // Only blame the preamble if the snippet works without it.
                let minimal = compile_to_png(&document(MINIMAL_PREAMBLE, &req.color_hex, &req.snippet));
                if minimal.is_ok() {
                    if let Ok(mut g) = BROKEN_PREAMBLES.lock() {
                        g.get_or_insert_with(HashMap::new).insert(preamble_key, ());
                    }
                }
                return minimal;
            }
        }
    }
    compile_to_png(&document(MINIMAL_PREAMBLE, &req.color_hex, &req.snippet))
}

const MINIMAL_PREAMBLE: &str = "\\usepackage{amsmath,amssymb}";

/// Reads width/height from a PNG's IHDR chunk.
fn png_dimensions(path: &Path) -> Option<(u32, u32)> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.len() < 24 || &bytes[1..4] != b"PNG" {
        return None;
    }
    let w = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let h = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    Some((w, h))
}

fn compile_to_png(tex_source: &str) -> Result<RenderedMath, String> {
    let root = cache_root().ok_or("No home directory for the math cache")?;
    let key = format!("{:016x}", fnv1a(tex_source));
    let png_path = root.join(format!("{key}.png"));
    let failure_path = root.join(format!("{key}.err"));

    let rendered = |png_path: PathBuf| -> Result<RenderedMath, String> {
        let (w, h) = png_dimensions(&png_path).ok_or("Unreadable preview image")?;
        Ok(RenderedMath {
            png_path,
            width_pt: w as f32 / RASTER_SCALE,
            height_pt: h as f32 / RASTER_SCALE,
        })
    };
    if png_path.exists() {
        return rendered(png_path);
    }
    if let Ok(msg) = std::fs::read_to_string(&failure_path) {
        return Err(msg);
    }

    let work_dir = root.join(format!("work-{key}"));
    std::fs::create_dir_all(&work_dir).map_err(|e| e.to_string())?;
    let result = (|| {
        std::fs::write(work_dir.join("snippet.tex"), tex_source).map_err(|e| e.to_string())?;
        run_pdflatex(&work_dir)?;
        rasterize(&work_dir.join("snippet.pdf"), &png_path)?;
        rendered(png_path.clone())
    })();
    if let Err(msg) = &result {
        // Remember LaTeX errors so hovering a broken equation doesn't recompile it.
        if !msg.starts_with("pdflatex") {
            let _ = std::fs::write(&failure_path, msg);
        }
    }
    let _ = std::fs::remove_dir_all(&work_dir);
    result
}

fn run_pdflatex(work_dir: &Path) -> Result<(), String> {
    let mut child = Command::new("pdflatex")
        .args(["-interaction=nonstopmode", "-halt-on-error", "-no-shell-escape", "snippet.tex"])
        .current_dir(work_dir)
        .env("PATH", get_latex_path_env())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("pdflatex could not be started: {e}"))?;

    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() > COMPILE_TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("pdflatex timed out".to_string());
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(15)),
            Err(e) => return Err(format!("pdflatex failed: {e}")),
        }
    };
    if status.success() {
        return Ok(());
    }
    let log = std::fs::read_to_string(work_dir.join("snippet.log")).unwrap_or_default();
    Err(first_latex_error(&log).unwrap_or_else(|| "LaTeX error".to_string()))
}

/// First `! ...` error line from a LaTeX log, e.g. "Undefined control sequence."
pub fn first_latex_error(log: &str) -> Option<String> {
    let mut lines = log.lines();
    while let Some(line) = lines.next() {
        if let Some(msg) = line.strip_prefix("! ") {
            let mut msg = msg.trim().to_string();
            // The offending token appears on the following "l.<n> ..." line.
            if let Some(ctx) = lines.find(|l| l.starts_with("l.")) {
                if let Some((_, tail)) = ctx.split_once(' ') {
                    let tail = tail.trim();
                    if !tail.is_empty() {
                        let n = tail.chars().count();
                        let tail: String = tail.chars().skip(n.saturating_sub(40)).collect();
                        msg.push_str(&format!(" — {tail}"));
                    }
                }
            }
            return Some(msg);
        }
    }
    None
}

fn rasterize(pdf_path: &Path, png_path: &Path) -> Result<(), String> {
    let pdf = pdf_path.to_str().ok_or("Invalid path")?;
    let doc = mupdf::Document::open(pdf).map_err(|e| e.to_string())?;
    let page = doc.load_page(0).map_err(|e| e.to_string())?;
    let matrix = mupdf::Matrix::new_scale(RASTER_SCALE, RASTER_SCALE);
    let pixmap = page
        .to_pixmap(&matrix, &mupdf::Colorspace::device_rgb(), true, true)
        .map_err(|e| e.to_string())?;
    // Write to a temp name first so a half-written PNG is never picked up from the cache.
    let tmp = png_path.with_extension("png.tmp");
    pixmap
        .save_as(tmp.to_str().ok_or("Invalid path")?, mupdf::ImageFormat::PNG)
        .map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, png_path).map_err(|e| e.to_string())
}
