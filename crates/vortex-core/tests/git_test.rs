use std::process::Command;
use vortex_core::git::*;

#[test]
fn gutter_marks_added_modified_and_removed_lines() {
    let old = "a\nb\nc\nd\ne\n";
    let new = "a\nB\nc\nnew1\nnew2\nd\n"; // b modified, 2 lines added after c, e removed
    let m = gutter_markers(old, new);
    assert_eq!(
        m.lines,
        vec![None, Some(LineChange::Modified), None, Some(LineChange::Added), Some(LineChange::Added), None]
    );
    assert_eq!(m.removed_after, vec![Some(5)]);

    let m = gutter_markers("x\ny\n", "y\n");
    assert_eq!(m.lines, vec![None]);
    assert_eq!(m.removed_after, vec![None]);
}

#[test]
fn replace_with_fewer_lines_marks_removal() {
    let m = gutter_markers("a\nb\nc\nz\n", "A\nz\n");
    assert_eq!(m.lines, vec![Some(LineChange::Modified), None]);
    assert_eq!(m.removed_after, vec![Some(0)]);
}

#[test]
fn unified_diff_collapses_context_and_highlights_words() {
    let old: String = (1..=20).map(|i| format!("line {i}\n")).collect();
    let new = old.replace("line 10\n", "line ten changed\n");
    let d = unified_diff(&old, &new, 2);
    assert_eq!((d.added, d.removed), (1, 1));
    assert!(matches!(d.rows.first(), Some(DiffRow::Skipped { lines: 7 })));
    assert!(matches!(d.rows.last(), Some(DiffRow::Skipped { lines: 8 })));
    let added = d.rows.iter().find_map(|r| match r {
        DiffRow::Line { kind: DiffLineKind::Added, text, emphasis, new_no, .. } => Some((text.clone(), emphasis.clone(), *new_no)),
        _ => None,
    });
    let (text, emphasis, new_no) = added.unwrap();
    assert_eq!(text, "line ten changed");
    assert_eq!(new_no, Some(10));
    assert!(!emphasis.is_empty());
    assert!(emphasis.iter().all(|r| &text[r.clone()] != "line"));
}

#[test]
fn identical_texts_have_no_rows() {
    let d = unified_diff("a\nb\n", "a\nb\n", 3);
    assert!(d.rows.is_empty());
}

#[test]
fn parses_porcelain_status() {
    let raw = b" M main.tex\0?? notes.txt\0R  new.tex\0old.tex\0D  gone.bib\0A  added.tex\0";
    let st = parse_status(raw);
    let kinds: Vec<_> = st.iter().map(|s| (s.path.as_str(), s.kind)).collect();
    assert_eq!(
        kinds,
        vec![
            ("added.tex", ChangeKind::Added),
            ("gone.bib", ChangeKind::Deleted),
            ("main.tex", ChangeKind::Modified),
            ("new.tex", ChangeKind::Renamed),
            ("notes.txt", ChangeKind::Untracked),
        ]
    );
}

fn run(dir: &std::path::Path, args: &[&str]) {
    let ok = Command::new("git").args(args).current_dir(dir).output().unwrap().status.success();
    assert!(ok, "git {:?} failed", args);
}

#[test]
fn reads_status_history_and_old_versions_from_a_real_repo() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    run(dir, &["init", "-q", "-b", "main"]);
    run(dir, &["config", "user.email", "t@example.com"]);
    run(dir, &["config", "user.name", "Tester"]);
    std::fs::write(dir.join("main.tex"), "v1\n").unwrap();
    run(dir, &["add", "."]);
    run(dir, &["commit", "-qm", "first"]);
    std::fs::write(dir.join("main.tex"), "v2\n").unwrap();
    run(dir, &["commit", "-qam", "second"]);
    std::fs::write(dir.join("main.tex"), "v3\n").unwrap();
    std::fs::write(dir.join("new.tex"), "x\n").unwrap();

    let root = repo_root(&dir.join("main.tex")).unwrap();
    assert_eq!(current_branch(&root).as_deref(), Some("main"));
    let rel = relative_path(&root, &dir.join("main.tex")).unwrap();
    assert_eq!(rel, "main.tex");

    let st = status(&root);
    assert_eq!(st.len(), 2);
    assert_eq!(st[0], FileStatus { path: "main.tex".into(), kind: ChangeKind::Modified });
    assert_eq!(st[1].kind, ChangeKind::Untracked);

    let log = file_log(&root, &rel, 10);
    assert_eq!(log.iter().map(|c| c.subject.as_str()).collect::<Vec<_>>(), vec!["second", "first"]);
    assert_eq!(log[0].author, "Tester");

    assert_eq!(file_at_revision(&root, "HEAD", &rel).as_deref(), Some(&b"v2\n"[..]));
    assert_eq!(file_at_revision(&root, &log[1].hash, &rel).as_deref(), Some(&b"v1\n"[..]));
    assert!(file_at_revision(&root, "HEAD", "new.tex").is_none());
}

#[test]
fn binary_content_is_not_text() {
    assert_eq!(as_text(b"hello\n").as_deref(), Some("hello\n"));
    assert!(as_text(b"\x89PNG\0\0").is_none());
}
