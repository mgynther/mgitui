//! Conversions from the git and validation layers' types to the logic layer's types, and back.

use logic::model::{
    AmendBase, ChangeKind, ChangedFile, Commit, DiffLine, FileDiff, Hunk, Label, LineKind, Refs,
    WorkStatus,
};

/// The logic input problem for a validation error.
#[must_use]
pub fn input_problem(error: &validation::ValidationError) -> logic::note::InputProblem {
    use logic::note::InputProblem as I;
    use validation::ValidationError as V;
    match error {
        V::BranchName(rule) => I::BranchName(branch_name_problem(*rule)),
        V::NotANumber => I::NotANumber,
        V::OutOfRange { count } => I::OutOfRange { count: *count },
        V::EmptySearch => I::EmptySearch,
        V::EmptyCommitMessage => I::EmptyCommitMessage,
    }
}

fn branch_name_problem(rule: validation::BranchNameProblem) -> logic::note::BranchNameProblem {
    use logic::note::BranchNameProblem as L;
    use validation::BranchNameProblem as V;
    match rule {
        V::Empty => L::Empty,
        V::Reserved => L::Reserved,
        V::StartsWithDash => L::StartsWithDash,
        V::BadSlash => L::BadSlash,
        V::ComponentStartsWithDot => L::ComponentStartsWithDot,
        V::ComponentEndsWithLock => L::ComponentEndsWithLock,
        V::EndsWithDot => L::EndsWithDot,
        V::DoubleDot => L::DoubleDot,
        V::AtBrace => L::AtBrace,
        V::ForbiddenCharacter(c) => L::ForbiddenCharacter(c),
    }
}

/// The git base for what the logic layer compares the amended commit with.
#[must_use]
pub fn amend_base(base: AmendBase) -> git::Base {
    match base {
        AmendBase::ParentCommit => git::Base::ParentOfHead,
        AmendBase::Nothing => git::Base::Empty,
    }
}

/// The logic files for git file changes.
#[must_use]
pub fn changed_files(files: Vec<git::FileChange>) -> Vec<ChangedFile> {
    files.into_iter().map(changed_file).collect()
}

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
        author_email: commit.author_email,
        date: commit.date,
        committer: commit.committer,
        committer_email: commit.committer_email,
        committer_date: commit.committer_date,
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
    use super::{
        amend_base, change_kind, changed_files, commit, file_diff, input_problem, refs, work_status,
    };
    use logic::model::{ChangeKind, Label, LineKind};

    #[test]
    fn validation_errors_become_input_problems() {
        use logic::note::BranchNameProblem as L;
        use logic::note::InputProblem as I;
        use validation::BranchNameProblem as V;
        use validation::ValidationError as E;
        let rules: [(V, L); 10] = [
            (V::Empty, L::Empty),
            (V::Reserved, L::Reserved),
            (V::StartsWithDash, L::StartsWithDash),
            (V::BadSlash, L::BadSlash),
            (V::ComponentStartsWithDot, L::ComponentStartsWithDot),
            (V::ComponentEndsWithLock, L::ComponentEndsWithLock),
            (V::EndsWithDot, L::EndsWithDot),
            (V::DoubleDot, L::DoubleDot),
            (V::AtBrace, L::AtBrace),
            (V::ForbiddenCharacter('~'), L::ForbiddenCharacter('~')),
        ];
        for (from, to) in rules {
            assert_eq!(input_problem(&E::BranchName(from)), I::BranchName(to));
        }
        assert_eq!(input_problem(&E::NotANumber), I::NotANumber);
        assert_eq!(
            input_problem(&E::OutOfRange { count: 3 }),
            I::OutOfRange { count: 3 }
        );
        assert_eq!(input_problem(&E::EmptySearch), I::EmptySearch);
        assert_eq!(input_problem(&E::EmptyCommitMessage), I::EmptyCommitMessage);
    }

    #[test]
    fn amend_bases() {
        assert_eq!(
            amend_base(logic::model::AmendBase::ParentCommit),
            git::Base::ParentOfHead
        );
        assert_eq!(
            amend_base(logic::model::AmendBase::Nothing),
            git::Base::Empty
        );
        assert_eq!(changed_files(vec![]), vec![]);
    }

    #[test]
    fn commits_and_refs() {
        let converted_commit: logic::model::Commit = commit(git::Commit {
            hash: "h".to_string(),
            parents: vec!["p".to_string()],
            author: "a".to_string(),
            author_email: "ae".to_string(),
            date: "d".to_string(),
            committer: "c".to_string(),
            committer_email: "ce".to_string(),
            committer_date: "cd".to_string(),
            subject: "s".to_string(),
        });
        assert_eq!(
            (
                converted_commit.hash,
                converted_commit.parents,
                converted_commit.author,
                converted_commit.author_email,
                converted_commit.date,
                converted_commit.committer,
                converted_commit.committer_email,
                converted_commit.committer_date,
                converted_commit.subject
            ),
            (
                "h".to_string(),
                vec!["p".to_string()],
                "a".to_string(),
                "ae".to_string(),
                "d".to_string(),
                "c".to_string(),
                "ce".to_string(),
                "cd".to_string(),
                "s".to_string()
            )
        );
        let git_ref = |kind: git::RefKind| git::Ref {
            target: "t".to_string(),
            kind,
            is_head: true,
        };
        let converted_refs: logic::model::Refs = refs(
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
        assert_eq!(converted_refs.head, Some("t".to_string()));
        assert_eq!(
            converted_refs.labels,
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
