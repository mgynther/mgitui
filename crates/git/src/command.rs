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
    /// `git add -A`: stages all changes in the working tree, including deletions and untracked
    /// files.
    AddAll,
    /// `git rev-parse --verify -q HEAD^`: whether the last commit has a parent. Exit code 1
    /// means no.
    HasParent,
    /// `git hash-object -t tree /dev/null`: the id of the empty tree, which depends on the
    /// repository's hash format.
    EmptyTree,
    /// `git diff --cached --name-status -z -M <base>`: the files whose index content differs
    /// from `base`.
    StagedSince {
        /// The commit or tree to compare the index with.
        base: String,
    },
    /// `git diff --cached <base> -- <paths>`: the diff of the index against `base`.
    DiffSince {
        /// The commit or tree to compare the index with.
        base: String,
        /// The paths: one, or the original and new path of a rename.
        paths: Vec<String>,
    },
    /// `git reset -q HEAD^ -- <paths>`: sets the index entries of the paths to the parent of
    /// the last commit, removing paths that it does not have.
    ResetToParent {
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
    /// `git restore`: discards the unstaged changes of tracked paths, restoring them from the
    /// index.
    Restore {
        /// The paths.
        paths: Vec<String>,
    },
    /// `git clean -f`: deletes untracked files.
    Clean {
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
    /// `git log --all --topo-order`: a chunk of the history of all refs.
    Log {
        /// Number of commits to skip from the newest.
        skip: usize,
        /// Maximum number of commits.
        count: usize,
    },
    /// `git for-each-ref`: all branches and tags with the commits they point to.
    Refs,
    /// `git show -s --format=%B`: the full message of a commit.
    CommitMessage {
        /// The commit hash.
        hash: String,
    },
    /// `git show --name-status`: the files a commit changed, against its first parent.
    CommitFiles {
        /// The commit hash.
        hash: String,
    },
    /// `git show` of some paths: a commit's diff of one file, against its first parent.
    CommitFileDiff {
        /// The commit hash.
        hash: String,
        /// The path, or the original and new path of a rename.
        paths: Vec<String>,
    },
    /// `git branch <name> <commit>`: creates a branch without checking it out.
    CreateBranch {
        /// The new branch name.
        name: String,
        /// The commit the branch points to.
        at: String,
    },
    /// `git branch -D <name>`: deletes a local branch, also when it is not merged.
    DeleteBranch {
        /// The branch name.
        name: String,
    },
    /// `git remote update`: fetches all remotes.
    RemoteUpdate,
    /// `git fetch --all --prune`: fetches all remotes pruning stale remote branches.
    FetchAllPrune,
    /// `git reset --keep <commit>`: moves the checked out branch to a commit and updates the
    /// working tree. Unlike `--hard`, it refuses to overwrite local changes and untracked files.
    ResetKeep {
        /// The commit hash.
        hash: String,
    },
    /// `git show <commit>`: shows a commit in the user's pager, with the user's own git
    /// configuration. Run with [`crate::Git::run_interactive`].
    ShowCommit {
        /// The commit hash.
        hash: String,
    },
    /// `git cherry-pick <commit>`: applies a commit onto the checked out branch.
    CherryPick {
        /// The commit hash.
        hash: String,
    },
    /// `git rebase <onto> <branch>`: checks out a branch and rebases it onto another.
    Rebase {
        /// The branch to rebase onto.
        onto: String,
        /// The branch to rebase.
        branch: String,
    },
    /// `git rebase -i <onto> <branch>`: rebases interactively, with git running Vim for the
    /// todo list and messages. Run with [`crate::Git::run_interactive`].
    RebaseInteractive {
        /// The branch to rebase onto.
        onto: String,
        /// The branch to rebase.
        branch: String,
    },
    /// `git switch <branch>`: checks out a local branch.
    Switch {
        /// The branch name.
        branch: String,
    },
    /// `git commit -F <file>`: commits with the message in a file.
    Commit {
        /// The file that contains the message.
        message_file: String,
        /// Whether to amend the last commit.
        amend: bool,
    },
}

impl Command {
    /// The arguments before the variable ones, without the leading `git`.
    fn fixed_args(&self) -> &'static [&'static str] {
        match self {
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
            Command::AddAll => &["add", "-A"],
            Command::HasHead => &["rev-parse", "--verify", "-q", "HEAD"],
            Command::HasParent => &["rev-parse", "--verify", "-q", "HEAD^"],
            Command::EmptyTree => &["hash-object", "-t", "tree", "/dev/null"],
            Command::StagedSince { .. } => &["diff", "--cached", "--name-status", "-z", "-M"],
            Command::DiffSince { .. } => &["diff", "--no-ext-diff", "--cached", "-M"],
            Command::ResetToParent { .. } => &["reset", "-q", "HEAD^", "--"],
            Command::RestoreStaged { .. } => &["restore", "--staged", "--"],
            Command::RemoveCached { .. } => &["rm", "--cached", "-q", "-r", "--"],
            Command::Restore { .. } => &["restore", "--"],
            Command::Clean { .. } => &["clean", "-f", "-q", "--"],
            Command::ApplyCached { reverse: false } => &["apply", "--cached"],
            Command::ApplyCached { reverse: true } => &["apply", "--cached", "--reverse"],
            Command::LastMessage => &["log", "-1", "--format=%B"],
            Command::Log { .. } => &[
                "log",
                "--all",
                "--topo-order",
                "-z",
                "--date=format-local:%Y-%m-%d %H:%M",
                "--format=%H%x1f%P%x1f%an%x1f%ad%x1f%cn%x1f%cd%x1f%s",
            ],
            Command::CommitMessage { .. } => &["show", "-s", "--format=%B"],
            Command::CommitFiles { .. } => &[
                "show",
                "--format=",
                "--name-status",
                "-z",
                "--diff-merges=first-parent",
                "-M",
            ],
            Command::CommitFileDiff { .. } => &[
                "show",
                "--format=",
                "--no-ext-diff",
                "--diff-merges=first-parent",
                "-M",
            ],
            Command::CreateBranch { .. } => &["branch"],
            Command::Switch { .. } => &["switch"],
            Command::CherryPick { .. } => &["cherry-pick"],
            Command::ResetKeep { .. } => &["reset", "--keep"],
            Command::ShowCommit { .. } => &["show"],
            Command::RemoteUpdate => &["remote", "update"],
            Command::FetchAllPrune => &["fetch", "--all", "--prune"],
            Command::DeleteBranch { .. } => &["branch", "-D"],
            Command::Rebase { .. } => &["rebase"],
            Command::RebaseInteractive { .. } => &["rebase", "-i"],
            Command::Refs => &[
                "for-each-ref",
                "--format=%(HEAD)%00%(objectname)%00%(*objectname)%00%(refname)",
                "refs/heads",
                "refs/remotes",
                "refs/tags",
            ],
            Command::Commit { amend: false, .. } => &["commit", "-F"],
            Command::Commit { amend: true, .. } => &["commit", "--amend", "-F"],
        }
    }

    /// The command line arguments, without the leading `git`.
    #[must_use]
    pub fn args(&self) -> Vec<String> {
        let mut args: Vec<String> = self
            .fixed_args()
            .iter()
            .map(|a: &&str| (*a).to_string())
            .collect();
        if let Command::Log { skip, count } = self {
            args.push(format!("--skip={skip}"));
            args.push(format!("--max-count={count}"));
        }
        match self {
            Command::Diff { paths, .. }
            | Command::Add { paths }
            | Command::RestoreStaged { paths }
            | Command::RemoveCached { paths }
            | Command::Restore { paths }
            | Command::Clean { paths }
            | Command::ResetToParent { paths } => args.extend(paths.iter().cloned()),
            Command::DiffUntracked { path } => args.push(path.clone()),
            Command::Commit { message_file, .. } => args.push(message_file.clone()),
            Command::CommitMessage { hash } | Command::CommitFiles { hash } => {
                args.push(hash.clone());
            }
            Command::CreateBranch { name, at } => {
                args.push(name.clone());
                args.push(at.clone());
            }
            Command::Switch { branch } => args.push(branch.clone()),
            Command::CherryPick { hash }
            | Command::ResetKeep { hash }
            | Command::ShowCommit { hash } => args.push(hash.clone()),
            Command::DeleteBranch { name } => args.push(name.clone()),
            Command::Rebase { onto, branch } | Command::RebaseInteractive { onto, branch } => {
                args.push(onto.clone());
                args.push(branch.clone());
            }
            Command::CommitFileDiff { hash: base, paths } | Command::DiffSince { base, paths } => {
                args.push(base.clone());
                args.push("--".to_string());
                args.extend(paths.iter().cloned());
            }
            Command::StagedSince { base } => args.push(base.clone()),
            Command::ShowToplevel
            | Command::Status
            | Command::HasHead
            | Command::AddAll
            | Command::HasParent
            | Command::EmptyTree
            | Command::ApplyCached { .. }
            | Command::LastMessage
            | Command::Log { .. }
            | Command::Refs
            | Command::RemoteUpdate
            | Command::FetchAllPrune => {}
        }
        args
    }

    /// Environment variables this command needs. They can be overridden with
    /// [`crate::Git::with_env`].
    #[must_use]
    pub fn env(&self) -> &'static [(&'static str, &'static str)] {
        match self {
            Command::RebaseInteractive { .. } => {
                &[("GIT_SEQUENCE_EDITOR", "vim"), ("GIT_EDITOR", "vim")]
            }
            _ => &[],
        }
    }

    /// Exit codes that mean success for this command.
    #[must_use]
    pub fn success_codes(&self) -> &'static [i32] {
        match self {
            Command::DiffUntracked { .. } | Command::HasHead | Command::HasParent => &[0, 1],
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
        assert_eq!(args(&Command::AddAll), "add -A");
        assert_eq!(args(&Command::HasHead), "rev-parse --verify -q HEAD");
        assert_eq!(args(&Command::HasParent), "rev-parse --verify -q HEAD^");
        assert_eq!(args(&Command::EmptyTree), "hash-object -t tree /dev/null");
        assert_eq!(
            args(&Command::StagedSince {
                base: "b".to_string()
            }),
            "diff --cached --name-status -z -M b"
        );
        assert_eq!(
            args(&Command::DiffSince {
                base: "b".to_string(),
                paths: paths.clone()
            }),
            "diff --no-ext-diff --cached -M b -- p"
        );
        assert_eq!(
            args(&Command::ResetToParent {
                paths: paths.clone()
            }),
            "reset -q HEAD^ -- p"
        );
        assert_eq!(
            args(&Command::RestoreStaged {
                paths: paths.clone()
            }),
            "restore --staged -- p"
        );
        assert_eq!(
            args(&Command::RemoveCached {
                paths: paths.clone()
            }),
            "rm --cached -q -r -- p"
        );
        assert_eq!(
            args(&Command::Restore {
                paths: paths.clone()
            }),
            "restore -- p"
        );
        assert_eq!(args(&Command::Clean { paths }), "clean -f -q -- p");
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
    fn history_arguments() {
        assert_eq!(
            args(&Command::Log { skip: 10, count: 5 }),
            "log --all --topo-order -z --date=format-local:%Y-%m-%d %H:%M \
             --format=%H%x1f%P%x1f%an%x1f%ad%x1f%cn%x1f%cd%x1f%s --skip=10 --max-count=5"
        );
        assert!(args(&Command::Refs).starts_with("for-each-ref --format=%(HEAD)"));
    }

    #[test]
    fn commit_detail_arguments() {
        let hash: String = "abc".to_string();
        assert_eq!(
            args(&Command::CommitMessage { hash: hash.clone() }),
            "show -s --format=%B abc"
        );
        assert_eq!(
            args(&Command::CommitFiles { hash: hash.clone() }),
            "show --format= --name-status -z --diff-merges=first-parent -M abc"
        );
        assert_eq!(
            args(&Command::CommitFileDiff {
                hash,
                paths: vec!["o".to_string(), "n".to_string()]
            }),
            "show --format= --no-ext-diff --diff-merges=first-parent -M abc -- o n"
        );
    }

    #[test]
    fn branch_arguments() {
        assert_eq!(
            args(&Command::CreateBranch {
                name: "feature".to_string(),
                at: "abc".to_string()
            }),
            "branch feature abc"
        );
        assert_eq!(
            args(&Command::Switch {
                branch: "main".to_string()
            }),
            "switch main"
        );
    }

    #[test]
    fn cherry_pick_arguments() {
        assert_eq!(
            args(&Command::CherryPick {
                hash: "abc".to_string()
            }),
            "cherry-pick abc"
        );
        assert_eq!(args(&Command::RemoteUpdate), "remote update");
        assert_eq!(args(&Command::FetchAllPrune), "fetch --all --prune");
        assert_eq!(
            args(&Command::ResetKeep {
                hash: "abc".to_string()
            }),
            "reset --keep abc"
        );
        assert_eq!(
            args(&Command::ShowCommit {
                hash: "abc".to_string()
            }),
            "show abc"
        );
        assert_eq!(
            args(&Command::DeleteBranch {
                name: "old".to_string()
            }),
            "branch -D old"
        );
        assert_eq!(
            args(&Command::Rebase {
                onto: "main".to_string(),
                branch: "topic".to_string()
            }),
            "rebase main topic"
        );
        let interactive: Command = Command::RebaseInteractive {
            onto: "main".to_string(),
            branch: "topic".to_string(),
        };
        assert_eq!(args(&interactive), "rebase -i main topic");
        assert_eq!(
            interactive.env(),
            &[("GIT_SEQUENCE_EDITOR", "vim"), ("GIT_EDITOR", "vim")]
        );
        assert_eq!(Command::Status.env(), &[]);
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
