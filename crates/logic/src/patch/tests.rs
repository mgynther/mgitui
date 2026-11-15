use std::collections::BTreeSet;

use super::{LineRef, build_patch};
use crate::model::{DiffLine, FileDiff, Hunk, LineKind};

fn line(spec: &str) -> DiffLine {
    let (prefix, text): (&str, &str) = spec.split_at(1);
    DiffLine {
        kind: match prefix {
            "+" => LineKind::Added,
            "-" => LineKind::Removed,
            "\\" => LineKind::NoNewline,
            _ => LineKind::Context,
        },
        text: text.to_string(),
    }
}

/// A hunk whose old and new ranges are given as (start, count).
fn hunk(old: (usize, usize), new: (usize, usize), lines: &[&str]) -> Hunk {
    Hunk {
        header: String::new(),
        old_start: old.0,
        old_count: old.1,
        new_start: new.0,
        new_count: new.1,
        lines: lines.iter().map(|l: &&str| line(l)).collect(),
    }
}

fn diff(hunks: Vec<Hunk>) -> FileDiff {
    FileDiff {
        header: vec!["--- a/f".to_string(), "+++ b/f".to_string()],
        hunks,
        binary: false,
    }
}

fn select(refs: &[LineRef]) -> BTreeSet<LineRef> {
    refs.iter().copied().collect()
}

/// A change of b to B and d to D, with context around.
fn two_changes() -> FileDiff {
    diff(vec![hunk(
        (1, 5),
        (1, 5),
        &[" a", "-b", "+B", " c", "-d", "+D", " e"],
    )])
}

#[test]
fn stage_one_changed_line_pair() {
    let patch: Option<String> = build_patch(&two_changes(), &select(&[(0, 1), (0, 2)]), false);
    assert_eq!(
        patch.as_deref(),
        Some("--- a/f\n+++ b/f\n@@ -1,5 +1,5 @@\n a\n-b\n+B\n c\n d\n e\n")
    );
}

#[test]
fn stage_only_an_addition_or_only_a_removal() {
    let added: Option<String> = build_patch(&two_changes(), &select(&[(0, 5)]), false);
    assert_eq!(
        added.as_deref(),
        Some("--- a/f\n+++ b/f\n@@ -1,5 +1,6 @@\n a\n b\n c\n d\n+D\n e\n")
    );
    let removed: Option<String> = build_patch(&two_changes(), &select(&[(0, 4)]), false);
    assert_eq!(
        removed.as_deref(),
        Some("--- a/f\n+++ b/f\n@@ -1,5 +1,4 @@\n a\n b\n c\n-d\n e\n")
    );
}

#[test]
fn unstage_keeps_unselected_additions_as_context() {
    let patch: Option<String> = build_patch(&two_changes(), &select(&[(0, 1), (0, 2)]), true);
    assert_eq!(
        patch.as_deref(),
        Some("--- a/f\n+++ b/f\n@@ -1,5 +1,5 @@\n a\n-b\n+B\n c\n D\n e\n")
    );
}

#[test]
fn context_only_selection_is_nothing() {
    assert_eq!(
        build_patch(&two_changes(), &select(&[(0, 0), (0, 3)]), false),
        None
    );
    assert_eq!(build_patch(&two_changes(), &select(&[]), false), None);
}

#[test]
fn later_hunks_are_shifted_by_earlier_included_hunks() {
    let diff: FileDiff = diff(vec![
        hunk((1, 1), (1, 3), &[" a", "+x", "+y"]),
        hunk((10, 2), (12, 1), &[" j", "-k"]),
        hunk((20, 1), (21, 2), &[" t", "+u"]),
    ]);
    let forward: String = build_patch(&diff, &select(&[(0, 1), (0, 2), (2, 1)]), false).unwrap();
    assert_eq!(
        forward,
        "--- a/f\n+++ b/f\n@@ -1,1 +1,3 @@\n a\n+x\n+y\n@@ -20,1 +22,2 @@\n t\n+u\n"
    );
    let reverse: String = build_patch(&diff, &select(&[(1, 1), (2, 1)]), true).unwrap();
    assert_eq!(
        reverse,
        "--- a/f\n+++ b/f\n@@ -12,2 +12,1 @@\n j\n-k\n@@ -22,1 +21,2 @@\n t\n+u\n"
    );
}

#[test]
fn zero_counts_use_the_line_before() {
    let diff: FileDiff = diff(vec![hunk((1, 1), (0, 0), &["-only"])]);
    let patch: String = build_patch(&diff, &select(&[(0, 0)]), false).unwrap();
    assert_eq!(patch, "--- a/f\n+++ b/f\n@@ -1,1 +0,0 @@\n-only\n");
}

#[test]
fn no_newline_markers_follow_kept_lines() {
    let diff: FileDiff = diff(vec![hunk(
        (1, 1),
        (1, 1),
        &[
            "-old",
            "\\ No newline at end of file",
            "+new",
            "\\ No newline at end of file",
        ],
    )]);
    let both: String = build_patch(&diff, &select(&[(0, 0), (0, 2)]), false).unwrap();
    assert_eq!(
        both,
        "--- a/f\n+++ b/f\n@@ -1,1 +1,1 @@\n-old\n\\ No newline at end of file\n+new\n\\ No newline at end of file\n"
    );
    let removal_only: String = build_patch(&diff, &select(&[(0, 0)]), false).unwrap();
    assert_eq!(
        removal_only,
        "--- a/f\n+++ b/f\n@@ -1,1 +0,0 @@\n-old\n\\ No newline at end of file\n"
    );
}
