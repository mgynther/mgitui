//! The functions logic uses to get things done, provided by wiring.

use crate::log::LogLevel;

/// Everything outside the application state that logic needs.
pub trait Ports {
    /// Logs a message from the logic layer.
    fn log(&self, level: LogLevel, message: &str);
}
