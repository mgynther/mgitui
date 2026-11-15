//! The data logic works with: changed files and diffs.

/// How a file changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    /// Content changed.
    Modified,
    /// The file is new.
    Added,
    /// The file was deleted.
    Deleted,
    /// The file was renamed.
    Renamed,
    /// The file was copied.
    Copied,
    /// The file type changed.
    TypeChanged,
    /// The file is not tracked by git.
    Untracked,
    /// The file has a merge conflict.
    Conflict,
}

/// A changed file in the staged or unstaged list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangedFile {
    /// The path relative to the repository root.
    pub path: String,
    /// The path before a rename or copy.
    pub original_path: Option<String>,
    /// How the file changed.
    pub kind: ChangeKind,
    /// Whether the file mode changed.
    pub mode_changed: bool,
}

impl ChangedFile {
    /// The paths git needs for this file in the `side` list. A rename in the staged list needs
    /// the original path too: the diff then shows the rename, and unstaging restores the
    /// original path in the index.
    #[must_use]
    pub fn paths(&self, side: Side) -> Vec<String> {
        match (&self.original_path, side) {
            (Some(original), Side::Staged) => vec![original.clone(), self.path.clone()],
            (_, Side::Staged | Side::Unstaged) => vec![self.path.clone()],
        }
    }

    /// Whether single lines of this file can be staged: only content changes of files git
    /// already tracks.
    #[must_use]
    pub fn supports_line_staging(&self) -> bool {
        self.kind == ChangeKind::Modified && !self.mode_changed
    }
}

/// Staged and unstaged changes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkStatus {
    /// Changes in the index.
    pub staged: Vec<ChangedFile>,
    /// Changes in the working tree, including untracked files and conflicts.
    pub unstaged: Vec<ChangedFile>,
}

impl WorkStatus {
    /// Whether there are any staged or unstaged changes.
    #[must_use]
    pub fn has_changes(&self) -> bool {
        !self.staged.is_empty() || !self.unstaged.is_empty()
    }

    /// Whether there are changes to tracked files: anything staged, or unstaged changes other
    /// than untracked files. Merge conflicts count as changes.
    #[must_use]
    pub fn has_tracked_changes(&self) -> bool {
        !self.staged.is_empty()
            || self
                .unstaged
                .iter()
                .any(|file: &ChangedFile| file.kind != ChangeKind::Untracked)
    }
}

/// What the staged list is compared with in amend mode, so that it shows what the amended
/// commit will contain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmendBase {
    /// The commit before the last one.
    ParentCommit,
    /// Nothing: the last commit is the first one, so all of its files are new.
    Nothing,
}

/// Which list of changes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Side {
    /// Working tree changes.
    #[default]
    Unstaged,
    /// Index changes.
    Staged,
}

/// The kind of a line in a hunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    /// An unchanged line.
    Context,
    /// An added line.
    Added,
    /// A removed line.
    Removed,
    /// `\ No newline at end of file`.
    NoNewline,
}

/// A line in a hunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    /// The kind of line.
    pub kind: LineKind,
    /// The text after the prefix character.
    pub text: String,
}

/// A block of changes with its position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunk {
    /// The `@@ ... @@` line.
    pub header: String,
    /// First line in the old version.
    pub old_start: usize,
    /// Number of lines in the old version.
    pub old_count: usize,
    /// First line in the new version.
    pub new_start: usize,
    /// Number of lines in the new version.
    pub new_count: usize,
    /// The lines.
    pub lines: Vec<DiffLine>,
}

/// The diff of one file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileDiff {
    /// The lines before the first hunk.
    pub header: Vec<String>,
    /// The hunks.
    pub hunks: Vec<Hunk>,
    /// Whether the file is binary.
    pub binary: bool,
}

/// What a row of a shown diff is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind {
    /// A hunk header.
    HunkHeader,
    /// A line of a hunk.
    Line(LineKind),
    /// A note instead of content, such as for binary files.
    Note,
}

/// A row of a shown diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffRow {
    /// What the row is.
    pub kind: RowKind,
    /// The text, without a prefix character.
    pub text: String,
    /// The hunk the row belongs to.
    pub hunk: usize,
    /// The line within the hunk, `None` for hunk headers and notes.
    pub line: Option<usize>,
}

/// The rows shown for a diff: hunk headers and lines, or a note when there is no text content.
#[must_use]
pub fn diff_rows(diff: &FileDiff) -> Vec<DiffRow> {
    if diff.binary {
        return vec![note("Binary file")];
    }
    if diff.hunks.is_empty() {
        return vec![note("No content changes")];
    }
    let mut rows: Vec<DiffRow> = Vec::new();
    for (hunk_index, hunk) in diff.hunks.iter().enumerate() {
        rows.push(DiffRow {
            kind: RowKind::HunkHeader,
            text: hunk.header.clone(),
            hunk: hunk_index,
            line: None,
        });
        for (line_index, line) in hunk.lines.iter().enumerate() {
            rows.push(DiffRow {
                kind: RowKind::Line(line.kind),
                text: line.text.clone(),
                hunk: hunk_index,
                line: Some(line_index),
            });
        }
    }
    rows
}

fn note(text: &str) -> DiffRow {
    DiffRow {
        kind: RowKind::Note,
        text: text.to_string(),
        hunk: 0,
        line: None,
    }
}

/// A commit in the history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit {
    /// The full hash.
    pub hash: String,
    /// The hashes of the parents, first parent first.
    pub parents: Vec<String>,
    /// The author name.
    pub author: String,
    /// The author date, formatted for display.
    pub date: String,
    /// The first line of the message.
    pub subject: String,
}

/// A branch or tag label of a commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Label {
    /// A local branch.
    Local {
        /// The branch name.
        name: String,
        /// Whether the branch is checked out.
        head: bool,
    },
    /// A remote-tracking branch.
    Remote {
        /// The remote name.
        remote: String,
        /// The branch name on the remote.
        branch: String,
    },
    /// A tag.
    Tag {
        /// The tag name.
        name: String,
    },
}

/// All labels and the commit `HEAD` points to.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Refs {
    /// Labels with the hash of the commit they point to.
    pub labels: Vec<(String, Label)>,
    /// The commit `HEAD` points to, if there are commits.
    pub head: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{
        ChangeKind, ChangedFile, DiffLine, DiffRow, FileDiff, Hunk, LineKind, RowKind, Side,
        WorkStatus, diff_rows,
    };

    fn file(kind: ChangeKind, mode_changed: bool) -> ChangedFile {
        ChangedFile {
            path: "p".to_string(),
            original_path: None,
            kind,
            mode_changed,
        }
    }

    #[test]
    fn line_staging_only_for_plain_modifications() {
        assert!(file(ChangeKind::Modified, false).supports_line_staging());
        assert!(!file(ChangeKind::Modified, true).supports_line_staging());
        assert!(!file(ChangeKind::Untracked, false).supports_line_staging());
    }

    #[test]
    fn a_staged_rename_needs_both_paths() {
        let renamed: ChangedFile = ChangedFile {
            original_path: Some("old".to_string()),
            ..file(ChangeKind::Renamed, false)
        };
        assert_eq!(renamed.paths(Side::Staged), vec!["old", "p"]);
        assert_eq!(renamed.paths(Side::Unstaged), vec!["p"]);
        let modified: ChangedFile = file(ChangeKind::Modified, false);
        assert_eq!(modified.paths(Side::Staged), vec!["p"]);
    }

    #[test]
    fn has_changes() {
        assert!(!WorkStatus::default().has_changes());
        let staged: WorkStatus = WorkStatus {
            staged: vec![file(ChangeKind::Added, false)],
            unstaged: vec![],
        };
        assert!(staged.has_changes());
        let unstaged: WorkStatus = WorkStatus {
            staged: vec![],
            unstaged: vec![file(ChangeKind::Added, false)],
        };
        assert!(unstaged.has_changes());
    }

    #[test]
    fn has_tracked_changes_ignores_untracked_files() {
        assert!(!WorkStatus::default().has_tracked_changes());
        let untracked: WorkStatus = WorkStatus {
            staged: vec![],
            unstaged: vec![file(ChangeKind::Untracked, false)],
        };
        assert!(untracked.has_changes());
        assert!(!untracked.has_tracked_changes());
        let staged: WorkStatus = WorkStatus {
            staged: vec![file(ChangeKind::Added, false)],
            unstaged: vec![],
        };
        assert!(staged.has_tracked_changes());
        let unstaged: WorkStatus = WorkStatus {
            staged: vec![],
            unstaged: vec![
                file(ChangeKind::Untracked, false),
                file(ChangeKind::Modified, false),
            ],
        };
        assert!(unstaged.has_tracked_changes());
    }

    #[test]
    fn rows_of_hunks() {
        let diff: FileDiff = FileDiff {
            header: vec![],
            hunks: vec![Hunk {
                header: "@@ -1 +1 @@".to_string(),
                old_start: 1,
                old_count: 1,
                new_start: 1,
                new_count: 1,
                lines: vec![DiffLine {
                    kind: LineKind::Added,
                    text: "x".to_string(),
                }],
            }],
            binary: false,
        };
        assert_eq!(
            diff_rows(&diff),
            vec![
                DiffRow {
                    kind: RowKind::HunkHeader,
                    text: "@@ -1 +1 @@".to_string(),
                    hunk: 0,
                    line: None
                },
                DiffRow {
                    kind: RowKind::Line(LineKind::Added),
                    text: "x".to_string(),
                    hunk: 0,
                    line: Some(0)
                },
            ]
        );
    }

    #[test]
    fn notes_instead_of_content() {
        let binary: FileDiff = FileDiff {
            binary: true,
            ..FileDiff::default()
        };
        assert_eq!(diff_rows(&binary)[0].text, "Binary file");
        assert_eq!(
            diff_rows(&FileDiff::default())[0].text,
            "No content changes"
        );
    }
}
