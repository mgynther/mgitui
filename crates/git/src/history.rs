//! Reading the commit history and refs.

use std::ffi::{OsStr, OsString};

use crate::command::Command;
use crate::diff::{FileDiff, parse_diff};
use crate::log::Log;
use crate::runner::{Git, GitError, Output};
use crate::status::{ChangeKind, FileChange};

/// A commit in the history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit {
    /// The full hash.
    pub hash: String,
    /// The hashes of the parents, first parent first.
    pub parents: Vec<String>,
    /// The author name.
    pub author: String,
    /// The author date as `YYYY-MM-DD HH:MM` in local time.
    pub date: String,
    /// The committer name.
    pub committer: String,
    /// The committer date as `YYYY-MM-DD HH:MM` in local time.
    pub committer_date: String,
    /// The first line of the message.
    pub subject: String,
}

/// What a ref is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefKind {
    /// A local branch.
    Local(String),
    /// A remote-tracking branch.
    Remote {
        /// The remote name, for example `origin`.
        remote: String,
        /// The branch name on the remote.
        branch: String,
    },
    /// A tag.
    Tag(String),
}

/// A branch or tag and the commit it points to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ref {
    /// The hash of the commit.
    pub target: String,
    /// What the ref is.
    pub kind: RefKind,
    /// Whether this is the checked out branch.
    pub is_head: bool,
}

impl Git {
    /// Up to `count` commits of all refs in topological order, after skipping `skip`.
    ///
    /// # Errors
    ///
    /// Fails when git fails or its output cannot be parsed.
    pub fn log(&self, skip: usize, count: usize, log: &Log<'_>) -> Result<Vec<Commit>, GitError> {
        let output: Output = self.run(&Command::Log { skip, count }, None, log)?;
        parse_log(&output.stdout)
    }

    /// All local branches, remote-tracking branches and tags.
    ///
    /// # Errors
    ///
    /// Fails when git fails or its output cannot be parsed.
    pub fn refs(&self, log: &Log<'_>) -> Result<Vec<Ref>, GitError> {
        let output: Output = self.run(&Command::Refs, None, log)?;
        parse_refs(&output.stdout)
    }

    /// The commit `HEAD` points to, or `None` in a repository without commits.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    pub fn head(&self, log: &Log<'_>) -> Result<Option<String>, GitError> {
        let output: Output = self.run(&Command::HasHead, None, log)?;
        let hash: &str = output.stdout.trim();
        Ok((output.code == 0 && !hash.is_empty()).then(|| hash.to_string()))
    }
}

impl Git {
    /// Shows commit `hash` with `git show` on the terminal, in the user's pager, so the caller
    /// must hand the terminal over. When `LESS` is not set, git would run less with `FRX`,
    /// where `F` quits at once when the commit fits on one screen and the application would
    /// redraw over it. `LESS=R` keeps less open until the user quits it, with colors.
    ///
    /// # Errors
    ///
    /// Fails when git fails, for example when the commit does not exist.
    pub fn show_commit(&self, hash: &str, log: &Log<'_>) -> Result<(), GitError> {
        let command: Command = Command::ShowCommit {
            hash: hash.to_string(),
        };
        let less: Option<OsString> = std::env::var_os("LESS");
        with_less_default(self.clone(), less.as_deref()).run_interactive(&command, log)
    }

    /// The full message of a commit.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    pub fn commit_message(&self, hash: &str, log: &Log<'_>) -> Result<String, GitError> {
        let command: Command = Command::CommitMessage {
            hash: hash.to_string(),
        };
        Ok(self.run(&command, None, log)?.stdout)
    }

    /// The files a commit changed, compared with its first parent.
    ///
    /// # Errors
    ///
    /// Fails when git fails or its output cannot be parsed.
    pub fn commit_files(&self, hash: &str, log: &Log<'_>) -> Result<Vec<FileChange>, GitError> {
        let command: Command = Command::CommitFiles {
            hash: hash.to_string(),
        };
        parse_name_status(&self.run(&command, None, log)?.stdout)
    }

    /// The diff of one file of a commit, compared with its first parent. `paths` is the path,
    /// or the original and new path of a rename.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    pub fn commit_file_diff(
        &self,
        hash: &str,
        paths: &[String],
        log: &Log<'_>,
    ) -> Result<FileDiff, GitError> {
        let command: Command = Command::CommitFileDiff {
            hash: hash.to_string(),
            paths: paths.to_vec(),
        };
        Ok(parse_diff(&self.run(&command, None, log)?.stdout))
    }
}

/// Parses `--name-status -z` output: a status, then one path, or two for renames and copies.
///
/// # Errors
///
/// Returns [`GitError::Parse`] for unknown statuses or missing paths.
pub fn parse_name_status(output: &str) -> Result<Vec<FileChange>, GitError> {
    let mut changes: Vec<FileChange> = Vec::new();
    let mut tokens = output.split('\0').filter(|t: &&str| !t.is_empty());
    while let Some(status) = tokens.next() {
        let error = || GitError::Parse(format!("unexpected name status {status:?}"));
        let kind: ChangeKind = match status.chars().next() {
            Some('M') => ChangeKind::Modified,
            Some('A') => ChangeKind::Added,
            Some('D') => ChangeKind::Deleted,
            Some('R') => ChangeKind::Renamed,
            Some('C') => ChangeKind::Copied,
            Some('T') => ChangeKind::TypeChanged,
            _ => return Err(error()),
        };
        let first: &str = tokens.next().ok_or_else(error)?;
        let (path, original_path): (String, Option<String>) =
            if matches!(kind, ChangeKind::Renamed | ChangeKind::Copied) {
                let second: &str = tokens.next().ok_or_else(error)?;
                (second.to_string(), Some(first.to_string()))
            } else {
                (first.to_string(), None)
            };
        changes.push(FileChange {
            path,
            original_path,
            kind,
            mode_changed: false,
        });
    }
    Ok(changes)
}

/// Parses `git log -z` output in the format of [`Command::Log`].
///
/// # Errors
///
/// Returns [`GitError::Parse`] for records without all fields.
pub fn parse_log(output: &str) -> Result<Vec<Commit>, GitError> {
    output
        .split('\0')
        .map(|record: &str| record.trim_start_matches('\n'))
        .filter(|record: &&str| !record.is_empty())
        .map(|record: &str| {
            let fields: Vec<&str> = record.splitn(7, '\u{1f}').collect();
            let [
                hash,
                parents,
                author,
                date,
                committer,
                committer_date,
                subject,
            ] = fields[..]
            else {
                return Err(GitError::Parse(format!("unexpected log record {record:?}")));
            };
            Ok(Commit {
                hash: hash.to_string(),
                parents: parents.split_whitespace().map(str::to_string).collect(),
                author: author.to_string(),
                date: date.to_string(),
                committer: committer.to_string(),
                committer_date: committer_date.to_string(),
                subject: subject.to_string(),
            })
        })
        .collect()
}

/// Parses `git for-each-ref` output in the format of [`Command::Refs`]. Annotated tags point
/// to the commit they peel to. Symbolic remote refs such as `origin/HEAD` are left out.
///
/// # Errors
///
/// Returns [`GitError::Parse`] for lines without all fields.
pub fn parse_refs(output: &str) -> Result<Vec<Ref>, GitError> {
    let mut refs: Vec<Ref> = Vec::new();
    for line in output.lines().filter(|l: &&str| !l.is_empty()) {
        let fields: Vec<&str> = line.split('\0').collect();
        let [head, object, peeled, name] = fields[..] else {
            return Err(GitError::Parse(format!("unexpected ref line {line:?}")));
        };
        let target: &str = if peeled.is_empty() { object } else { peeled };
        let kind: Option<RefKind> = if let Some(branch) = name.strip_prefix("refs/heads/") {
            Some(RefKind::Local(branch.to_string()))
        } else if let Some(tag) = name.strip_prefix("refs/tags/") {
            Some(RefKind::Tag(tag.to_string()))
        } else {
            name.strip_prefix("refs/remotes/")
                .and_then(|rest: &str| rest.split_once('/'))
                .filter(|(_, branch): &(&str, &str)| *branch != "HEAD")
                .map(|(remote, branch): (&str, &str)| RefKind::Remote {
                    remote: remote.to_string(),
                    branch: branch.to_string(),
                })
        };
        if let Some(kind) = kind {
            refs.push(Ref {
                target: target.to_string(),
                kind,
                is_head: head == "*",
            });
        }
    }
    Ok(refs)
}

/// `git` with `LESS=R` when the user has not set `LESS` (`less`), see [`Git::show_commit`].
fn with_less_default(git: Git, less: Option<&OsStr>) -> Git {
    match less {
        Some(_) => git,
        None => git.with_env("LESS", "R"),
    }
}

#[cfg(test)]
mod tests;
