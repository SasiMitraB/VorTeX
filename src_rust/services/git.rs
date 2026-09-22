//! Minimal git integration: repository status, per-file history, file contents
//! at a revision (all via the `git` CLI), and line diffs computed in-process.

use similar::{ChangeTag, DiffOp, TextDiff};
use std::path::{Path, PathBuf};
use std::process::Command;

fn git(root: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let out = Command::new("git")
        .args(args)
        .current_dir(root)
        .env("GIT_OPTIONAL_LOCKS", "0") // don't take index.lock for read-only status
        .output()
        .ok()?;
    out.status.success().then_some(out.stdout)
}

fn git_string(root: &Path, args: &[&str]) -> Option<String> {
    git(root, args).map(|b| String::from_utf8_lossy(&b).trim_end().to_string())
}

/// Top-level directory of the repository containing `path`, if any.
pub fn repo_root(path: &Path) -> Option<PathBuf> {
    let dir = if path.is_dir() { path } else { path.parent()? };
    git_string(dir, &["rev-parse", "--show-toplevel"]).map(PathBuf::from)
}

pub fn current_branch(root: &Path) -> Option<String> {
    git_string(root, &["rev-parse", "--abbrev-ref", "HEAD"])
}

/// `path` relative to `root`, with `/` separators (as git expects).
pub fn relative_path(root: &Path, path: &Path) -> Option<String> {
    // Canonicalize both so /private/tmp vs /tmp style differences don't matter.
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let rel = path.strip_prefix(&root).ok()?;
    Some(rel.to_string_lossy().replace('\\', "/"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
    Conflicted,
}

impl ChangeKind {
    pub fn letter(self) -> &'static str {
        match self {
            ChangeKind::Modified => "M",
            ChangeKind::Added => "A",
            ChangeKind::Deleted => "D",
            ChangeKind::Renamed => "R",
            ChangeKind::Untracked => "U",
            ChangeKind::Conflicted => "!",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileStatus {
    /// Path relative to the repository root.
    pub path: String,
    pub kind: ChangeKind,
}

/// Parses `git status --porcelain=v1 -z` output.
pub fn parse_status(raw: &[u8]) -> Vec<FileStatus> {
    let text = String::from_utf8_lossy(raw);
    let mut entries = text.split('\0').filter(|e| !e.is_empty());
    let mut out = Vec::new();
    while let Some(entry) = entries.next() {
        if entry.len() < 4 {
            continue;
        }
        let (x, y) = (entry.as_bytes()[0], entry.as_bytes()[1]);
        let path = entry[3..].to_string();
        let kind = match (x, y) {
            (b'?', b'?') => ChangeKind::Untracked,
            (b'U', _) | (_, b'U') | (b'A', b'A') | (b'D', b'D') => ChangeKind::Conflicted,
            (b'R', _) | (_, b'R') => {
                // Renames are followed by the original path as a separate entry.
                entries.next();
                ChangeKind::Renamed
            }
            (b'D', _) | (_, b'D') => ChangeKind::Deleted,
            (b'A', _) => ChangeKind::Added,
            _ => ChangeKind::Modified,
        };
        out.push(FileStatus { path, kind });
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

pub fn status(root: &Path) -> Vec<FileStatus> {
    git(root, &["status", "--porcelain=v1", "-z"]).map(|raw| parse_status(&raw)).unwrap_or_default()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit {
    pub hash: String,
    pub short_hash: String,
    pub author: String,
    pub relative_date: String,
    pub subject: String,
}

const LOG_FORMAT: &str = "--format=%H%x1f%h%x1f%an%x1f%ar%x1f%s";

pub fn parse_log(raw: &str) -> Vec<Commit> {
    raw.lines()
        .filter_map(|line| {
            let mut f = line.split('\u{1f}');
            Some(Commit {
                hash: f.next()?.to_string(),
                short_hash: f.next()?.to_string(),
                author: f.next()?.to_string(),
                relative_date: f.next()?.to_string(),
                subject: f.next().unwrap_or("").to_string(),
            })
        })
        .collect()
}

/// Most recent commits touching `rel_path` (following renames).
pub fn file_log(root: &Path, rel_path: &str, limit: usize) -> Vec<Commit> {
    let n = format!("-n{limit}");
    git_string(root, &["log", LOG_FORMAT, &n, "--follow", "--", rel_path])
        .map(|raw| parse_log(&raw))
        .unwrap_or_default()
}

/// Most recent commits touching anything under `scope` (repo-relative, "" = whole repo).
pub fn log(root: &Path, scope: &str, limit: usize) -> Vec<Commit> {
    let n = format!("-n{limit}");
    let spec = if scope.is_empty() { "." } else { scope };
    git_string(root, &["log", LOG_FORMAT, &n, "--", spec]).map(|raw| parse_log(&raw)).unwrap_or_default()
}

/// Contents of `rel_path` at `rev` (e.g. "HEAD" or a commit hash), if it exists there.
pub fn file_at_revision(root: &Path, rev: &str, rel_path: &str) -> Option<Vec<u8>> {
    let spec = format!("{rev}:{rel_path}");
    git(root, &["show", &spec])
}

/// Decodes file bytes as text; `None` for binary content.
pub fn as_text(bytes: &[u8]) -> Option<String> {
    if bytes.iter().take(8000).any(|&b| b == 0) {
        return None;
    }
    String::from_utf8(bytes.to_vec()).ok()
}

// ─────────────────────────────────────────────────────────────────────────────
// Gutter markers
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineChange {
    Added,
    Modified,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GutterMarkers {
    /// Per buffer row of the new text.
    pub lines: Vec<Option<LineChange>>,
    /// Rows *after which* lines were removed; `None` entry = removed before row 0.
    pub removed_after: Vec<Option<usize>>,
}

fn split_lines(text: &str) -> Vec<&str> {
    text.lines().collect()
}

/// Markers for `new` (the buffer) relative to `old` (the committed version).
pub fn gutter_markers(old: &str, new: &str) -> GutterMarkers {
    let old_lines = split_lines(old);
    let new_lines = split_lines(new);
    let diff = TextDiff::configure().diff_slices(&old_lines, &new_lines);
    let mut markers = GutterMarkers { lines: vec![None; new_lines.len()], removed_after: Vec::new() };
    for op in diff.ops() {
        match *op {
            DiffOp::Equal { .. } => {}
            DiffOp::Insert { new_index, new_len, .. } => {
                for row in new_index..new_index + new_len {
                    markers.lines[row] = Some(LineChange::Added);
                }
            }
            DiffOp::Delete { new_index, .. } => {
                markers.removed_after.push(new_index.checked_sub(1));
            }
            DiffOp::Replace { old_len, new_index, new_len, .. } => {
                // The first min(old, new) lines were edited; any extra new lines are additions.
                for (i, row) in (new_index..new_index + new_len).enumerate() {
                    markers.lines[row] = Some(if i < old_len { LineChange::Modified } else { LineChange::Added });
                }
                if old_len > new_len {
                    markers.removed_after.push(Some(new_index + new_len - 1));
                }
            }
        }
    }
    markers
}

// ─────────────────────────────────────────────────────────────────────────────
// Unified diff for the diff view
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineKind {
    Context,
    Added,
    Removed,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DiffRow {
    Line {
        kind: DiffLineKind,
        old_no: Option<usize>,
        new_no: Option<usize>,
        text: String,
        /// Byte ranges within `text` that changed (word-level), for emphasis.
        emphasis: Vec<std::ops::Range<usize>>,
    },
    /// Collapsed run of unchanged lines.
    Skipped(usize),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FileDiff {
    pub rows: Vec<DiffRow>,
    pub added: usize,
    pub removed: usize,
}

/// Unified diff of `old` → `new` with `context` unchanged lines around each change.
pub fn unified_diff(old: &str, new: &str, context: usize) -> FileDiff {
    let diff = TextDiff::configure().diff_lines(old, new);
    let mut out = FileDiff::default();
    let groups = diff.grouped_ops(context);

    let mut last_old_end = 0usize;
    for group in &groups {
        let first_old = group.first().map(|op| op.old_range().start).unwrap_or(0);
        if first_old > last_old_end {
            out.rows.push(DiffRow::Skipped(first_old - last_old_end));
        }
        for op in group {
            for change in diff.iter_inline_changes(op) {
                let kind = match change.tag() {
                    ChangeTag::Equal => DiffLineKind::Context,
                    ChangeTag::Insert => DiffLineKind::Added,
                    ChangeTag::Delete => DiffLineKind::Removed,
                };
                let mut text = String::new();
                let mut emphasis = Vec::new();
                for (emphasized, value) in change.iter_strings_lossy() {
                    let start = text.len();
                    text.push_str(&value);
                    if emphasized && kind != DiffLineKind::Context {
                        emphasis.push(start..text.len());
                    }
                }
                let trimmed = text.trim_end_matches(['\n', '\r']).len();
                text.truncate(trimmed);
                for r in &mut emphasis {
                    r.end = r.end.min(trimmed);
                }
                emphasis.retain(|r| r.start < r.end);
                // A line that changed entirely gets no word emphasis (the row tint says it all).
                if emphasis.len() == 1 && emphasis[0] == (0..text.len()) {
                    emphasis.clear();
                }
                match kind {
                    DiffLineKind::Added => out.added += 1,
                    DiffLineKind::Removed => out.removed += 1,
                    DiffLineKind::Context => {}
                }
                out.rows.push(DiffRow::Line {
                    kind,
                    old_no: change.old_index().map(|i| i + 1),
                    new_no: change.new_index().map(|i| i + 1),
                    text,
                    emphasis,
                });
            }
        }
        last_old_end = group.last().map(|op| op.old_range().end).unwrap_or(last_old_end);
    }
    let old_total = diff.old_slices().len();
    if !groups.is_empty() && old_total > last_old_end {
        out.rows.push(DiffRow::Skipped(old_total - last_old_end));
    }
    out
}
