//! The wording of the messages printed on standard error when mgitui cannot run.

/// Why mgitui cannot run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExitProblem {
    /// The directory is not inside a git repository.
    NotInRepository {
        /// The directory mgitui was started in.
        dir: String,
        /// What git reported.
        detail: String,
    },
    /// The terminal could not be used.
    Terminal {
        /// What failed.
        detail: String,
    },
}

/// The message for `problem`.
#[must_use]
pub fn exit_text(problem: &ExitProblem) -> String {
    match problem {
        ExitProblem::NotInRepository { dir, detail } => {
            format!("mgitui: {dir} is not inside a git repository ({detail})")
        }
        ExitProblem::Terminal { detail } => format!("mgitui: {detail}"),
    }
}

#[cfg(test)]
mod tests {
    use super::{ExitProblem, exit_text};

    #[test]
    fn exit_texts() {
        assert_eq!(
            exit_text(&ExitProblem::NotInRepository {
                dir: "/tmp/x".to_string(),
                detail: "git failed".to_string(),
            }),
            "mgitui: /tmp/x is not inside a git repository (git failed)"
        );
        assert_eq!(
            exit_text(&ExitProblem::Terminal {
                detail: "stty failed".to_string(),
            }),
            "mgitui: stty failed"
        );
    }
}
