//! Rendered change-tracking PDFs between two versions of a LaTeX project.
//!
//! Both versions are materialized into a throw-away directory, `latexdiff
//! --flatten` merges them into one marked-up document, and latexmk compiles it
//! there. Only the resulting PDF is copied back into the project.

use crate::compiler::get_latex_path_env;
use crate::project::{detect_engine, Engine};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LatexDiffRequest {
    pub repo_root: PathBuf,
    /// Repo-relative directory that is copied for each version ("" = whole repo).
    pub scope: String,
    /// Repo-relative path of the main `.tex` document.
    pub main_rel: String,
    pub old_rev: String,
    /// `None` compares against the working copy.
    pub new_rev: Option<String>,
    pub output_pdf: PathBuf,
}

/// File name for the generated PDF, e.g. `paper-diff-1a2b3c4-working.pdf`.
pub fn output_name(main_rel: &str, old_rev: &str, new_rev: Option<&str>) -> String {
    let stem = Path::new(main_rel).file_stem().and_then(|s| s.to_str()).unwrap_or("document");
    let short = |r: &str| r.chars().take(7).collect::<String>();
    let new = new_rev.map(short).unwrap_or_else(|| "working".to_string());
    format!("{stem}-diff-{}-{new}.pdf", short(old_rev))
}

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Result<Self, String> {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("vortex-latexdiff-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&dir).map_err(|e| format!("Could not create temp folder: {e}"))?;
        Ok(Self(dir))
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn short(rev: &str) -> String {
    rev.chars().take(7).collect()
}

fn pathspec(scope: &str) -> &str {
    if scope.is_empty() {
        "."
    } else {
        scope
    }
}

/// Extracts the `scope` tree of `rev` into `dest` via `git archive | tar -x`.
fn export_revision(root: &Path, rev: &str, scope: &str, dest: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    let mut archive = Command::new("git")
        .args(["archive", "--format=tar", rev, "--", pathspec(scope)])
        .current_dir(root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Could not run git: {e}"))?;
    let tar = Command::new("tar")
        .arg("-x")
        .arg("-C")
        .arg(dest)
        .stdin(archive.stdout.take().ok_or("git archive produced no output")?)
        .output()
        .map_err(|e| format!("Could not run tar: {e}"))?;
    let git_out = archive.wait_with_output().map_err(|e| e.to_string())?;
    if !git_out.status.success() {
        let err = String::from_utf8_lossy(&git_out.stderr);
        return Err(format!("Could not read commit {}: {}", short(rev), err.trim()));
    }
    if !tar.status.success() {
        return Err(format!("Could not unpack commit {}: {}", short(rev), String::from_utf8_lossy(&tar.stderr).trim()));
    }
    Ok(())
}

/// Copies the working tree's tracked and untracked-but-not-ignored files under `scope`.
fn export_working_copy(root: &Path, scope: &str, dest: &Path) -> Result<(), String> {
    let out = Command::new("git")
        .args(["ls-files", "-z", "--cached", "--others", "--exclude-standard", "--", pathspec(scope)])
        .current_dir(root)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()
        .map_err(|e| format!("Could not run git: {e}"))?;
    if !out.status.success() {
        return Err(format!("git ls-files failed: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    for rel in String::from_utf8_lossy(&out.stdout).split('\0').filter(|s| !s.is_empty()) {
        let src = root.join(rel);
        // Tracked files deleted in the working tree are still listed.
        if !src.is_file() {
            continue;
        }
        let target = dest.join(rel);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::copy(&src, &target).map_err(|e| format!("Could not copy {rel}: {e}"))?;
    }
    Ok(())
}

/// First few LaTeX errors from a log file, for the failure message.
pub fn log_errors(log: &str, max: usize) -> Vec<String> {
    let lines: Vec<&str> = log.lines().collect();
    let mut errors = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let is_error = line.starts_with("! ") || line.contains(".tex:") && line.contains(": ") && !line.contains("Warning");
        if is_error {
            let mut msg = line.trim().to_string();
            // "l.42 \foo" follows "! Undefined control sequence." a line or two later.
            if let Some(ctx) = lines.iter().skip(i + 1).take(3).find(|l| l.starts_with("l.")) {
                msg.push_str(" — ");
                msg.push_str(ctx.trim());
            }
            if !errors.contains(&msg) {
                errors.push(msg);
            }
            if errors.len() >= max {
                break;
            }
        }
    }
    errors
}

/// Runs latexdiff with `extra` options, compiles, and returns the PDF path inside `work_dir`.
fn diff_and_compile(old_main: &Path, new_main: &Path, engine: Engine, extra: &[&str], path_env: &str) -> Result<PathBuf, String> {
    let work_dir = new_main.parent().ok_or("Main file has no parent folder")?;
    let stem = new_main.file_stem().and_then(|s| s.to_str()).unwrap_or("document");
    let diff_name = format!("{stem}-vortex-latexdiff");
    let diff_tex = work_dir.join(format!("{diff_name}.tex"));

    let out = Command::new("latexdiff")
        .env("PATH", path_env)
        .current_dir(work_dir)
        .args(["--flatten", "--encoding=utf8"])
        .args(extra)
        .arg(old_main)
        .arg(new_main)
        .output()
        .map_err(|e| format!("Could not run latexdiff (is it installed?): {e}"))?;
    if !out.status.success() || out.stdout.is_empty() {
        let err = String::from_utf8_lossy(&out.stderr);
        let last = err.lines().filter(|l| !l.trim().is_empty()).last().unwrap_or("unknown error");
        return Err(format!("latexdiff failed: {last}"));
    }
    std::fs::write(&diff_tex, &out.stdout).map_err(|e| e.to_string())?;

    let _ = Command::new("latexmk")
        .env("PATH", path_env)
        .current_dir(work_dir)
        .args([engine.latexmk_flag(), "-interaction=nonstopmode", "-f", "-file-line-error", "-outdir=."])
        .arg(format!("{diff_name}.tex"))
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("Could not run latexmk: {e}"))?;

    let pdf = work_dir.join(format!("{diff_name}.pdf"));
    if pdf.is_file() {
        return Ok(pdf);
    }
    let log = std::fs::read(work_dir.join(format!("{diff_name}.log")))
        .map(|b| String::from_utf8_lossy(&b).to_string())
        .unwrap_or_default();
    let errors = log_errors(&log, 3);
    Err(if errors.is_empty() {
        "Compiling the diff produced no PDF".to_string()
    } else {
        format!("Compiling the diff failed:\n{}", errors.join("\n"))
    })
}

/// Builds the change-tracking PDF and copies it to `req.output_pdf`.
pub fn generate(req: &LatexDiffRequest) -> Result<PathBuf, String> {
    let tmp = TempDir::new()?;
    let old_dir = tmp.0.join("old");
    let new_dir = tmp.0.join("new");

    export_revision(&req.repo_root, &req.old_rev, &req.scope, &old_dir)?;
    match &req.new_rev {
        Some(rev) => export_revision(&req.repo_root, rev, &req.scope, &new_dir)?,
        None => export_working_copy(&req.repo_root, &req.scope, &new_dir)?,
    }

    let old_main = old_dir.join(&req.main_rel);
    let new_main = new_dir.join(&req.main_rel);
    let label = |rev: &Option<String>| rev.as_deref().map(short).unwrap_or_else(|| "the working copy".to_string());
    if !old_main.is_file() {
        return Err(format!("{} does not exist in commit {}", req.main_rel, short(&req.old_rev)));
    }
    if !new_main.is_file() {
        return Err(format!("{} does not exist in {}", req.main_rel, label(&req.new_rev)));
    }

    let source = std::fs::read_to_string(&new_main).unwrap_or_default();
    let engine = detect_engine(&source);
    let path_env = get_latex_path_env();

    // latexdiff's markup occasionally breaks compilation (math, figures, citations,
    // bibliography entries flattened in from a committed .bbl); the fallback marks
    // those up coarsely and shows the new bibliography unmarked.
    let pdf = diff_and_compile(&old_main, &new_main, engine, &[], &path_env).or_else(|first_err| {
        diff_and_compile(
            &old_main,
            &new_main,
            engine,
            &[
                "--math-markup=whole",
                "--graphics-markup=none",
                "--disable-citation-markup",
                r"--config=PICTUREENV=(?:picture|DIFnomarkup|thebibliography)[\w\d*@]*",
            ],
            &path_env,
        )
        .map_err(|_| first_err)
    })?;

    if let Some(parent) = req.output_pdf.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::copy(&pdf, &req.output_pdf).map_err(|e| format!("Could not save the PDF: {e}"))?;
    Ok(req.output_pdf.clone())
}
