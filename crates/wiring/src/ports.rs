//! The functions wiring provides to the logic layer.

use crate::log::LogBuffer;

/// Implements the logic layer's ports with the git and validation layers.
pub struct SessionPorts<'a> {
    /// Collects log messages from every layer.
    pub log: &'a LogBuffer,
    /// Runs git in the repository.
    pub git: &'a git::Git,
}

impl logic::ports::Ports for SessionPorts<'_> {
    fn log(&self, level: logic::log::LogLevel, message: &str) {
        self.log.logic(level, message);
    }
}

#[cfg(test)]
mod tests {
    use super::SessionPorts;
    use crate::log::LogBuffer;
    use crate::test_support::fixed_clock;
    use logic::log::{Layer, LogEntry, LogLevel};
    use logic::ports::Ports;

    #[test]
    fn logic_log_goes_to_buffer() {
        let log: LogBuffer = LogBuffer::new(fixed_clock);
        let git: git::Git = git::Git::new(std::path::Path::new("."));
        let ports: SessionPorts<'_> = SessionPorts {
            log: &log,
            git: &git,
        };
        ports.log(LogLevel::Info, "hello");
        let entries: Vec<LogEntry> = log.drain();
        assert_eq!(entries[0].layer, Layer::Logic);
        assert_eq!(entries[0].message, "hello");
    }
}
