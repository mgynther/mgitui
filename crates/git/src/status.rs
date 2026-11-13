//! Parsing `git status --porcelain=v2 -z`.

use crate::command::Command;
use crate::log::Log;
use crate::runner::{Git, GitError, Output};

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
    /// The file type changed, for example from a file to a symbolic link.
    TypeChanged,
    /// The file is not tracked by git.
    Untracked,
    /// The file has a merge conflict.
    Conflict,
}

/// A changed file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChange {
    /// The path relative to the repository root.
    pub path: String,
    /// The path before a rename or copy.
    pub original_path: Option<String>,
    /// How the file changed.
    pub kind: ChangeKind,
    /// Whether the file mode (for example the executable bit) changed.
    pub mode_changed: bool,
}

/// The staged and unstaged changes of the repository.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Status {
    /// Changes in the index compared with `HEAD`.
    pub staged: Vec<FileChange>,
    /// Changes in the working tree compared with the index, including untracked files and
    /// conflicts.
    pub unstaged: Vec<FileChange>,
}

impl Git {
    /// The staged and unstaged changes.
    ///
    /// # Errors
    ///
    /// Fails when git fails or its output cannot be parsed.
    pub fn status(&self, log: &Log<'_>) -> Result<Status, GitError> {
        let output: Output = self.run(&Command::Status, None, log)?;
        parse_status(&output.stdout)
    }
}

/// Parses the output of `git status --porcelain=v2 -z`.
///
/// # Errors
///
/// Returns [`GitError::Parse`] for records this application does not understand.
pub fn parse_status(output: &str) -> Result<Status, GitError> {
    let mut status: Status = Status::default();
    let mut records = output.split('\0').filter(|r: &&str| !r.is_empty());
    while let Some(record) = records.next() {
        let (tag, rest): (&str, &str) = record.split_at_checked(2).unwrap_or((record, ""));
        match tag {
            "1 " => {
                let fields: Vec<&str> = rest.splitn(8, ' ').collect();
                let [xy, _, head, index, worktree, _, _, path] = fields[..] else {
                    return Err(parse_error(record));
                };
                push_changes(&mut status, xy, (head, index, worktree), path, None, record)?;
            }
            "2 " => {
                let fields: Vec<&str> = rest.splitn(9, ' ').collect();
                let ([xy, _, head, index, worktree, _, _, _, path], Some(original)) =
                    (&fields[..], records.next())
                else {
                    return Err(parse_error(record));
                };
                push_changes(
                    &mut status,
                    xy,
                    (head, index, worktree),
                    path,
                    Some(original),
                    record,
                )?;
            }
            "u " => {
                let fields: Vec<&str> = rest.splitn(10, ' ').collect();
                let [_, _, _, _, _, _, _, _, _, path] = fields[..] else {
                    return Err(parse_error(record));
                };
                status.unstaged.push(FileChange {
                    path: path.to_string(),
                    original_path: None,
                    kind: ChangeKind::Conflict,
                    mode_changed: false,
                });
            }
            "? " => status.unstaged.push(FileChange {
                path: rest.to_string(),
                original_path: None,
                kind: ChangeKind::Untracked,
                mode_changed: false,
            }),
            "! " | "# " => {}
            _ => return Err(parse_error(record)),
        }
    }
    Ok(status)
}

/// Adds the staged and unstaged changes of an ordinary or renamed entry with status codes
/// `xy` and the modes in `HEAD`, the index and the working tree.
fn push_changes(
    status: &mut Status,
    xy: &str,
    (head, index, worktree): (&str, &str, &str),
    path: &str,
    original: Option<&str>,
    record: &str,
) -> Result<(), GitError> {
    let codes: Vec<char> = xy.chars().collect();
    let [staged, unstaged] = codes[..] else {
        return Err(parse_error(record));
    };
    if staged != '.' {
        status.staged.push(FileChange {
            path: path.to_string(),
            original_path: original.map(str::to_string),
            kind: kind(staged, record)?,
            mode_changed: mode_changed(head, index),
        });
    }
    if unstaged != '.' {
        status.unstaged.push(FileChange {
            path: path.to_string(),
            original_path: None,
            kind: kind(unstaged, record)?,
            mode_changed: mode_changed(index, worktree),
        });
    }
    Ok(())
}

/// Whether the mode changed between two versions that both exist (`000000` means missing).
fn mode_changed(before: &str, after: &str) -> bool {
    before != after && before != "000000" && after != "000000"
}

fn kind(code: char, record: &str) -> Result<ChangeKind, GitError> {
    match code {
        'M' => Ok(ChangeKind::Modified),
        'A' => Ok(ChangeKind::Added),
        'D' => Ok(ChangeKind::Deleted),
        'R' => Ok(ChangeKind::Renamed),
        'C' => Ok(ChangeKind::Copied),
        'T' => Ok(ChangeKind::TypeChanged),
        _ => Err(parse_error(record)),
    }
}

fn parse_error(record: &str) -> GitError {
    GitError::Parse(format!("unexpected status record {record:?}"))
}

#[cfg(test)]
mod tests;
