//! Minimal, self-contained source tarballs for arXiv or journal submission.
//!
//! The project is copied into a throw-away directory and compiled with
//! `latexmk -recorder`; the `.fls` file lists every project file TeX actually
//! read (sources, class files, figures, ...). Only those files go into the
//! tarball, together with the `.bbl` (and, for journals, the `.bib`/`.bst`).
//! Comments are stripped from the `.tex` files and the `.bib` is reduced to
//! the cited entries. The tarball is then unpacked into a second clean
//! directory and compiled again to check that it is complete.
//!
//! All builds pin TeX's timestamps via `SOURCE_DATE_EPOCH`, so identical input
//! gives a byte-identical PDF; the PDF built from the tarball is compared with
//! the one built from the untouched sources, page by page if the bytes differ.
//!
//! Each run creates `<output_root>/<project>_<target>_<date>[_vN]/` with
//! `<name>.tar.gz`, `<name>.pdf`, `<name>.sha256` and `manifest.txt`, and
//! optionally tags the packaged commit as `submitted/<name>`.

use crate::compiler::get_latex_path_env;
use crate::latexdiff::{export_revision, log_errors, TempDir};
use crate::project::{detect_engine_for, Engine};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum SubmissionTarget {
    /// Sources + `.bbl` (arXiv does not run BibTeX); checked with the engine alone.
    Arxiv,
    /// Sources + `.bbl` + `.bib` + `.bst`; checked with the full latexmk pipeline.
    Journal,
}

impl SubmissionTarget {
    fn name(self) -> &'static str {
        match self {
            SubmissionTarget::Arxiv => "arxiv",
            SubmissionTarget::Journal => "journal",
        }
    }

    fn title(self) -> &'static str {
        match self {
            SubmissionTarget::Arxiv => "arXiv",
            SubmissionTarget::Journal => "Journal",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SubmissionOptions {
    pub target: SubmissionTarget,
    /// Package the last commit instead of the files on disk.
    pub from_head: bool,
    pub strip_comments: bool,
    /// Ship only the cited `.bib` entries.
    pub prune_bib: bool,
    /// Tag the packaged commit `submitted/<name>` (skipped when packaged files are uncommitted).
    pub tag: bool,
}

#[derive(Debug, Clone)]
pub struct SubmissionRequest {
    pub project_dir: PathBuf,
    /// Main document, relative to `project_dir`.
    pub main_rel: String,
    pub options: SubmissionOptions,
    /// Folder in which the submission folder is created.
    pub output_root: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SubmissionReport {
    pub folder: String,
    pub tarball: String,
    /// The PDF the tarball compiles to (absent when the check build failed).
    pub pdf: Option<String>,
    pub size_bytes: u64,
    /// Packaged files, relative to the main document's folder.
    pub files: Vec<String>,
    /// What was cleaned up (comments stripped, entries pruned).
    pub steps: Vec<String>,
    /// Outcome of the check build and the PDF comparison.
    pub verification: Vec<String>,
    pub warnings: Vec<String>,
    /// The git tag created on the packaged commit.
    pub tag: Option<String>,
}

/// Never copied into the build directory.
const IGNORE: &[&str] = &[".git", "submitted_versions", ".DS_Store", "__pycache__", ".ipynb_checkpoints"];

/// Files TeX writes itself; never shipped even though TeX reads them back.
const GENERATED_SUFFIXES: &[&str] = &[
    "aux", "out", "toc", "lof", "lot", "nav", "snm", "fls", "fdb_latexmk", "log", "blg", "bcf", "run.xml", "xdv",
];

fn is_generated(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    name.ends_with(".synctex.gz") || GENERATED_SUFFIXES.iter().any(|s| name.ends_with(&format!(".{s}")))
}

fn ext(path: &Path) -> &str {
    path.extension().and_then(|e| e.to_str()).unwrap_or("")
}

fn engine_binary(engine: Engine) -> &'static str {
    match engine {
        Engine::Pdf => "pdflatex",
        Engine::Xe => "xelatex",
        Engine::Lua => "lualatex",
    }
}

/// TeX command with the TeX Live PATH and pinned timestamps; output is discarded.
fn tex_command(program: &str, dir: &Path, epoch: &str) -> Command {
    let mut cmd = Command::new(program);
    cmd.env("PATH", get_latex_path_env())
        .env("SOURCE_DATE_EPOCH", epoch)
        .env("FORCE_SOURCE_DATE", "1")
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    cmd
}

fn run(mut cmd: Command, what: &str) -> Result<bool, String> {
    cmd.status().map(|s| s.success()).map_err(|e| format!("Could not run {what}: {e}"))
}

fn read_lossy(path: &Path) -> String {
    fs::read(path).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default()
}

// ── Source preparation ──────────────────────────────────────────────────────

/// Copies `src` into `dest`, skipping VCS folders, `skip` and build products.
fn copy_tree(src: &Path, dest: &Path, skip: &Path) -> Result<(), String> {
    let walker = walkdir::WalkDir::new(src).into_iter().filter_entry(|e| {
        let name = e.file_name().to_str().unwrap_or("");
        e.depth() == 0 || !(IGNORE.contains(&name) || e.path() == skip)
    });
    for entry in walker {
        let entry = entry.map_err(|e| e.to_string())?;
        let rel = entry.path().strip_prefix(src).map_err(|e| e.to_string())?;
        let target = dest.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target).map_err(|e| e.to_string())?;
        } else if entry.file_type().is_file() && !is_stale(entry.path()) {
            fs::copy(entry.path(), &target).map_err(|e| format!("Could not copy {}: {e}", rel.display()))?;
        }
    }
    Ok(())
}

/// Build products left over from earlier builds; a stale `.aux`/`.bbl` could hide problems.
fn is_stale(path: &Path) -> bool {
    is_generated(path) || ext(path) == "bbl"
}

fn remove_stale(dir: &Path) {
    for entry in walkdir::WalkDir::new(dir).into_iter().flatten() {
        if entry.file_type().is_file() && is_stale(entry.path()) {
            let _ = fs::remove_file(entry.path());
        }
    }
    for name in IGNORE {
        let _ = fs::remove_dir_all(dir.join(name));
    }
}

/// Fills a fresh directory with the project (or commit `head`) and returns the project folder
/// inside it.
fn prepare_source(req: &SubmissionRequest, head: Option<&str>, dest: &Path) -> Result<PathBuf, String> {
    if req.options.from_head {
        let root = crate::git::repo_root(&req.project_dir).ok_or("The project is not in a git repository")?;
        let head = head.ok_or("The project has no commits yet")?;
        let scope = crate::git::relative_path(&root, &req.project_dir).unwrap_or_default();
        export_revision(&root, head, &scope, dest)?;
        let dir = dest.join(&scope);
        remove_stale(&dir);
        Ok(dir)
    } else {
        copy_tree(&req.project_dir, dest, &req.output_root)?;
        Ok(dest.to_path_buf())
    }
}

/// Project files that TeX read, according to the `.fls` recorder file, relative to `root`.
/// Also returns files read from outside `root` (they can't be packaged).
fn recorded_inputs(root: &Path, project: &Path, stem: &str) -> (BTreeSet<PathBuf>, BTreeSet<PathBuf>) {
    let fls = read_lossy(&root.join(format!("{stem}.fls")));
    let mut written = HashSet::new();
    let mut read = Vec::new();
    for line in fls.lines() {
        let (list, path) = if let Some(p) = line.strip_prefix("INPUT ") {
            (&mut read, p)
        } else if let Some(p) = line.strip_prefix("OUTPUT ") {
            written.insert(root.join(p.trim()).canonicalize().unwrap_or_default());
            continue;
        } else {
            continue;
        };
        list.push(root.join(path.trim()));
    }
    let mut files = BTreeSet::new();
    let mut outside = BTreeSet::new();
    for path in read {
        // Canonical paths so /var vs /private/var style differences don't matter.
        let Ok(full) = path.canonicalize() else { continue };
        if !full.is_file() || is_generated(&full) || written.contains(&full) {
            continue;
        }
        if let Ok(rel) = full.strip_prefix(root) {
            files.insert(rel.to_path_buf());
        } else if let Ok(rel) = full.strip_prefix(project) {
            outside.insert(rel.to_path_buf());
        }
        // Anything else is a TeX distribution file.
    }
    (files, outside)
}

/// `.bib` and `.bst` files that BibTeX or Biber read, according to the `.blg` file.
fn bibliography_inputs(root: &Path, stem: &str) -> BTreeSet<PathBuf> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r"(?m)Database file #\d+: (\S+)|The style file: (\S+)|Found BibTeX data source '([^']+)'").unwrap()
    });
    let blg = read_lossy(&root.join(format!("{stem}.blg")));
    re.captures_iter(&blg)
        .filter_map(|c| c.get(1).or(c.get(2)).or(c.get(3)))
        .map(|m| PathBuf::from(m.as_str()))
        .filter_map(|p| {
            let full = root.join(&p).canonicalize().ok()?;
            Some(full.strip_prefix(root).ok()?.to_path_buf())
        })
        .collect()
}

// ── Comment stripping ───────────────────────────────────────────────────────

/// Commands whose first argument may legitimately contain a bare `%`.
const BRACED_ARG_COMMANDS: &[&str] = &["\\url", "\\href"];
const DELIMITED_COMMANDS: &[&str] = &["\\verb", "\\lstinline"];

/// Byte index of the first unescaped `%` that starts a comment.
fn comment_start(line: &str) -> Option<usize> {
    let b = line.as_bytes();
    let n = b.len();
    let mut i = 0;
    while i < n {
        match b[i] {
            b'\\' => {
                let mut j = i + 1;
                while j < n && b[j].is_ascii_alphabetic() {
                    j += 1;
                }
                if j == i + 1 {
                    i += 2; // \%, \\, \{ ...
                    continue;
                }
                let name = &line[i..j];
                let starred = j < n && b[j] == b'*';
                if starred {
                    j += 1;
                }
                if DELIMITED_COMMANDS.contains(&name) && j < n {
                    let delim = b[j];
                    i = b[j + 1..].iter().position(|&c| c == delim).map_or(n, |p| j + p + 2);
                    continue;
                }
                if !starred && BRACED_ARG_COMMANDS.contains(&name) {
                    let mut k = j;
                    while k < n && b[k] == b' ' {
                        k += 1;
                    }
                    if k < n && b[k] == b'{' {
                        let mut depth = 0;
                        while k < n {
                            if b[k] == b'\\' {
                                k += 2;
                                continue;
                            }
                            match b[k] {
                                b'{' => depth += 1,
                                b'}' => depth -= 1,
                                _ => {}
                            }
                            k += 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        i = k;
                        continue;
                    }
                }
                i = j;
            }
            b'%' => return Some(i),
            _ => i += 1,
        }
    }
    None
}

/// Removes comments, leaving `\%`, `%` inside `\url`/`\href`/`\verb` and verbatim-like
/// environments alone. A trailing `%` that suppresses an end-of-line space is kept, as
/// are `%!` magic comments.
pub fn strip_comments(text: &str) -> String {
    static BEGIN: OnceLock<Regex> = OnceLock::new();
    let begin_verbatim = BEGIN.get_or_init(|| {
        Regex::new(r"\\begin\{(verbatim\*?|Verbatim\*?|lstlisting|minted|comment|filecontents\*?)\}").unwrap()
    });
    let mut out = Vec::new();
    let mut verbatim_end: Option<String> = None;
    for line in text.lines() {
        if let Some(end) = &verbatim_end {
            out.push(line.to_string());
            if line.contains(end.as_str()) {
                verbatim_end = None;
            }
            continue;
        }
        let start = comment_start(line);
        let code = start.map_or(line, |s| &line[..s]);
        if let Some(m) = begin_verbatim.captures(code) {
            let whole = m.get(0).unwrap();
            let end = format!("\\end{{{}}}", &m[1]);
            if !line[whole.end()..].contains(&end) {
                verbatim_end = Some(end);
            }
            out.push(line.to_string());
            continue;
        }
        match start {
            None => out.push(line.to_string()),
            Some(_) if code.trim().is_empty() => {
                if line.trim_start().starts_with("%!") {
                    out.push(line.to_string());
                }
            }
            // Keep the % itself: it suppresses the end-of-line space.
            Some(_) => out.push(format!("{code}%")),
        }
    }
    let mut s = out.join("\n");
    s.push('\n');
    s
}

// ── Bibliography pruning ────────────────────────────────────────────────────

/// Keys cited in the `.aux` files under `root` (BibTeX and biblatex).
fn cited_keys(root: &Path) -> HashSet<String> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"\\citation\{([^}]*)\}|\\abx@aux@cite\{(?:\d+\}\{)?([^}]*)\}").unwrap());
    let mut keys = HashSet::new();
    for entry in walkdir::WalkDir::new(root).into_iter().flatten() {
        if ext(entry.path()) != "aux" {
            continue;
        }
        for c in re.captures_iter(&read_lossy(entry.path())) {
            let list = c.get(1).or(c.get(2)).map_or("", |m| m.as_str());
            keys.extend(list.split(',').map(|k| k.trim().to_string()).filter(|k| !k.is_empty()));
        }
    }
    keys
}

struct BibEntry<'a> {
    kind: String,
    /// Empty for `@string`, `@preamble` and `@comment`.
    key: &'a str,
    raw: &'a str,
}

fn bib_entries(text: &str) -> Vec<BibEntry<'_>> {
    static HEAD: OnceLock<Regex> = OnceLock::new();
    let head_re = HEAD.get_or_init(|| Regex::new(r"^@\s*(\w+)\s*([{(])").unwrap());
    let b = text.as_bytes();
    let mut entries = Vec::new();
    let mut i = 0;
    while let Some(off) = text[i..].find('@') {
        let at = i + off;
        let Some(head) = head_re.captures(&text[at..]) else {
            i = at + 1;
            continue;
        };
        let kind = head[1].to_ascii_lowercase();
        let closer = if &head[2] == "{" { b'}' } else { b')' };
        let mut j = at + head.get(0).unwrap().end();
        let body_start = j;
        let mut depth = 0;
        while j < b.len() {
            match b[j] {
                b'{' => depth += 1,
                b'}' if depth > 0 => depth -= 1,
                c if c == closer && depth == 0 => break,
                _ => {}
            }
            j += 1;
        }
        let end = (j + 1).min(b.len());
        let body = &text[body_start..j.min(b.len())];
        let key = match kind.as_str() {
            "string" | "preamble" | "comment" => "",
            _ => body.split(',').next().unwrap_or("").trim(),
        };
        entries.push(BibEntry { kind, key, raw: &text[at..end] });
        i = end;
    }
    entries
}

/// `text` reduced to the entries whose keys are in `keys`, plus `@string`/`@preamble`
/// and the parents those entries refer to. Returns (new text, kept, total).
pub fn prune_bib(text: &str, keys: &HashSet<String>) -> (String, usize, usize) {
    static PARENT: OnceLock<Regex> = OnceLock::new();
    let parent_re =
        PARENT.get_or_init(|| Regex::new(r#"(?i)\b(?:crossref|xref|xdata|related)\s*=\s*[{"]([^}"]+)"#).unwrap());
    let entries = bib_entries(text);
    let by_key: HashMap<String, &str> =
        entries.iter().filter(|e| !e.key.is_empty()).map(|e| (e.key.to_lowercase(), e.raw)).collect();
    let mut wanted: HashSet<String> = keys.iter().map(|k| k.to_lowercase()).collect();
    let mut pending: Vec<String> = wanted.iter().cloned().collect();
    while let Some(key) = pending.pop() {
        let Some(raw) = by_key.get(&key) else { continue };
        for c in parent_re.captures_iter(raw) {
            for parent in c[1].split(',').map(|p| p.trim().to_lowercase()) {
                if !parent.is_empty() && wanted.insert(parent.clone()) {
                    pending.push(parent);
                }
            }
        }
    }
    let is_kept = |e: &BibEntry| !e.key.is_empty() && wanted.contains(&e.key.to_lowercase());
    let kept: Vec<&str> =
        entries.iter().filter(|e| e.kind == "string" || e.kind == "preamble" || is_kept(e)).map(|e| e.raw).collect();
    let total = entries.iter().filter(|e| !e.key.is_empty()).count();
    let kept_count = entries.iter().filter(|e| is_kept(e)).count();
    (kept.join("\n\n") + "\n", kept_count, total)
}

// ── Packaging and verification ──────────────────────────────────────────────

fn sha256(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    Ok(Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect())
}

/// A reproducible `.tar.gz` (fixed owner, mode and mtime) of `files` under `root`.
fn write_tarball(out: &Path, root: &Path, files: &BTreeSet<PathBuf>) -> Result<(), String> {
    let file = fs::File::create(out).map_err(|e| format!("Could not create {}: {e}", out.display()))?;
    let gz = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut tar = tar::Builder::new(gz);
    tar.mode(tar::HeaderMode::Deterministic);
    for rel in files {
        tar.append_path_with_name(root.join(rel), rel).map_err(|e| format!("Could not add {}: {e}", rel.display()))?;
    }
    tar.into_inner().and_then(|gz| gz.finish()).map_err(|e| e.to_string())?;
    Ok(())
}

fn unpack(tarball: &Path, dest: &Path) -> Result<(), String> {
    let file = fs::File::open(tarball).map_err(|e| e.to_string())?;
    tar::Archive::new(flate2::read::GzDecoder::new(file)).unpack(dest).map_err(|e| format!("Could not unpack: {e}"))
}

/// Page count and suspicious lines from a TeX log.
fn check_log(log: &str) -> (Option<u32>, Vec<String>) {
    static PAGES: OnceLock<Regex> = OnceLock::new();
    static PROBLEM: OnceLock<Regex> = OnceLock::new();
    let pages_re = PAGES.get_or_init(|| Regex::new(r"Output written on .*? \((\d+) pages?").unwrap());
    let problem_re = PROBLEM.get_or_init(|| Regex::new(r"undefined|File .* not found|Missing character").unwrap());
    let pages = pages_re.captures(log).and_then(|c| c[1].parse().ok());
    let problems = log.lines().filter(|l| problem_re.is_match(l)).map(|l| l.trim().to_string()).collect();
    (pages, problems)
}

/// Compares two PDFs, strictest test first. Returns (identical, message).
fn compare_pdfs(reference: &Path, candidate: &Path) -> (bool, String) {
    let (Ok(a), Ok(b)) = (sha256(reference), sha256(candidate)) else {
        return (false, "Could not read the PDFs to compare them.".to_string());
    };
    if a == b {
        return (true, format!("PDF is byte-identical to the build from the original sources (SHA-256 {a})."));
    }
    let open = |p: &Path| mupdf::Document::open(p.to_str()?).ok();
    let (Some(ref_doc), Some(new_doc)) = (open(reference), open(candidate)) else {
        return (false, "PDF bytes differ from the original build and could not be rendered.".to_string());
    };
    let (n_ref, n_new) = (ref_doc.page_count().unwrap_or(0), new_doc.page_count().unwrap_or(0));
    if n_ref != n_new {
        return (false, format!("PDF differs from the original build: {n_ref} vs {n_new} pages."));
    }
    let matrix = mupdf::Matrix::new_scale(100.0 / 72.0, 100.0 / 72.0);
    let rgb = mupdf::Colorspace::device_rgb();
    let render = |doc: &mupdf::Document, i: i32| -> Option<Vec<u8>> {
        let pixmap = doc.load_page(i).ok()?.to_pixmap(&matrix, &rgb, false, true).ok()?;
        Some(pixmap.samples().to_vec())
    };
    let changed: Vec<String> = (0..n_ref)
        .filter(|&i| render(&ref_doc, i).is_none() || render(&ref_doc, i) != render(&new_doc, i))
        .map(|i| (i + 1).to_string())
        .collect();
    if changed.is_empty() {
        (true, "PDF bytes differ from the original build, but every page renders pixel-identically.".to_string())
    } else {
        (false, format!("PDF differs from the original build on page(s) {}.", changed.join(", ")))
    }
}

/// A new `<project>_<target>_<date>[_vN]` folder name that does not exist yet.
fn submission_dir(parent: &Path, project: &str, target: SubmissionTarget) -> PathBuf {
    let base = format!("{project}_{}_{}", target.name(), chrono::Local::now().format("%Y-%m-%d"));
    let mut path = parent.join(&base);
    let mut version = 2;
    while path.exists() {
        path = parent.join(format!("{base}_v{version}"));
        version += 1;
    }
    path
}

// ── Git tag ─────────────────────────────────────────────────────────────────

/// Packaged files (relative to `dir`) that are modified, untracked or ignored, i.e. not
/// as committed. The generated `.bbl` is not checked.
fn uncommitted(dir: &Path, files: &BTreeSet<PathBuf>) -> Result<Vec<String>, String> {
    let paths: Vec<&PathBuf> = files.iter().filter(|f| ext(f) != "bbl").collect();
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    let out = Command::new("git")
        .args(["status", "--porcelain", "-z", "--untracked-files=all", "--ignored=matching", "--"])
        .args(&paths)
        .current_dir(dir)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()
        .map_err(|e| format!("Could not run git: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    // Entries are "XY path"; a rename's original path follows as a bare entry.
    Ok(String::from_utf8_lossy(&out.stdout)
        .split('\0')
        .filter(|e| e.len() > 3 && e.as_bytes()[2] == b' ')
        .map(|e| e[3..].to_string())
        .collect())
}

/// `submitted/<folder name>`, made a valid ref name and unique among the tags.
fn tag_name(repo: &Path, folder_name: &str) -> String {
    let clean: String =
        folder_name.chars().map(|c| if c.is_ascii_alphanumeric() || "._-".contains(c) { c } else { '-' }).collect();
    let base = format!("submitted/{}", clean.trim_start_matches(['.', '-']).replace("..", "."));
    let mut name = base.clone();
    let mut n = 2;
    while crate::git::tag_exists(repo, &name) {
        name = format!("{base}-{n}");
        n += 1;
    }
    name
}

/// Short description of the commit being packaged.
fn git_state(project: &Path, from_head: bool) -> String {
    let git = |args: &[&str]| {
        Command::new("git")
            .args(args)
            .current_dir(project)
            .env("GIT_OPTIONAL_LOCKS", "0")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    };
    let Some(commit) = git(&["rev-parse", "HEAD"]) else { return "not a git repository".to_string() };
    if from_head {
        return format!("{commit} (committed version)");
    }
    let dirty = git(&["status", "--porcelain", "--untracked-files=no", "--", "."]).is_some_and(|s| !s.is_empty());
    format!("{commit}{}", if dirty { " + uncommitted changes" } else { " (clean working tree)" })
}

/// Builds the submission folder and returns what was done.
pub fn prepare(req: &SubmissionRequest) -> Result<SubmissionReport, String> {
    let opts = &req.options;
    let main_rel = Path::new(&req.main_rel);
    let main_name = main_rel.file_name().and_then(|n| n.to_str()).ok_or("Invalid main document")?;
    let stem = main_rel.file_stem().and_then(|n| n.to_str()).unwrap_or("main").to_string();
    let main_dir_rel = main_rel.parent().unwrap_or(Path::new(""));
    let engine = detect_engine_for(&req.project_dir.join(main_rel));
    let epoch = chrono::Utc::now().timestamp().to_string();

    // Resolved once, so a commit made meanwhile can't end up packaged or tagged.
    let head = crate::git::head_commit(&req.project_dir);

    let tmp = TempDir::new("vortex-submission")?;
    let tmp_root = tmp.0.canonicalize().map_err(|e| e.to_string())?;
    let (build_root, verify_dir) = (tmp_root.join("build"), tmp_root.join("verify"));
    fs::create_dir_all(&build_root).map_err(|e| e.to_string())?;
    fs::create_dir_all(&verify_dir).map_err(|e| e.to_string())?;

    let project = prepare_source(req, head.as_deref(), &build_root)?;
    // Everything is packaged relative to the main document's folder, as arXiv expects.
    let root = project.join(main_dir_rel);
    if !root.join(main_name).is_file() {
        let source = if opts.from_head { "the last commit" } else { "the project" };
        return Err(format!("{} does not exist in {source}", req.main_rel));
    }

    let mut cmd = tex_command("latexmk", &root, &epoch);
    cmd.args([engine.latexmk_flag(), "-recorder", "-interaction=nonstopmode", "-halt-on-error", main_name]);
    if !run(cmd, "latexmk")? || !root.join(format!("{stem}.pdf")).is_file() {
        let errors = log_errors(&read_lossy(&root.join(format!("{stem}.log"))), 5);
        return Err(if errors.is_empty() {
            "The document does not compile.".to_string()
        } else {
            format!("The document does not compile:\n{}", errors.join("\n"))
        });
    }

    let mut warnings = Vec::new();
    let mut steps = Vec::new();
    let (mut files, outside) = recorded_inputs(&root, &project, &stem);
    if !outside.is_empty() {
        let list: Vec<String> = outside.iter().map(|p| p.display().to_string()).collect();
        warnings.push(format!(
            "Files outside the main document's folder can't be packaged: {}. Move them next to {main_name}.",
            list.join(", ")
        ));
    }
    let bbl = PathBuf::from(format!("{stem}.bbl"));
    if root.join(&bbl).is_file() {
        files.insert(bbl);
    }
    if opts.target == SubmissionTarget::Journal {
        files.extend(bibliography_inputs(&root, &stem));
    }
    let reference_pdf = tmp_root.join("reference.pdf");
    fs::copy(root.join(format!("{stem}.pdf")), &reference_pdf).map_err(|e| e.to_string())?;

    // The build directory is a throw-away copy, so files are cleaned in place.
    if opts.strip_comments {
        for rel in files.iter().filter(|f| ext(f) == "tex") {
            let path = root.join(rel);
            let before = read_lossy(&path);
            let after = strip_comments(&before);
            if before != after {
                fs::write(&path, &after).map_err(|e| e.to_string())?;
                let removed = before.lines().count().saturating_sub(after.lines().count());
                steps.push(format!("Stripped comments from {} ({removed} lines removed)", rel.display()));
            }
        }
    }
    if opts.prune_bib {
        let keys = cited_keys(&root);
        for rel in files.iter().filter(|f| ext(f) == "bib") {
            if keys.contains("*") {
                steps.push(format!("\\nocite{{*}} found; kept all of {}", rel.display()));
                continue;
            }
            let path = root.join(rel);
            let (text, kept, total) = prune_bib(&read_lossy(&path), &keys);
            fs::write(&path, text).map_err(|e| e.to_string())?;
            steps.push(format!("Pruned {}: kept {kept} of {total} entries", rel.display()));
        }
    }

    let project_name = req.project_dir.file_name().and_then(|n| n.to_str()).unwrap_or("paper");
    let folder = submission_dir(&req.output_root, project_name, opts.target);
    let name = folder.file_name().and_then(|n| n.to_str()).unwrap_or("submission").to_string();
    fs::create_dir_all(&folder).map_err(|e| format!("Could not create {}: {e}", folder.display()))?;
    let tarball = folder.join(format!("{name}.tar.gz"));
    write_tarball(&tarball, &root, &files)?;
    let size_bytes = fs::metadata(&tarball).map(|m| m.len()).unwrap_or(0);

    // Check that the tarball compiles on its own.
    unpack(&tarball, &verify_dir)?;
    let ok = match opts.target {
        // arXiv uses the shipped .bbl and runs the engine repeatedly.
        SubmissionTarget::Arxiv => {
            let mut ok = true;
            for _ in 0..3 {
                let mut cmd = tex_command(engine_binary(engine), &verify_dir, &epoch);
                cmd.args(["-interaction=nonstopmode", main_name]);
                ok &= run(cmd, engine_binary(engine))?;
            }
            ok
        }
        SubmissionTarget::Journal => {
            let mut cmd = tex_command("latexmk", &verify_dir, &epoch);
            cmd.args([engine.latexmk_flag(), "-interaction=nonstopmode", main_name]);
            run(cmd, "latexmk")?
        }
    };
    let (pages, problems) = check_log(&read_lossy(&verify_dir.join(format!("{stem}.log"))));
    let verified_pdf = verify_dir.join(format!("{stem}.pdf"));
    let mut verification = Vec::new();
    let mut pdf = None;
    match pages {
        Some(pages) if ok && verified_pdf.is_file() => {
            verification.push(format!("Check build from the tarball OK: {pages} pages."));
            let (identical, message) = compare_pdfs(&reference_pdf, &verified_pdf);
            if identical {
                verification.push(message);
            } else {
                warnings.push(message);
            }
            // Keep the PDF the tarball produces, plus checksums of both, so the
            // submission can later be checked with `shasum -a 256 -c <name>.sha256`.
            let pdf_out = folder.join(format!("{name}.pdf"));
            fs::copy(&verified_pdf, &pdf_out).map_err(|e| e.to_string())?;
            let checksums = format!("{}  {name}.tar.gz\n{}  {name}.pdf\n", sha256(&tarball)?, sha256(&pdf_out)?);
            fs::write(folder.join(format!("{name}.sha256")), &checksums).map_err(|e| e.to_string())?;
            pdf = Some((pdf_out.to_string_lossy().to_string(), checksums));
        }
        _ => warnings.push("The check build from the tarball failed.".to_string()),
    }
    if !problems.is_empty() {
        let shown = problems.len().min(20);
        warnings.push(format!("The check build's log reports problems:\n  {}", problems[..shown].join("\n  ")));
    }
    if opts.target == SubmissionTarget::Arxiv {
        if size_bytes > 50_000_000 {
            warnings.push("arXiv's default size limit is 50 MB.".to_string());
        }
        if engine != Engine::Pdf {
            warnings.push(format!("arXiv compiles with pdfLaTeX by default; check that it accepts {}.", engine.name()));
        }
        if root.join(format!("{stem}.bcf")).is_file() {
            warnings.push(
                "biblatex: arXiv only uses the shipped .bbl if it was made with the same biber version as theirs."
                    .to_string(),
            );
        }
    }

    // Mark the packaged commit, but only when the tarball builds and matches that commit.
    let mut tag = None;
    if let (true, Some((_, checksums)), Some(head)) = (opts.tag, &pdf, &head) {
        let changed = if opts.from_head { Ok(Vec::new()) } else { uncommitted(&req.project_dir.join(main_dir_rel), &files) };
        match changed {
            Ok(changed) if changed.is_empty() => {
                let tag_name = tag_name(&req.project_dir, &name);
                let message = format!(
                    "{} submission {name}\n\nMain document: {}\nSHA-256:\n{checksums}",
                    opts.target.title(),
                    req.main_rel
                );
                match crate::git::create_tag(&req.project_dir, &tag_name, head, &message) {
                    Ok(()) => tag = Some(tag_name),
                    Err(e) => warnings.push(format!("Could not create the git tag: {e}")),
                }
            }
            Ok(changed) => {
                let shown: Vec<&str> = changed.iter().take(5).map(String::as_str).collect();
                let more = if changed.len() > 5 { format!(" and {} more", changed.len() - 5) } else { String::new() };
                warnings.push(format!(
                    "Not tagged in git: packaged files differ from the last commit ({}{more}). Commit them, or package the last commit.",
                    shown.join(", ")
                ));
            }
            Err(e) => warnings.push(format!("Not tagged in git: {e}")),
        }
    }

    let file_list: Vec<String> = files.iter().map(|p| p.display().to_string()).collect();
    let options: Vec<&str> = [
        (opts.from_head, "last commit"),
        (!opts.strip_comments, "comments kept"),
        (!opts.prune_bib, "all references kept"),
    ]
    .iter()
    .filter(|(on, _)| *on)
    .map(|(_, label)| *label)
    .collect();
    let mut manifest = vec![
        format!("Created:  {}", chrono::Local::now().format("%Y-%m-%dT%H:%M:%S")),
        format!("Target:   {}", opts.target.name()),
        format!("Main:     {}", req.main_rel),
        format!("Engine:   {}", engine.name()),
        format!("Options:  {}", if options.is_empty() { "(defaults)".to_string() } else { options.join(", ") }),
        format!("Source:   {}", git_state(&req.project_dir, opts.from_head)),
        format!("Tarball:  {name}.tar.gz ({:.1} MB, {} files)", size_bytes as f64 / 1e6, files.len()),
        format!("Git tag:  {}", tag.as_deref().unwrap_or("(none)")),
        String::new(),
    ];
    manifest.extend(steps.iter().cloned());
    manifest.extend(verification.iter().cloned());
    manifest.extend(warnings.iter().map(|w| format!("WARNING: {w}")));
    manifest.push(String::new());
    manifest.push("Files:".to_string());
    manifest.extend(file_list.iter().map(|f| format!("  {f}")));
    fs::write(folder.join("manifest.txt"), manifest.join("\n") + "\n").map_err(|e| e.to_string())?;

    Ok(SubmissionReport {
        folder: folder.to_string_lossy().to_string(),
        tarball: tarball.to_string_lossy().to_string(),
        pdf: pdf.map(|(path, _)| path),
        size_bytes,
        files: file_list,
        steps,
        verification,
        warnings,
        tag,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_comments_but_keeps_escapes_urls_and_verbatim() {
        let src = "\
%!TEX program = pdflatex
% a full-line comment
Text 50\\% off % note
\\url{http://a.b/%20c} % tail
\\verb|%x| y
\\begin{verbatim}
% kept
\\end{verbatim}
line\\\\% after a line break
";
        let out = strip_comments(src);
        assert_eq!(
            out,
            "\
%!TEX program = pdflatex
Text 50\\% off %
\\url{http://a.b/%20c} %
\\verb|%x| y
\\begin{verbatim}
% kept
\\end{verbatim}
line\\\\%
"
        );
    }

    #[test]
    fn prunes_uncited_entries_and_keeps_crossref_parents() {
        let bib = r#"@string{jfm = "J. Fluid Mech."}
@article{a, title={A {nested} title}, journal=jfm}
@inproceedings{b, crossref={proc}}
@proceedings{proc, title={Proc}}
@article{unused, title={X}}
"#;
        let keys: HashSet<String> = ["a", "B"].iter().map(|s| s.to_string()).collect();
        let (out, kept, total) = prune_bib(bib, &keys);
        assert_eq!((kept, total), (3, 4));
        assert!(out.contains("@string{jfm") && out.contains("{proc,") && !out.contains("unused"));
        assert!(out.contains("A {nested} title"));
    }
}
