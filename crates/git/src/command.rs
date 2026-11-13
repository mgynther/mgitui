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
        };
        let mut args: Vec<String> = fixed.iter().map(|a: &&str| (*a).to_string()).collect();
        match self {
            Command::Diff { paths, .. } => args.extend(paths.iter().cloned()),
            Command::DiffUntracked { path } => args.push(path.clone()),
            Command::ShowToplevel | Command::Status => {}
        }
        args
    }

    /// Exit codes that mean success for this command.
    #[must_use]
    pub fn success_codes(&self) -> &'static [i32] {
        match self {
            Command::DiffUntracked { .. } => &[0, 1],
            Command::ShowToplevel | Command::Status | Command::Diff { .. } => &[0],
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
