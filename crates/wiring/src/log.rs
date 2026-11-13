//! Collects log messages from every layer and converts them to the logic layer's log entries.

use std::cell::RefCell;
use std::time::{SystemTime, UNIX_EPOCH};

use logic::log::{Layer, LogEntry, LogLevel};

/// Returns the current time as seconds since 1970-01-01 00:00 UTC.
#[must_use]
pub fn system_clock() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d: std::time::Duration| d.as_secs())
}

/// Collects log entries until they are moved into the application's log store.
pub struct LogBuffer {
    entries: RefCell<Vec<LogEntry>>,
    clock: fn() -> u64,
}

impl LogBuffer {
    /// Creates an empty buffer that timestamps entries with `clock`.
    #[must_use]
    pub fn new(clock: fn() -> u64) -> Self {
        LogBuffer {
            entries: RefCell::new(Vec::new()),
            clock,
        }
    }

    /// Adds an entry from `layer`.
    pub fn push(&self, layer: Layer, level: LogLevel, message: &str) {
        self.entries.borrow_mut().push(LogEntry {
            time: (self.clock)(),
            layer,
            level,
            message: message.to_string(),
        });
    }

    /// Removes and returns all collected entries, oldest first.
    #[must_use]
    pub fn drain(&self) -> Vec<LogEntry> {
        self.entries.borrow_mut().drain(..).collect()
    }

    /// Logging function for the git layer.
    pub fn git(&self, level: git::log::LogLevel, message: &str) {
        self.push(Layer::Git, git_level(level), message);
    }

    /// Logging function for the logic layer.
    pub fn logic(&self, level: LogLevel, message: &str) {
        self.push(Layer::Logic, level, message);
    }

    /// Logging function for the ui layer.
    pub fn ui(&self, level: ui::log::LogLevel, message: &str) {
        self.push(Layer::Ui, ui_level(level), message);
    }

    /// Logging function for the validation layer.
    pub fn validation(&self, level: validation::log::LogLevel, message: &str) {
        self.push(Layer::Validation, validation_level(level), message);
    }

    /// Logging function for the wiring layer.
    pub fn wiring(&self, level: LogLevel, message: &str) {
        self.push(Layer::Wiring, level, message);
    }
}

fn git_level(level: git::log::LogLevel) -> LogLevel {
    match level {
        git::log::LogLevel::Error => LogLevel::Error,
        git::log::LogLevel::Warn => LogLevel::Warn,
        git::log::LogLevel::Info => LogLevel::Info,
        git::log::LogLevel::Debug => LogLevel::Debug,
    }
}

fn ui_level(level: ui::log::LogLevel) -> LogLevel {
    match level {
        ui::log::LogLevel::Error => LogLevel::Error,
        ui::log::LogLevel::Warn => LogLevel::Warn,
        ui::log::LogLevel::Info => LogLevel::Info,
        ui::log::LogLevel::Debug => LogLevel::Debug,
    }
}

fn validation_level(level: validation::log::LogLevel) -> LogLevel {
    match level {
        validation::log::LogLevel::Error => LogLevel::Error,
        validation::log::LogLevel::Warn => LogLevel::Warn,
        validation::log::LogLevel::Info => LogLevel::Info,
        validation::log::LogLevel::Debug => LogLevel::Debug,
    }
}

#[cfg(test)]
mod tests {
    use super::{LogBuffer, system_clock};
    use logic::log::{Layer, LogEntry, LogLevel};

    fn fixed_clock() -> u64 {
        42
    }

    fn summary(entries: &[LogEntry]) -> Vec<(u64, Layer, LogLevel, String)> {
        entries
            .iter()
            .map(|e: &LogEntry| (e.time, e.layer, e.level, e.message.clone()))
            .collect()
    }

    #[test]
    fn system_clock_is_after_2025() {
        assert!(system_clock() > 1_735_689_600);
    }

    #[test]
    fn every_layer_logs_with_its_name_and_level() {
        let buffer: LogBuffer = LogBuffer::new(fixed_clock);
        buffer.git(git::log::LogLevel::Error, "g1");
        buffer.git(git::log::LogLevel::Warn, "g2");
        buffer.git(git::log::LogLevel::Info, "g3");
        buffer.git(git::log::LogLevel::Debug, "g4");
        buffer.ui(ui::log::LogLevel::Error, "u1");
        buffer.ui(ui::log::LogLevel::Warn, "u2");
        buffer.ui(ui::log::LogLevel::Info, "u3");
        buffer.ui(ui::log::LogLevel::Debug, "u4");
        buffer.validation(validation::log::LogLevel::Error, "v1");
        buffer.validation(validation::log::LogLevel::Warn, "v2");
        buffer.validation(validation::log::LogLevel::Info, "v3");
        buffer.validation(validation::log::LogLevel::Debug, "v4");
        buffer.logic(LogLevel::Info, "l");
        buffer.wiring(LogLevel::Warn, "w");
        let entries: Vec<LogEntry> = buffer.drain();
        let levels: [LogLevel; 4] = [
            LogLevel::Error,
            LogLevel::Warn,
            LogLevel::Info,
            LogLevel::Debug,
        ];
        let mut expected: Vec<(u64, Layer, LogLevel, String)> = Vec::new();
        for (layer, prefix) in [
            (Layer::Git, "g"),
            (Layer::Ui, "u"),
            (Layer::Validation, "v"),
        ] {
            for (index, level) in levels.iter().enumerate() {
                expected.push((42, layer, *level, format!("{prefix}{}", index + 1)));
            }
        }
        expected.push((42, Layer::Logic, LogLevel::Info, "l".to_string()));
        expected.push((42, Layer::Wiring, LogLevel::Warn, "w".to_string()));
        assert_eq!(summary(&entries), expected);
        assert_eq!(buffer.drain(), Vec::<LogEntry>::new());
    }
}
