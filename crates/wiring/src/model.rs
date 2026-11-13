//! Conversions from the git layer's types to the logic layer's types.

use logic::model::{ChangeKind, ChangedFile, DiffLine, FileDiff, Hunk, LineKind, WorkStatus};

/// The logic status for a git status.
#[must_use]
pub fn work_status(status: git::Status) -> WorkStatus {
    WorkStatus {
        staged: status.staged.into_iter().map(changed_file).collect(),
        unstaged: status.unstaged.into_iter().map(changed_file).collect(),
    }
}

fn changed_file(change: git::FileChange) -> ChangedFile {
    ChangedFile {
        path: change.path,
        original_path: change.original_path,
        kind: change_kind(change.kind),
        mode_changed: change.mode_changed,
    }
}

fn change_kind(kind: git::ChangeKind) -> ChangeKind {
    match kind {
        git::ChangeKind::Modified => ChangeKind::Modified,
        git::ChangeKind::Added => ChangeKind::Added,
        git::ChangeKind::Deleted => ChangeKind::Deleted,
        git::ChangeKind::Renamed => ChangeKind::Renamed,
        git::ChangeKind::Copied => ChangeKind::Copied,
        git::ChangeKind::TypeChanged => ChangeKind::TypeChanged,
        git::ChangeKind::Untracked => ChangeKind::Untracked,
        git::ChangeKind::Conflict => ChangeKind::Conflict,
    }
}

/// The logic diff for a git diff.
#[must_use]
pub fn file_diff(diff: git::FileDiff) -> FileDiff {
    FileDiff {
        header: diff.header,
        hunks: diff.hunks.into_iter().map(hunk).collect(),
        binary: diff.binary,
    }
}

fn hunk(hunk: git::Hunk) -> Hunk {
    Hunk {
        header: hunk.header,
        old_start: hunk.old_start,
        old_count: hunk.old_count,
        new_start: hunk.new_start,
        new_count: hunk.new_count,
        lines: hunk.lines.into_iter().map(line).collect(),
    }
}

fn line(line: git::DiffLine) -> DiffLine {
    DiffLine {
        kind: match line.kind {
            git::LineKind::Context => LineKind::Context,
            git::LineKind::Added => LineKind::Added,
            git::LineKind::Removed => LineKind::Removed,
            git::LineKind::NoNewline => LineKind::NoNewline,
        },
        text: line.text,
    }
}

#[cfg(test)]
mod tests {
    use super::{change_kind, file_diff, work_status};
    use logic::model::{ChangeKind, LineKind};

    #[test]
    fn change_kinds() {
        let cases: [(git::ChangeKind, ChangeKind); 8] = [
            (git::ChangeKind::Modified, ChangeKind::Modified),
            (git::ChangeKind::Added, ChangeKind::Added),
            (git::ChangeKind::Deleted, ChangeKind::Deleted),
            (git::ChangeKind::Renamed, ChangeKind::Renamed),
            (git::ChangeKind::Copied, ChangeKind::Copied),
            (git::ChangeKind::TypeChanged, ChangeKind::TypeChanged),
            (git::ChangeKind::Untracked, ChangeKind::Untracked),
            (git::ChangeKind::Conflict, ChangeKind::Conflict),
        ];
        for (from, to) in cases {
            assert_eq!(change_kind(from), to);
        }
    }

    #[test]
    fn status_keeps_fields() {
        let status: logic::model::WorkStatus = work_status(git::Status {
            staged: vec![git::FileChange {
                path: "new".to_string(),
                original_path: Some("old".to_string()),
                kind: git::ChangeKind::Renamed,
                mode_changed: true,
            }],
            unstaged: vec![],
        });
        assert_eq!(status.staged[0].path, "new");
        assert_eq!(status.staged[0].original_path, Some("old".to_string()));
        assert!(status.staged[0].mode_changed);
    }

    #[test]
    fn diff_keeps_fields() {
        let diff: logic::model::FileDiff =
            file_diff(git::parse_diff("h\n@@ -1,2 +3,4 @@\n a\n-b\n+c\n\\ x\n"));
        assert_eq!(diff.header, vec!["h"]);
        assert_eq!(
            (
                diff.hunks[0].old_start,
                diff.hunks[0].old_count,
                diff.hunks[0].new_start,
                diff.hunks[0].new_count
            ),
            (1, 2, 3, 4)
        );
        let kinds: Vec<LineKind> = diff.hunks[0].lines.iter().map(|l| l.kind).collect();
        assert_eq!(
            kinds,
            vec![
                LineKind::Context,
                LineKind::Removed,
                LineKind::Added,
                LineKind::NoNewline
            ]
        );
        assert!(!diff.binary);
    }
}
