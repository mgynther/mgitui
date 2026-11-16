//! Validation of checkout choices, search text and commit messages.

use crate::error::ValidationError;
use crate::log::{Log, LogLevel};

/// A checkout choice that is one of the listed numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckoutChoice(usize);

impl CheckoutChoice {
    /// The zero-based index of the chosen item.
    #[must_use]
    pub fn index(self) -> usize {
        self.0
    }
}

/// Search text that is not empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchText(String);

impl SearchText {
    /// The search text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A commit message that is not empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitMessage(String);

impl CommitMessage {
    /// The commit message.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Validates a number typed to choose one of `count` listed items, numbered from 1.
///
/// # Errors
///
/// Returns [`ValidationError::NotANumber`] or [`ValidationError::OutOfRange`]. The error is
/// also logged.
pub fn checkout_choice(
    input: &str,
    count: usize,
    log: &Log<'_>,
) -> Result<CheckoutChoice, ValidationError> {
    let result: Result<CheckoutChoice, ValidationError> = match input.trim().parse::<usize>() {
        Err(_) => Err(ValidationError::NotANumber),
        Ok(number) if number >= 1 && number <= count => Ok(CheckoutChoice(number - 1)),
        Ok(_) => Err(ValidationError::OutOfRange { count }),
    };
    log_rejected(&result, input, log);
    result
}

/// Validates search text.
///
/// # Errors
///
/// Returns [`ValidationError::EmptySearch`] when the text is empty. The error is also logged.
pub fn search_text(input: &str, log: &Log<'_>) -> Result<SearchText, ValidationError> {
    let result: Result<SearchText, ValidationError> = if input.is_empty() {
        Err(ValidationError::EmptySearch)
    } else {
        Ok(SearchText(input.to_string()))
    };
    log_rejected(&result, input, log);
    result
}

/// Validates a commit message. A message with only whitespace counts as empty.
///
/// # Errors
///
/// Returns [`ValidationError::EmptyCommitMessage`] when the message is empty. The error is also
/// logged.
pub fn commit_message(input: &str, log: &Log<'_>) -> Result<CommitMessage, ValidationError> {
    let result: Result<CommitMessage, ValidationError> = if input.trim().is_empty() {
        Err(ValidationError::EmptyCommitMessage)
    } else {
        Ok(CommitMessage(input.to_string()))
    };
    log_rejected(&result, input, log);
    result
}

fn log_rejected<T>(result: &Result<T, ValidationError>, input: &str, log: &Log<'_>) {
    if let Err(error) = result {
        log(LogLevel::Info, &format!("{error}: {input:?}"));
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::{checkout_choice, commit_message, search_text};
    use crate::error::ValidationError;
    use crate::log::LogLevel;

    struct Logged(RefCell<Vec<(LogLevel, String)>>);

    impl Logged {
        fn new() -> Self {
            Logged(RefCell::new(Vec::new()))
        }

        fn log(&self, level: LogLevel, message: &str) {
            self.0.borrow_mut().push((level, message.to_string()));
        }

        fn lines(&self) -> Vec<(LogLevel, String)> {
            self.0.borrow().clone()
        }
    }

    #[test]
    fn checkout_choice_accepts_only_listed_numbers() {
        let logged: Logged = Logged::new();
        let log = |level: LogLevel, message: &str| logged.log(level, message);
        assert_eq!(
            checkout_choice("1", 3, &log).map(super::CheckoutChoice::index),
            Ok(0)
        );
        assert_eq!(
            checkout_choice(" 3 ", 3, &log).map(super::CheckoutChoice::index),
            Ok(2)
        );
        assert_eq!(logged.lines(), Vec::new());
        assert_eq!(
            checkout_choice("x", 3, &log),
            Err(ValidationError::NotANumber)
        );
        assert_eq!(
            checkout_choice("0", 3, &log),
            Err(ValidationError::OutOfRange { count: 3 })
        );
        assert_eq!(
            checkout_choice("4", 3, &log),
            Err(ValidationError::OutOfRange { count: 3 })
        );
        assert_eq!(
            logged.lines(),
            vec![
                (LogLevel::Info, "choice is not a number: \"x\"".to_string()),
                (
                    LogLevel::Info,
                    "choice must be a number from 1 to 3: \"0\"".to_string()
                ),
                (
                    LogLevel::Info,
                    "choice must be a number from 1 to 3: \"4\"".to_string()
                ),
            ]
        );
    }

    #[test]
    fn search_text_must_not_be_empty() {
        let logged: Logged = Logged::new();
        let log = |level: LogLevel, message: &str| logged.log(level, message);
        assert_eq!(
            search_text(" a ", &log).map(|s| s.as_str().to_string()),
            Ok(" a ".to_string())
        );
        assert_eq!(search_text("", &log), Err(ValidationError::EmptySearch));
        assert_eq!(
            logged.lines(),
            vec![(LogLevel::Info, "search text is empty: \"\"".to_string())]
        );
    }

    #[test]
    fn commit_message_must_not_be_empty() {
        let logged: Logged = Logged::new();
        let log = |level: LogLevel, message: &str| logged.log(level, message);
        assert_eq!(
            commit_message("Fix\n\nBody", &log).map(|m| m.as_str().to_string()),
            Ok("Fix\n\nBody".to_string())
        );
        assert_eq!(
            commit_message(" \n ", &log),
            Err(ValidationError::EmptyCommitMessage)
        );
        assert_eq!(
            logged.lines(),
            vec![(
                LogLevel::Info,
                "commit message is empty: \" \\n \"".to_string()
            )]
        );
    }
}
