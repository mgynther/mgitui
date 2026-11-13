//! The functions logic uses to get things done, provided by wiring.

use crate::log::LogLevel;

/// Search text that passed validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchText(pub String);

/// Everything outside the application state that logic needs. Failures are logged by the
/// layer where they happen; the error values only carry a short message for the status line.
pub trait Ports {
    /// Logs a message from the logic layer.
    fn log(&self, level: LogLevel, message: &str);

    /// Validates search text typed by the user.
    ///
    /// # Errors
    ///
    /// Returns a short description when the text is not valid.
    fn validate_search(&self, input: &str) -> Result<SearchText, String>;
}
