//! Parsing `git diff` output for one file.

use crate::command::Command;
use crate::log::Log;
use crate::runner::{Git, GitError, Output};

/// The kind of a line in a hunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    /// An unchanged line, prefix ` `.
    Context,
    /// An added line, prefix `+`.
    Added,
    /// A removed line, prefix `-`.
    Removed,
    /// `\ No newline at end of file`, about the line before it.
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

/// A hunk: a block of changes with its position.
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
    /// The lines of the hunk.
    pub lines: Vec<DiffLine>,
}

/// The diff of one file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileDiff {
    /// The lines before the first hunk: `diff --git`, `index`, `---` and `+++`.
    pub header: Vec<String>,
    /// The hunks.
    pub hunks: Vec<Hunk>,
    /// Whether git reported the file as binary.
    pub binary: bool,
}

impl Git {
    /// The diff of a tracked file: staged or unstaged changes. `paths` is the path, or the
    /// original and new path of a rename.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    pub fn diff(
        &self,
        paths: &[String],
        staged: bool,
        log: &Log<'_>,
    ) -> Result<FileDiff, GitError> {
        let command: Command = Command::Diff {
            paths: paths.to_vec(),
            staged,
        };
        let output: Output = self.run(&command, None, log)?;
        Ok(parse_diff(&output.stdout))
    }

    /// The diff of an untracked file, showing all of its content as added.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    pub fn diff_untracked(&self, path: &str, log: &Log<'_>) -> Result<FileDiff, GitError> {
        let command: Command = Command::DiffUntracked {
            path: path.to_string(),
        };
        let output: Output = self.run(&command, None, log)?;
        Ok(parse_diff(&output.stdout))
    }
}

/// Parses the diff of one file. Combined diffs of conflicts have no usable line positions;
/// their lines are kept as context lines with their prefixes.
#[must_use]
pub fn parse_diff(output: &str) -> FileDiff {
    let mut diff: FileDiff = FileDiff::default();
    let mut combined: bool = false;
    // Split on `\n` only: `lines()` would also remove a `\r` that belongs to the content.
    for line in output.split_terminator('\n') {
        if line.starts_with("@@@") {
            combined = true;
            diff.hunks.push(hunk(line, (0, 0, 0, 0)));
        } else if line.starts_with("@@") {
            diff.hunks.push(hunk(line, hunk_range(line)));
        } else if let Some(current) = diff.hunks.last_mut() {
            current.lines.push(if combined {
                DiffLine {
                    kind: LineKind::Context,
                    text: line.to_string(),
                }
            } else {
                diff_line(line)
            });
        } else {
            if line.starts_with("Binary files ") {
                diff.binary = true;
            }
            diff.header.push(line.to_string());
        }
    }
    diff
}

fn hunk(
    header: &str,
    (old_start, old_count, new_start, new_count): (usize, usize, usize, usize),
) -> Hunk {
    Hunk {
        header: header.to_string(),
        old_start,
        old_count,
        new_start,
        new_count,
        lines: Vec::new(),
    }
}

fn diff_line(line: &str) -> DiffLine {
    let mut chars = line.chars();
    let kind: LineKind = match chars.next() {
        Some('+') => LineKind::Added,
        Some('-') => LineKind::Removed,
        Some('\\') => LineKind::NoNewline,
        _ => LineKind::Context,
    };
    DiffLine {
        kind,
        text: chars.as_str().to_string(),
    }
}

/// Parses `@@ -a,b +c,d @@`, where a missing count means 1.
fn hunk_range(header: &str) -> (usize, usize, usize, usize) {
    let mut parts = header.split(' ').skip(1);
    let old: (usize, usize) = range(parts.next().unwrap_or_default().trim_start_matches('-'));
    let new: (usize, usize) = range(parts.next().unwrap_or_default().trim_start_matches('+'));
    (old.0, old.1, new.0, new.1)
}

fn range(text: &str) -> (usize, usize) {
    let (start, count): (&str, Option<&str>) = match text.split_once(',') {
        Some((start, count)) => (start, Some(count)),
        None => (text, None),
    };
    (
        start.parse().unwrap_or(0),
        count.map_or(1, |c: &str| c.parse().unwrap_or(0)),
    )
}

#[cfg(test)]
mod tests;
