/// LaTeX preprocessor for grammar checking.
///
/// Converts a .tex buffer into plain English that `harper-core` can lint,
/// while building a source map so that lint byte-spans in the cleaned text
/// can be translated back to exact byte offsets in the original `.tex` file.
use std::collections::HashMap;

use crate::bibtex_parser::BibEntryItem;

// ---------------------------------------------------------------------------
// Source map
// ---------------------------------------------------------------------------

/// A checkpoint that maps a byte offset in the *cleaned* text to the
/// corresponding byte offset in the *original* `.tex` content.
#[derive(Debug, Clone, Copy)]
pub struct SourceMapEntry {
    pub clean_byte: usize,
    pub orig_byte: usize,
}

pub struct SourceMap(pub Vec<SourceMapEntry>);

impl SourceMap {
    /// Given a byte offset `cb` in the cleaned text, return the best
    /// matching byte offset in the original content.
    pub fn to_orig(&self, cb: usize) -> usize {
        let entries = &self.0;
        if entries.is_empty() {
            return cb;
        }
        match entries.binary_search_by_key(&cb, |e| e.clean_byte) {
            Ok(i) => entries[i].orig_byte,
            Err(0) => entries[0].orig_byte,
            Err(i) => {
                let prev = entries[i - 1];
                let delta = cb.saturating_sub(prev.clean_byte);
                prev.orig_byte + delta
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Environment classification
// ---------------------------------------------------------------------------

const IGNORED_ENVS: &[&str] = &[
    "equation", "equation*",
    "align", "align*", "aligned",
    "alignat", "alignat*",
    "flalign", "flalign*",
    "gather", "gather*",
    "multline", "multline*",
    "eqnarray", "eqnarray*",
    "math", "displaymath",
    "split", "cases",
    // Code listings
    "verbatim", "verbatim*",
    "lstlisting", "minted", "alltt",
    "BVerbatim", "LVerbatim", "Verbatim",
];

fn is_ignored_env(name: &str) -> bool {
    IGNORED_ENVS.contains(&name)
}

// ---------------------------------------------------------------------------
// BibTeX citation formatting
// ---------------------------------------------------------------------------

fn format_citation(author_field: &str, year: &str) -> String {
    let parts: Vec<&str> = author_field
        .split(" and ")
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    let last_names: Vec<String> = parts
        .iter()
        .map(|part| {
            if let Some(comma_pos) = part.find(',') {
                part[..comma_pos].trim().to_string()
            } else {
                part.split_whitespace().last().unwrap_or(part).to_string()
            }
        })
        .collect();

    let name_str = match last_names.len() {
        0 => "Author".to_string(),
        1 => last_names[0].clone(),
        2 => format!("{} and {}", last_names[0], last_names[1]),
        _ => format!("{} et al.", last_names[0]),
    };

    if year.is_empty() {
        name_str
    } else {
        format!("{} {}", name_str, year)
    }
}

fn resolve_cite(keys_str: &str, bib: &HashMap<String, BibEntryItem>) -> String {
    let keys: Vec<&str> = keys_str.split(',').map(str::trim).collect();
    let mut parts = Vec::new();
    for key in &keys {
        if let Some(entry) = bib.get(*key) {
            if let Some(ref fields) = entry.fields {
                let author = fields.author.as_deref().unwrap_or("");
                let year = fields.year.as_deref().unwrap_or("");
                parts.push(format_citation(author, year));
                continue;
            }
        }
        parts.push("Author et al. YEAR".to_string());
    }
    parts.join("; ")
}

// ---------------------------------------------------------------------------
// Math to Unicode converter
// ---------------------------------------------------------------------------

fn superscript_char(c: char) -> char {
    match c {
        '0' => '⁰',
        '1' => '¹',
        '2' => '²',
        '3' => '³',
        '4' => '⁴',
        '5' => '⁵',
        '6' => '⁶',
        '7' => '⁷',
        '8' => '⁸',
        '9' => '⁹',
        '+' => '⁺',
        '-' => '⁻',
        '=' => '⁼',
        '(' => '⁽',
        ')' => '⁾',
        'n' => 'ⁿ',
        'i' => 'ⁱ',
        other => other,
    }
}

fn subscript_char(c: char) -> char {
    match c {
        '0' => '₀',
        '1' => '₁',
        '2' => '₂',
        '3' => '₃',
        '4' => '₄',
        '5' => '₅',
        '6' => '₆',
        '7' => '₇',
        '8' => '₈',
        '9' => '₉',
        '+' => '₊',
        '-' => '₋',
        '=' => '₌',
        '(' => '₍',
        ')' => '₎',
        'a' => 'ₐ',
        'e' => 'ₑ',
        'o' => 'ₒ',
        'x' => 'ₓ',
        'i' => 'ᵢ',
        'j' => 'ⱼ',
        other => other,
    }
}

pub fn latex_math_to_unicode(math: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = math.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let c = chars[i];
        if c == '\\' {
            i += 1;
            let mut cmd = String::new();
            while i < len && chars[i].is_ascii_alphabetic() {
                cmd.push(chars[i]);
                i += 1;
            }
            while i < len && chars[i] == ' ' {
                i += 1;
            }
            match cmd.as_str() {
                "alpha" => out.push('α'),
                "beta" => out.push('β'),
                "gamma" => out.push('γ'),
                "delta" => out.push('δ'),
                "epsilon" | "varepsilon" => out.push('ε'),
                "zeta" => out.push('ζ'),
                "eta" => out.push('η'),
                "theta" | "vartheta" => out.push('θ'),
                "iota" => out.push('ι'),
                "kappa" => out.push('κ'),
                "lambda" => out.push('λ'),
                "mu" => out.push('μ'),
                "nu" => out.push('ν'),
                "xi" => out.push('ξ'),
                "pi" | "varpi" => out.push('π'),
                "rho" | "varrho" => out.push('ρ'),
                "sigma" | "varsigma" => out.push('σ'),
                "tau" => out.push('τ'),
                "upsilon" => out.push('υ'),
                "phi" | "varphi" => out.push('φ'),
                "chi" => out.push('χ'),
                "psi" => out.push('ψ'),
                "omega" => out.push('ω'),
                "Gamma" => out.push('Γ'),
                "Delta" => out.push('Δ'),
                "Theta" => out.push('Θ'),
                "Lambda" => out.push('Λ'),
                "Xi" => out.push('Ξ'),
                "Pi" => out.push('Π'),
                "Sigma" => out.push('Σ'),
                "Upsilon" => out.push('Υ'),
                "Phi" => out.push('Φ'),
                "Psi" => out.push('Ψ'),
                "Omega" => out.push('Ω'),
                "le" | "leq" => out.push('≤'),
                "ge" | "geq" => out.push('≥'),
                "ne" | "neq" => out.push('≠'),
                "approx" => out.push('≈'),
                "equiv" => out.push('≡'),
                "sim" => out.push('∼'),
                "propto" => out.push('∝'),
                "times" => out.push('×'),
                "cdot" => out.push('·'),
                "pm" => out.push('±'),
                "mp" => out.push('∓'),
                "div" => out.push('÷'),
                "in" => out.push('∈'),
                "notin" => out.push('∉'),
                "subset" => out.push('⊂'),
                "subseteq" => out.push('⊆'),
                "supset" => out.push('⊃'),
                "supseteq" => out.push('⊇'),
                "cap" => out.push('∩'),
                "cup" => out.push('∪'),
                "setminus" => out.push('∖'),
                "to" | "rightarrow" => out.push('→'),
                "leftarrow" => out.push('←'),
                "Rightarrow" => out.push('⇒'),
                "Leftarrow" => out.push('⇐'),
                "iff" | "Leftrightarrow" => out.push('⇔'),
                "mapsto" => out.push('↦'),
                "forall" => out.push('∀'),
                "exists" => out.push('∃'),
                "infty" | "inf" => out.push('∞'),
                "nabla" => out.push('∇'),
                "partial" => out.push('∂'),
                "sum" => out.push('∑'),
                "prod" => out.push('∏'),
                "int" => out.push('∫'),
                "oint" => out.push('∮'),
                "sqrt" => out.push('√'),
                "mathbb" => {
                    if i < len && chars[i] == '{' {
                        i += 1;
                        if i < len {
                            match chars[i] {
                                'R' => out.push('ℝ'),
                                'C' => out.push('ℂ'),
                                'N' => out.push('ℕ'),
                                'Z' => out.push('ℤ'),
                                'Q' => out.push('ℚ'),
                                other => out.push(other),
                            }
                            i += 1;
                        }
                        if i < len && chars[i] == '}' {
                            i += 1;
                        }
                    }
                }
                "frac" => {}
                "text" | "mathrm" | "mathbf" | "mathit" | "mathbfit" | "bm" => {}
                _ => {}
            }
        } else if c == '^' {
            i += 1;
            if i < len {
                if chars[i] == '{' {
                    i += 1;
                    while i < len && chars[i] != '}' {
                        out.push(superscript_char(chars[i]));
                        i += 1;
                    }
                    if i < len { i += 1; }
                } else {
                    out.push(superscript_char(chars[i]));
                    i += 1;
                }
            }
        } else if c == '_' {
            i += 1;
            if i < len {
                if chars[i] == '{' {
                    i += 1;
                    while i < len && chars[i] != '}' {
                        out.push(subscript_char(chars[i]));
                        i += 1;
                    }
                    if i < len { i += 1; }
                } else {
                    out.push(subscript_char(chars[i]));
                    i += 1;
                }
            }
        } else if c == '{' || c == '}' {
            i += 1;
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Main preprocessor
// ---------------------------------------------------------------------------

pub fn preprocess_latex(
    content: &str,
    bib: &HashMap<String, BibEntryItem>,
) -> (String, SourceMap) {
    let bytes = content.as_bytes();
    let len = bytes.len();

    let mut cleaned = String::with_capacity(len);
    let mut map: Vec<SourceMapEntry> = Vec::new();

    let mut i = 0usize;
    let mut env_stack: Vec<(String, bool)> = Vec::new();
    let mut in_line_comment = false;

    let currently_ignored = |stack: &[(String, bool)]| -> bool {
        stack.last().map(|(_, ign)| *ign).unwrap_or(false)
    };

    macro_rules! emit {
        ($s:expr, $ob:expr) => {
            let s: &str = $s;
            let ob: usize = $ob;
            if !s.is_empty() {
                map.push(SourceMapEntry { clean_byte: cleaned.len(), orig_byte: ob });
                cleaned.push_str(s);
            }
        };
    }

    while i < len {
        if bytes[i] == b'\n' {
            if in_line_comment {
                in_line_comment = false;
            }
            if !currently_ignored(&env_stack) {
                emit!("\n", i);
            }
            i += 1;
            continue;
        }

        if in_line_comment {
            i += 1;
            continue;
        }

        if bytes[i] == b'%' {
            in_line_comment = true;
            i += 1;
            continue;
        }

        // $$ display math - strip entirely and replace with a space
        if i + 1 < len && bytes[i] == b'$' && bytes[i + 1] == b'$' {
            i += 2;
            while i < len {
                if i + 1 < len && bytes[i] == b'$' && bytes[i + 1] == b'$' {
                    i += 2;
                    break;
                }
                if bytes[i] == b'\n' { emit!("\n", i); }
                i += 1;
            }
            emit!(" ", i);
            continue;
        }

        // $ inline math - convert to Unicode representation
        if bytes[i] == b'$' {
            let math_start = i + 1;
            i += 1;
            while i < len && bytes[i] != b'$' {
                if bytes[i] == b'\n' { emit!("\n", i); }
                i += 1;
            }
            let math_end = i;
            if i < len { i += 1; }
            if !currently_ignored(&env_stack) {
                let math_raw = std::str::from_utf8(&bytes[math_start..math_end]).unwrap_or("");
                let unicode_math = latex_math_to_unicode(math_raw);
                if !unicode_math.is_empty() {
                    emit!(&unicode_math, math_start);
                }
            }
            continue;
        }

        // LaTeX commands
        if bytes[i] == b'\\' {
            i += 1;

            // \\ line break
            if i < len && bytes[i] == b'\\' {
                i += 1;
                if !currently_ignored(&env_stack) {
                    emit!(" ", i);
                }
                continue;
            }

            // \[ ... \] display math
            if i < len && bytes[i] == b'[' {
                i += 1;
                while i < len {
                    if bytes[i] == b'\\' && i + 1 < len && bytes[i + 1] == b']' {
                        i += 2;
                        break;
                    }
                    if bytes[i] == b'\n' { emit!("\n", i); }
                    i += 1;
                }
                if !currently_ignored(&env_stack) { emit!(" ", i); }
                continue;
            }

            // \( ... \) inline math
            if i < len && bytes[i] == b'(' {
                let math_start = i + 1;
                i += 1;
                let math_end;
                while i < len {
                    if bytes[i] == b'\\' && i + 1 < len && bytes[i + 1] == b')' {
                        break;
                    }
                    if bytes[i] == b'\n' { emit!("\n", i); }
                    i += 1;
                }
                math_end = i;
                if i + 1 < len && bytes[i] == b'\\' && bytes[i + 1] == b')' {
                    i += 2;
                }
                if !currently_ignored(&env_stack) {
                    let math_raw = std::str::from_utf8(&bytes[math_start..math_end]).unwrap_or("");
                    let unicode_math = latex_math_to_unicode(math_raw);
                    if !unicode_math.is_empty() {
                        emit!(&unicode_math, math_start);
                    }
                }
                continue;
            }

            // Read command name
            let cmd_start = i;
            while i < len && bytes[i].is_ascii_alphabetic() { i += 1; }
            let cmd = std::str::from_utf8(&bytes[cmd_start..i]).unwrap_or("");
            if i < len && bytes[i] == b'*' { i += 1; }
            while i < len && bytes[i] == b' ' { i += 1; }

            match cmd {
                "begin" => {
                    if i < len && bytes[i] == b'{' {
                        i += 1;
                        let es = i;
                        while i < len && bytes[i] != b'}' { i += 1; }
                        let env_name = std::str::from_utf8(&bytes[es..i]).unwrap_or("").trim().to_string();
                        if i < len { i += 1; }
                        let ignored = is_ignored_env(&env_name);
                        env_stack.push((env_name.clone(), ignored));
                        // Skip tabular column spec
                        if env_name.starts_with("tabular") || env_name == "array" || env_name == "longtable" {
                            while i < len && bytes[i] == b' ' { i += 1; }
                            if i < len && bytes[i] == b'[' {
                                i += 1;
                                while i < len && bytes[i] != b']' { i += 1; }
                                if i < len { i += 1; }
                            }
                            while i < len && bytes[i] == b' ' { i += 1; }
                            if i < len && bytes[i] == b'{' {
                                i += 1;
                                while i < len && bytes[i] != b'}' { i += 1; }
                                if i < len { i += 1; }
                            }
                        }
                    }
                    continue;
                }

                "end" => {
                    if i < len && bytes[i] == b'{' {
                        i += 1;
                        while i < len && bytes[i] != b'}' { i += 1; }
                        if i < len { i += 1; }
                    }
                    env_stack.pop();
                    continue;
                }

                "cite" | "citep" | "citet" | "autocite" | "parencite" | "textcite"
                | "citealt" | "citealp" | "citeauthor" | "citeyear" | "fullcite" => {
                    while i < len && bytes[i] == b'[' {
                        i += 1;
                        while i < len && bytes[i] != b']' { i += 1; }
                        if i < len { i += 1; }
                        while i < len && bytes[i] == b' ' { i += 1; }
                    }
                    if i < len && bytes[i] == b'{' {
                        let orig_ob = i;
                        i += 1;
                        let ks = i;
                        while i < len && bytes[i] != b'}' { i += 1; }
                        let keys_str = std::str::from_utf8(&bytes[ks..i]).unwrap_or("").trim();
                        if i < len { i += 1; }
                        if !currently_ignored(&env_stack) {
                            let citation_text = resolve_cite(keys_str, bib);
                            emit!(&citation_text, orig_ob);
                        }
                    }
                    continue;
                }

                "ref" | "eqref" | "cref" | "autoref" | "pageref" | "vref" | "Cref" => {
                    skip_all_args(&mut i, bytes, len);
                    if !currently_ignored(&env_stack) {
                        emit!("Section", i);
                    }
                    continue;
                }

                "label" | "index" | "bibliography" | "addbibresource" | "bibliographystyle"
                | "usepackage" | "documentclass" | "input" | "include" | "includegraphics"
                | "newcommand" | "renewcommand" | "providecommand" | "def"
                | "setlength" | "setcounter" | "addtocounter" | "pagenumbering"
                | "pagestyle" | "thispagestyle" | "newtheorem" | "theoremstyle" | "vspace"
                | "hspace" | "vskip" | "hskip" | "hrule" | "noindent"
                | "maketitle" | "tableofcontents" | "listoffigures" | "listoftables"
                | "appendix" | "centering" | "raggedright" | "raggedleft" | "clearpage"
                | "newpage" | "pagebreak" | "linebreak" | "smallskip" | "medskip"
                | "bigskip" | "hline" | "cline" | "toprule" | "midrule"
                | "bottomrule" | "multirow" | "multicolumn" | "rowcolor" | "columncolor"
                | "cellcolor" | "arrayrulecolor" => {
                    skip_all_args(&mut i, bytes, len);
                    continue;
                }

                "part" | "chapter" | "section" | "subsection" | "subsubsection"
                | "paragraph" | "subparagraph" => {
                    while i < len && bytes[i] == b'[' {
                        i += 1;
                        while i < len && bytes[i] != b']' { i += 1; }
                        if i < len { i += 1; }
                    }
                    if i < len && bytes[i] == b'{' {
                        let ob = i + 1;
                        i += 1;
                        let ts = i;
                        let mut depth = 1usize;
                        while i < len {
                            if bytes[i] == b'{' { depth += 1; }
                            else if bytes[i] == b'}' { depth -= 1; if depth == 0 { break; } }
                            i += 1;
                        }
                        let title = std::str::from_utf8(&bytes[ts..i]).unwrap_or("").trim();
                        if i < len { i += 1; }
                        if !currently_ignored(&env_stack) {
                            emit!(title, ob);
                            emit!("\n", i);
                        }
                    }
                    continue;
                }

                "caption" => {
                    while i < len && bytes[i] == b'[' {
                        i += 1;
                        while i < len && bytes[i] != b']' { i += 1; }
                        if i < len { i += 1; }
                    }
                    if i < len && bytes[i] == b'{' {
                        let ob = i + 1;
                        i += 1;
                        let cs = i;
                        let mut depth = 1usize;
                        while i < len {
                            if bytes[i] == b'{' { depth += 1; }
                            else if bytes[i] == b'}' { depth -= 1; if depth == 0 { break; } }
                            i += 1;
                        }
                        let cap = std::str::from_utf8(&bytes[cs..i]).unwrap_or("").trim();
                        if i < len { i += 1; }
                        if !currently_ignored(&env_stack) {
                            let (cap_clean, _) = preprocess_latex(cap, bib);
                            emit!(&cap_clean, ob);
                            emit!("\n", i);
                        }
                    }
                    continue;
                }

                "textbf" | "textit" | "emph" | "text" | "textrm" | "texttt" | "textsc"
                | "textsl" | "underline" | "uline" | "textcolor" | "colorbox"
                | "mbox" | "makebox" | "fbox" | "framebox" | "footnote" | "footnotemark"
                | "footnotetext" | "marginpar" | "thanks" | "author" | "title" | "date"
                | "textup" | "textnormal" | "textsf" | "MakeUppercase" | "MakeLowercase" => {
                    while i < len && bytes[i] == b'[' {
                        i += 1;
                        while i < len && bytes[i] != b']' { i += 1; }
                        if i < len { i += 1; }
                    }
                    if i < len && bytes[i] == b'{' {
                        let ob = i + 1;
                        i += 1;
                        let ins = i;
                        let mut depth = 1usize;
                        while i < len {
                            if bytes[i] == b'{' { depth += 1; }
                            else if bytes[i] == b'}' { depth -= 1; if depth == 0 { break; } }
                            i += 1;
                        }
                        let inner = std::str::from_utf8(&bytes[ins..i]).unwrap_or("");
                        if i < len { i += 1; }
                        if !currently_ignored(&env_stack) {
                            let (inner_clean, _) = preprocess_latex(inner, bib);
                            emit!(&inner_clean, ob);
                        }
                    }
                    continue;
                }

                _ => {
                    if !currently_ignored(&env_stack) {
                        emit!(" ", i);
                    }
                    continue;
                }
            }
        }

        if bytes[i] == b'{' || bytes[i] == b'}' {
            i += 1;
            continue;
        }

        // Table cell separators and non-breaking spaces read as plain spaces.
        if bytes[i] == b'&' || bytes[i] == b'~' {
            if !currently_ignored(&env_stack) {
                emit!(" ", i);
            }
            i += 1;
            continue;
        }

        // Regular character
        if !currently_ignored(&env_stack) {
            let ch_byte = bytes[i];
            let ch_len = if ch_byte < 0x80 { 1 }
                else if ch_byte < 0xE0 { 2 }
                else if ch_byte < 0xF0 { 3 }
                else { 4 };
            let ch_end = (i + ch_len).min(len);
            if let Ok(s) = std::str::from_utf8(&bytes[i..ch_end]) {
                map.push(SourceMapEntry { clean_byte: cleaned.len(), orig_byte: i });
                cleaned.push_str(s);
            }
            i = ch_end;
        } else {
            let ch_byte = bytes[i];
            let ch_len = if ch_byte < 0x80 { 1 }
                else if ch_byte < 0xE0 { 2 }
                else if ch_byte < 0xF0 { 3 }
                else { 4 };
            i = (i + ch_len).min(len);
        }
    }

    (cleaned, SourceMap(map))
}

/// Byte ranges of inline math (`$…$`, `\\(…\\)`) in `content`, delimiters included.
/// Its Unicode rendering is for sentence flow, not for spell checking.
pub fn inline_math_ranges(content: &str) -> Vec<std::ops::Range<usize>> {
    let b = content.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'%' => {
                while i < b.len() && b[i] != b'\n' { i += 1; }
            }
            b'\\' if i + 1 < b.len() && b[i + 1] == b'(' => {
                let start = i;
                i += 2;
                while i + 1 < b.len() && !(b[i] == b'\\' && b[i + 1] == b')') { i += 1; }
                i = (i + 2).min(b.len());
                out.push(start..i);
                continue;
            }
            b'\\' => i += 1, // skips escaped characters such as \$
            b'$' if i + 1 < b.len() && b[i + 1] == b'$' => {
                i += 2;
                while i + 1 < b.len() && !(b[i] == b'$' && b[i + 1] == b'$') { i += 1; }
                i += 1;
            }
            b'$' => {
                let start = i;
                i += 1;
                while i < b.len() && b[i] != b'$' {
                    if b[i] == b'\\' { i += 1; }
                    i += 1;
                }
                i = (i + 1).min(b.len());
                out.push(start..i);
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    out
}

fn skip_all_args(i: &mut usize, bytes: &[u8], len: usize) {
    loop {
        // Spaces are only skipped when an argument follows; otherwise they separate words.
        let before_spaces = *i;
        while *i < len && bytes[*i] == b' ' { *i += 1; }
        if *i >= len || (bytes[*i] != b'[' && bytes[*i] != b'{') {
            *i = before_spaces;
            break;
        }
        if bytes[*i] == b'[' {
            *i += 1;
            while *i < len && bytes[*i] != b']' { *i += 1; }
            if *i < len { *i += 1; }
        } else if bytes[*i] == b'{' {
            *i += 1;
            let mut depth = 1usize;
            while *i < len {
                if bytes[*i] == b'{' { depth += 1; }
                else if bytes[*i] == b'}' { depth -= 1; if depth == 0 { *i += 1; break; } }
                *i += 1;
            }
        } else {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pp(s: &str) -> String {
        preprocess_latex(s, &HashMap::new()).0
    }

    #[test]
    fn inline_math_replaced() {
        let out = pp("The value $x^2 + y^2$ is important.");
        assert!(out.contains("x² + y²"));
        assert!(!out.contains("expression"));
    }

    #[test]
    fn display_math_removed() {
        let out = pp("Before. $$E = mc^2$$ After.");
        assert!(!out.contains("mc^2"));
        assert!(out.contains("Before"));
        assert!(out.contains("After"));
    }

    #[test]
    fn equation_env_removed() {
        let out = pp(r"Text before. \begin{equation} x = y \end{equation} Text after.");
        assert!(!out.contains("x = y"));
        assert!(out.contains("Text before"));
        assert!(out.contains("Text after"));
    }

    #[test]
    fn abstract_env_kept() {
        let out = pp(r"\begin{abstract} This is a good summary. \end{abstract}");
        assert!(out.contains("This is a good summary"));
    }

    #[test]
    fn caption_kept() {
        let out = pp(r"\caption{A table of results.}");
        assert!(out.contains("A table of results"));
    }

    #[test]
    fn verbatim_excluded() {
        let out = pp(r"\begin{verbatim} fn main() {} \end{verbatim}");
        assert!(!out.contains("fn main"));
    }

    #[test]
    fn cite_unresolved() {
        let out = pp(r"As shown in \cite{somekey}, the result holds.");
        assert!(out.contains("Author et al. YEAR"));
    }

    #[test]
    fn refs_keep_the_following_space() {
        assert!(pp(r"See Section~\ref{sec:a} and more.").contains("Section Section and more"));
    }

    #[test]
    fn table_cells_are_separated_by_spaces() {
        assert!(pp(r"\begin{tabular}{ll} A & B \\ \end{tabular}").contains("A   B"));
    }

    #[test]
    fn finds_inline_math() {
        let t = r"a $x$ b \$ c \(y\) d $$z$$ % $w$";
        let r = inline_math_ranges(t);
        assert_eq!(r.iter().map(|r| &t[r.clone()]).collect::<Vec<_>>(), vec!["$x$", r"\(y\)"]);
    }

    #[test]
    fn source_map_roundtrip() {
        let tex = "Hello $x^2$ world.";
        let (cleaned, map) = preprocess_latex(tex, &HashMap::new());
        let world_cb = cleaned.find("world").unwrap();
        let orig_byte = map.to_orig(world_cb);
        assert_eq!(&tex[orig_byte..orig_byte + 5], "world");
    }
}
