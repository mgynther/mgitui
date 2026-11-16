//! Branch name validation following git's ref naming rules (see `git check-ref-format`).

use std::fmt;

use crate::error::ValidationError;
use crate::log::{Log, LogLevel};

/// A branch name that follows git's naming rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchName(String);

impl BranchName {
    /// The branch name as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The git naming rule a branch name breaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchNameProblem {
    /// The name is empty.
    Empty,
    /// The name is `@` or `HEAD`.
    Reserved,
    /// The name starts with `-`.
    StartsWithDash,
    /// The name starts or ends with `/`, or contains `//`.
    BadSlash,
    /// A part between slashes starts with `.`.
    ComponentStartsWithDot,
    /// A part between slashes ends with `.lock`.
    ComponentEndsWithLock,
    /// The name ends with `.`.
    EndsWithDot,
    /// The name contains `..`.
    DoubleDot,
    /// The name contains `@{`.
    AtBrace,
    /// The name contains a space, a control character or one of `~ ^ : ? * [ \`.
    ForbiddenCharacter(char),
}

impl fmt::Display for BranchNameProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BranchNameProblem::Empty => write!(f, "it is empty"),
            BranchNameProblem::Reserved => write!(f, "it is reserved by git"),
            BranchNameProblem::StartsWithDash => write!(f, "it starts with '-'"),
            BranchNameProblem::BadSlash => {
                write!(f, "it starts or ends with '/' or contains '//'")
            }
            BranchNameProblem::ComponentStartsWithDot => write!(f, "a part starts with '.'"),
            BranchNameProblem::ComponentEndsWithLock => write!(f, "a part ends with '.lock'"),
            BranchNameProblem::EndsWithDot => write!(f, "it ends with '.'"),
            BranchNameProblem::DoubleDot => write!(f, "it contains '..'"),
            BranchNameProblem::AtBrace => write!(f, "it contains '@{{'"),
            BranchNameProblem::ForbiddenCharacter(c) => {
                write!(f, "it contains the character {c:?}")
            }
        }
    }
}

/// Validates a branch name typed by the user.
///
/// # Errors
///
/// Returns [`ValidationError::BranchName`] when the name breaks a git naming rule. The error is
/// also logged.
pub fn branch_name(input: &str, log: &Log<'_>) -> Result<BranchName, ValidationError> {
    match find_problem(input) {
        None => Ok(BranchName(input.to_string())),
        Some(problem) => {
            let error: ValidationError = ValidationError::BranchName(problem);
            log(LogLevel::Info, &format!("{error}: {input:?}"));
            Err(error)
        }
    }
}

fn find_problem(name: &str) -> Option<BranchNameProblem> {
    if name.is_empty() {
        return Some(BranchNameProblem::Empty);
    }
    if name == "@" || name == "HEAD" {
        return Some(BranchNameProblem::Reserved);
    }
    if name.starts_with('-') {
        return Some(BranchNameProblem::StartsWithDash);
    }
    if let Some(c) = name.chars().find(|c: &char| is_forbidden(*c)) {
        return Some(BranchNameProblem::ForbiddenCharacter(c));
    }
    if name.starts_with('/') || name.ends_with('/') || name.contains("//") {
        return Some(BranchNameProblem::BadSlash);
    }
    if name.contains("..") {
        return Some(BranchNameProblem::DoubleDot);
    }
    if name.contains("@{") {
        return Some(BranchNameProblem::AtBrace);
    }
    if name.ends_with('.') {
        return Some(BranchNameProblem::EndsWithDot);
    }
    for component in name.split('/') {
        if component.starts_with('.') {
            return Some(BranchNameProblem::ComponentStartsWithDot);
        }
        if component.strip_suffix(".lock").is_some() {
            return Some(BranchNameProblem::ComponentEndsWithLock);
        }
    }
    None
}

fn is_forbidden(c: char) -> bool {
    c.is_ascii_control() || matches!(c, ' ' | '~' | '^' | ':' | '?' | '*' | '[' | '\\')
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::{BranchNameProblem, branch_name};
    use crate::error::ValidationError;
    use crate::log::LogLevel;

    /// Validates `input`, also checking that a refused name is logged once.
    fn check(input: &str) -> Result<String, ValidationError> {
        let logged: RefCell<Vec<(LogLevel, String)>> = RefCell::new(Vec::new());
        let log = |level: LogLevel, message: &str| {
            logged.borrow_mut().push((level, message.to_string()));
        };
        let result: Result<String, ValidationError> =
            branch_name(input, &log).map(|name| name.as_str().to_string());
        let expected_log_lines: usize = usize::from(result.is_err());
        assert_eq!(logged.borrow().len(), expected_log_lines);
        if let Some((level, message)) = logged.borrow().first() {
            assert_eq!(*level, LogLevel::Info);
            assert!(message.starts_with("invalid branch name: "));
        }
        result
    }

    #[test]
    fn valid_names() {
        for name in [
            "main",
            "feature/login",
            "fix-1.2",
            "a/b/c",
            "x@y",
            "ünicode",
        ] {
            assert_eq!(check(name), Ok(name.to_string()));
        }
    }

    #[test]
    fn invalid_names() {
        let cases: [(&str, BranchNameProblem); 17] = [
            ("", BranchNameProblem::Empty),
            ("@", BranchNameProblem::Reserved),
            ("HEAD", BranchNameProblem::Reserved),
            ("-x", BranchNameProblem::StartsWithDash),
            ("a b", BranchNameProblem::ForbiddenCharacter(' ')),
            ("a~b", BranchNameProblem::ForbiddenCharacter('~')),
            ("a\tb", BranchNameProblem::ForbiddenCharacter('\t')),
            ("a\\b", BranchNameProblem::ForbiddenCharacter('\\')),
            ("/a", BranchNameProblem::BadSlash),
            ("a/", BranchNameProblem::BadSlash),
            ("a//b", BranchNameProblem::BadSlash),
            ("a..b", BranchNameProblem::DoubleDot),
            ("a@{b", BranchNameProblem::AtBrace),
            ("a.", BranchNameProblem::EndsWithDot),
            (".a", BranchNameProblem::ComponentStartsWithDot),
            ("a/.b", BranchNameProblem::ComponentStartsWithDot),
            ("a.lock/b", BranchNameProblem::ComponentEndsWithLock),
        ];
        for (name, problem) in cases {
            assert_eq!(
                check(name),
                Err(ValidationError::BranchName(problem)),
                "{name:?}"
            );
        }
    }

    #[test]
    fn problem_messages() {
        let cases: [(BranchNameProblem, &str); 10] = [
            (BranchNameProblem::Empty, "it is empty"),
            (BranchNameProblem::Reserved, "it is reserved by git"),
            (BranchNameProblem::StartsWithDash, "it starts with '-'"),
            (
                BranchNameProblem::BadSlash,
                "it starts or ends with '/' or contains '//'",
            ),
            (
                BranchNameProblem::ComponentStartsWithDot,
                "a part starts with '.'",
            ),
            (
                BranchNameProblem::ComponentEndsWithLock,
                "a part ends with '.lock'",
            ),
            (BranchNameProblem::EndsWithDot, "it ends with '.'"),
            (BranchNameProblem::DoubleDot, "it contains '..'"),
            (BranchNameProblem::AtBrace, "it contains '@{'"),
            (
                BranchNameProblem::ForbiddenCharacter('*'),
                "it contains the character '*'",
            ),
        ];
        for (problem, message) in cases {
            assert_eq!(problem.to_string(), message);
        }
    }
}
