//! The functions logic uses to get things done, provided by wiring.

use crate::log::LogLevel;
use crate::model::{ChangedFile, FileDiff, Side, WorkStatus};

/// Search text that passed validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchText(pub String);

/// A failure that was already logged by the layer where it happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Failed;

/// Everything outside the application state that logic needs. Failures are logged by the
/// layer where they happen; the error values only carry what the status line needs.
pub trait Ports {
    /// Logs a message from the logic layer.
    fn log(&self, level: LogLevel, message: &str);

    /// Validates search text typed by the user.
    ///
    /// # Errors
    ///
    /// Returns a short description when the text is not valid.
    fn validate_search(&self, input: &str) -> Result<SearchText, String>;

    /// The staged and unstaged changes.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    fn status(&self) -> Result<WorkStatus, Failed>;

    /// The diff of a changed file in the staged or unstaged list.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    fn file_diff(&self, file: &ChangedFile, side: Side) -> Result<FileDiff, Failed>;
}
