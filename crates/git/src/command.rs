//! Typed git commands.

/// A git command this application runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// `git rev-parse --show-toplevel`: the root directory of the repository.
    ShowToplevel,
    /// `git status`: staged, unstaged and untracked files.
    Status,
    /// `git diff` of the given paths, staged (index against `HEAD`) or unstaged (working tree
    /// against index).
    Diff {
        /// The paths: one, or the original and new path of a rename.
        paths: Vec<String>,
        /// Whether to show staged changes.
        staged: bool,
    },
    /// `git diff --no-index` of an untracked file, showing all of it as added.
    DiffUntracked {
        /// The path of the file.
        path: String,
    },
    /// `git add -A`: stages all changes of the paths, including deletions.
    Add {
        /// The paths.
        paths: Vec<String>,
    },
    /// `git rev-parse --verify HEAD`: whether there is a commit yet. Exit code 1 means no.
    HasHead,
    /// `git restore --staged`: unstages the paths.
    RestoreStaged {
        /// The paths.
        paths: Vec<String>,
    },
    /// `git rm --cached`: unstages the paths in a repository without commits.
    RemoveCached {
        /// The paths.
        paths: Vec<String>,
    },
    /// `git apply --cached`: applies a patch from standard input to the index.
    ApplyCached {
        /// Whether to apply the patch in reverse.
        reverse: bool,
    },
    /// `git log -1 --format=%B`: the message of the last commit.
    LastMessage,
    /// `git commit -F <file>`: commits with the message in a file.
    Commit {
        /// The file that contains the message.
        message_file: String,
        /// Whether to amend the last commit.
        amend: bool,
    },
}

impl Command {
    /// The command line arguments, without the leading `git`.
    #[must_use]
    pub fn args(&self) -> Vec<String> {
        let fixed: &[&str] = match self {
            Command::ShowToplevel => &["rev-parse", "--show-toplevel"],
            Command::Status => &["status", "--porcelain=v2", "-z", "--untracked-files=all"],
            Command::Diff { staged: false, .. } => &["diff", "--no-ext-diff", "--"],
            Command::Diff { staged: true, .. } => {
                &["diff", "--no-ext-diff", "--cached", "-M", "--"]
            }
            Command::DiffUntracked { .. } => {
                &["diff", "--no-ext-diff", "--no-index", "--", "/dev/null"]
            }
            Command::Add { .. } => &["add", "-A", "--"],
            Command::HasHead => &["rev-parse", "--verify", "-q", "HEAD"],
            Command::RestoreStaged { .. } => &["restore", "--staged", "--"],
            Command::RemoveCached { .. } => &["rm", "--cached", "-q", "-r", "--"],
            Command::ApplyCached { reverse: false } => &["apply", "--cached"],
            Command::ApplyCached { reverse: true } => &["apply", "--cached", "--reverse"],
            Command::LastMessage => &["log", "-1", "--format=%B"],
            Command::Commit { amend: false, .. } => &["commit", "-F"],
            Command::Commit { amend: true, .. } => &["commit", "--amend", "-F"],
        };
        let mut args: Vec<String> = fixed.iter().map(|a: &&str| (*a).to_string()).collect();
        match self {
            Command::Diff { paths, .. }
            | Command::Add { paths }
            | Command::RestoreStaged { paths }
            | Command::RemoveCached { paths } => args.extend(paths.iter().cloned()),
            Command::DiffUntracked { path } => args.push(path.clone()),
            Command::Commit { message_file, .. } => args.push(message_file.clone()),
            Command::ShowToplevel
            | Command::Status
            | Command::HasHead
            | Command::ApplyCached { .. }
            | Command::LastMessage => {}
        }
        args
    }

    /// Exit codes that mean success for this command.
    #[must_use]
    pub fn success_codes(&self) -> &'static [i32] {
        match self {
            Command::DiffUntracked { .. } | Command::HasHead => &[0, 1],
            _ => &[0],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Command;

    fn args(command: &Command) -> String {
        command.args().join(" ")
    }

    #[test]
    fn arguments() {
        assert_eq!(args(&Command::ShowToplevel), "rev-parse --show-toplevel");
        assert_eq!(
            args(&Command::Status),
            "status --porcelain=v2 -z --untracked-files=all"
        );
        assert_eq!(
            args(&Command::Diff {
                paths: vec!["a b".to_string()],
                staged: false
            }),
            "diff --no-ext-diff -- a b"
        );
        assert_eq!(
            args(&Command::Diff {
                paths: vec!["old".to_string(), "new".to_string()],
                staged: true
            }),
            "diff --no-ext-diff --cached -M -- old new"
        );
        assert_eq!(
            args(&Command::DiffUntracked {
                path: "n".to_string()
            }),
            "diff --no-ext-diff --no-index -- /dev/null n"
        );
    }

    #[test]
    fn staging_arguments() {
        let paths: Vec<String> = vec!["p".to_string()];
        assert_eq!(
            args(&Command::Add {
                paths: paths.clone()
            }),
            "add -A -- p"
        );
        assert_eq!(args(&Command::HasHead), "rev-parse --verify -q HEAD");
        assert_eq!(
            args(&Command::RestoreStaged {
                paths: paths.clone()
            }),
            "restore --staged -- p"
        );
        assert_eq!(
            args(&Command::RemoveCached { paths }),
            "rm --cached -q -r -- p"
        );
        assert_eq!(
            args(&Command::ApplyCached { reverse: false }),
            "apply --cached"
        );
        assert_eq!(
            args(&Command::ApplyCached { reverse: true }),
            "apply --cached --reverse"
        );
        assert_eq!(Command::HasHead.success_codes(), &[0, 1]);
    }

    #[test]
    fn commit_arguments() {
        assert_eq!(args(&Command::LastMessage), "log -1 --format=%B");
        assert_eq!(
            args(&Command::Commit {
                message_file: "/tmp/m".to_string(),
                amend: false
            }),
            "commit -F /tmp/m"
        );
        assert_eq!(
            args(&Command::Commit {
                message_file: "/tmp/m".to_string(),
                amend: true
            }),
            "commit --amend -F /tmp/m"
        );
    }

    #[test]
    fn success_codes() {
        assert_eq!(Command::Status.success_codes(), &[0]);
        assert_eq!(
            Command::DiffUntracked {
                path: String::new()
            }
            .success_codes(),
            &[0, 1]
        );
    }
}
