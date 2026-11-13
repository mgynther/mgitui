//! Reading the commit history and refs.

use crate::command::Command;
use crate::log::Log;
use crate::runner::{Git, GitError, Output};

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
            let fields: Vec<&str> = record.splitn(5, '\u{1f}').collect();
            let [hash, parents, author, date, subject] = fields[..] else {
                return Err(GitError::Parse(format!("unexpected log record {record:?}")));
            };
            Ok(Commit {
                hash: hash.to_string(),
                parents: parents.split_whitespace().map(str::to_string).collect(),
                author: author.to_string(),
                date: date.to_string(),
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

#[cfg(test)]
mod tests;
