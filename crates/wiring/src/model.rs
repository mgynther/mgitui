//! Conversions from the git layer's types to the logic layer's types.

use logic::model::{
    ChangeKind, ChangedFile, Commit, DiffLine, FileDiff, Hunk, Label, LineKind, Refs, WorkStatus,
};

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

/// The logic commit for a git commit.
#[must_use]
pub fn commit(commit: git::Commit) -> Commit {
    Commit {
        hash: commit.hash,
        parents: commit.parents,
        author: commit.author,
        date: commit.date,
        subject: commit.subject,
    }
}

/// The logic refs for git refs and the `HEAD` commit.
#[must_use]
pub fn refs(refs: Vec<git::Ref>, head: Option<String>) -> Refs {
    Refs {
        labels: refs
            .into_iter()
            .map(|r: git::Ref| {
                let label: Label = match r.kind {
                    git::RefKind::Local(name) => Label::Local {
                        name,
                        head: r.is_head,
                    },
                    git::RefKind::Remote { remote, branch } => Label::Remote { remote, branch },
                    git::RefKind::Tag(name) => Label::Tag { name },
                };
                (r.target, label)
            })
            .collect(),
        head,
    }
}

#[cfg(test)]
mod tests {
    use super::{change_kind, commit, file_diff, refs, work_status};
    use logic::model::{ChangeKind, Label, LineKind};

    #[test]
    fn commits_and_refs() {
        let converted: logic::model::Commit = commit(git::Commit {
            hash: "h".to_string(),
            parents: vec!["p".to_string()],
            author: "a".to_string(),
            date: "d".to_string(),
            subject: "s".to_string(),
        });
        assert_eq!(
            (
                converted.hash,
                converted.parents,
                converted.author,
                converted.date,
                converted.subject
            ),
            (
                "h".to_string(),
                vec!["p".to_string()],
                "a".to_string(),
                "d".to_string(),
                "s".to_string()
            )
        );
        let git_ref = |kind: git::RefKind| git::Ref {
            target: "t".to_string(),
            kind,
            is_head: true,
        };
        let converted: logic::model::Refs = refs(
            vec![
                git_ref(git::RefKind::Local("main".to_string())),
                git_ref(git::RefKind::Remote {
                    remote: "origin".to_string(),
                    branch: "main".to_string(),
                }),
                git_ref(git::RefKind::Tag("v1".to_string())),
            ],
            Some("t".to_string()),
        );
        assert_eq!(converted.head, Some("t".to_string()));
        assert_eq!(
            converted.labels,
            vec![
                (
                    "t".to_string(),
                    Label::Local {
                        name: "main".to_string(),
                        head: true
                    }
                ),
                (
                    "t".to_string(),
                    Label::Remote {
                        remote: "origin".to_string(),
                        branch: "main".to_string()
                    }
                ),
                (
                    "t".to_string(),
                    Label::Tag {
                        name: "v1".to_string()
                    }
                ),
            ]
        );
    }

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
