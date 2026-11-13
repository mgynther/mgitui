//! Validation errors.

use std::fmt;

use crate::branch::BranchNameProblem;

/// Why user input was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// The branch name breaks one of git's naming rules.
    BranchName(BranchNameProblem),
    /// The checkout choice is not a number.
    NotANumber,
    /// The checkout choice is not one of the listed numbers `1..=count`.
    OutOfRange {
        /// How many choices were listed.
        count: usize,
    },
    /// The search text is empty.
    EmptySearch,
    /// The commit message is empty.
    EmptyCommitMessage,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::BranchName(problem) => write!(f, "invalid branch name: {problem}"),
            ValidationError::NotANumber => write!(f, "choice is not a number"),
            ValidationError::OutOfRange { count } => {
                write!(f, "choice must be a number from 1 to {count}")
            }
            ValidationError::EmptySearch => write!(f, "search text is empty"),
            ValidationError::EmptyCommitMessage => write!(f, "commit message is empty"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ValidationError;
    use crate::branch::BranchNameProblem;

    #[test]
    fn messages() {
        let cases: [(ValidationError, &str); 5] = [
            (
                ValidationError::BranchName(BranchNameProblem::Empty),
                "invalid branch name: it is empty",
            ),
            (ValidationError::NotANumber, "choice is not a number"),
            (
                ValidationError::OutOfRange { count: 3 },
                "choice must be a number from 1 to 3",
            ),
            (ValidationError::EmptySearch, "search text is empty"),
            (
                ValidationError::EmptyCommitMessage,
                "commit message is empty",
            ),
        ];
        for (error, message) in cases {
            assert_eq!(error.to_string(), message);
        }
    }
}
