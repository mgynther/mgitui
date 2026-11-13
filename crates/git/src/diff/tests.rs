use super::{DiffLine, FileDiff, Hunk, LineKind, parse_diff};
use crate::runner::GitError;
use crate::test_repo::{Logged, TestRepo};

fn line(kind: LineKind, text: &str) -> DiffLine {
    DiffLine {
        kind,
        text: text.to_string(),
    }
}

#[test]
fn parses_header_hunks_and_lines() {
    let output: &str = "diff --git a/f b/f\nindex 1..2 100644\n--- a/f\n+++ b/f\n@@ -1,3 +1,3 @@ fn x\n a\n-b\n+c\n\\ No newline at end of file\n@@ -10 +10,0 @@\n-z\n";
    let diff: FileDiff = parse_diff(output);
    assert_eq!(
        diff.header,
        vec![
            "diff --git a/f b/f",
            "index 1..2 100644",
            "--- a/f",
            "+++ b/f"
        ]
    );
    assert!(!diff.binary);
    assert_eq!(
        diff.hunks,
        vec![
            Hunk {
                header: "@@ -1,3 +1,3 @@ fn x".to_string(),
                old_start: 1,
                old_count: 3,
                new_start: 1,
                new_count: 3,
                lines: vec![
                    line(LineKind::Context, "a"),
                    line(LineKind::Removed, "b"),
                    line(LineKind::Added, "c"),
                    line(LineKind::NoNewline, " No newline at end of file"),
                ],
            },
            Hunk {
                header: "@@ -10 +10,0 @@".to_string(),
                old_start: 10,
                old_count: 1,
                new_start: 10,
                new_count: 0,
                lines: vec![line(LineKind::Removed, "z")],
            },
        ]
    );
}

#[test]
fn empty_context_line_and_bad_ranges() {
    let diff: FileDiff = parse_diff("@@ -x,y @@\n\n");
    assert_eq!(diff.hunks[0].old_start, 0);
    assert_eq!(diff.hunks[0].old_count, 0);
    assert_eq!(diff.hunks[0].new_start, 0);
    assert_eq!(diff.hunks[0].lines, vec![line(LineKind::Context, "")]);
}

#[test]
fn binary_files() {
    let diff: FileDiff = parse_diff("diff --git a/p b/p\nBinary files a/p and b/p differ\n");
    assert!(diff.binary);
    assert_eq!(diff.hunks, vec![]);
}

#[test]
fn combined_diffs_keep_prefixes() {
    let diff: FileDiff = parse_diff("diff --cc f\n@@@ -1,1 -1,1 +1,5 @@@\n++<<<<<<< HEAD\n");
    assert_eq!(diff.hunks[0].old_start, 0);
    assert_eq!(
        diff.hunks[0].lines,
        vec![line(LineKind::Context, "++<<<<<<< HEAD")]
    );
}

#[test]
fn diffs_of_real_repository() {
    let repo: TestRepo = TestRepo::new();
    repo.write("f.txt", "one\ntwo\n");
    repo.git_ok(&["add", "."]);
    repo.git_ok(&["commit", "-q", "-m", "init"]);
    repo.write("f.txt", "one\n2\n");
    repo.write("new.txt", "hello\n");
    let logged: Logged = Logged::new();
    let unstaged: FileDiff = repo
        .git()
        .diff(&["f.txt".to_string()], false, &logged.log())
        .unwrap();
    assert_eq!(unstaged.hunks[0].lines[1], line(LineKind::Removed, "two"));
    repo.git_ok(&["add", "f.txt"]);
    let staged: FileDiff = repo
        .git()
        .diff(&["f.txt".to_string()], true, &logged.log())
        .unwrap();
    assert_eq!(staged.hunks[0].lines[2], line(LineKind::Added, "2"));
    let untracked: FileDiff = repo.git().diff_untracked("new.txt", &logged.log()).unwrap();
    assert_eq!(
        untracked.hunks[0].lines,
        vec![line(LineKind::Added, "hello")]
    );
    assert!(untracked.header.contains(&"+++ b/new.txt".to_string()));
}

#[test]
fn diff_failures() {
    let repo: TestRepo = TestRepo::new();
    let logged: Logged = Logged::new();
    let git: crate::runner::Git = repo.git().with_program("/nonexistent/git");
    assert!(matches!(
        git.diff(&["x".to_string()], false, &logged.log()),
        Err(GitError::Spawn(_))
    ));
    assert!(matches!(
        git.diff_untracked("x", &logged.log()),
        Err(GitError::Spawn(_))
    ));
}

#[test]
fn carriage_returns_are_content() {
    let diff: FileDiff = parse_diff("@@ -1 +1 @@\n-a\r\n+b\r\n");
    assert_eq!(
        diff.hunks[0].lines,
        vec![line(LineKind::Removed, "a\r"), line(LineKind::Added, "b\r")]
    );
}
