//! Logging as seen by the git layer. The logging function is provided by wiring.

/// Severity of a log message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    /// Something failed.
    Error,
    /// Something needs attention.
    Warn,
    /// Normal activity.
    Info,
    /// Details for debugging.
    Debug,
}

/// The logging function this layer uses.
pub type Log<'a> = dyn Fn(LogLevel, &str) + 'a;
