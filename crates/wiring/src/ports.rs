//! The functions wiring provides to the logic layer.

use crate::log::LogBuffer;

/// Implements the logic layer's ports with the git and validation layers.
pub struct SessionPorts<'a> {
    /// Collects log messages from every layer.
    pub log: &'a LogBuffer,
    /// Runs git in the repository.
    pub git: &'a git::Git,
}

impl SessionPorts<'_> {
    fn log_validation(&self) -> impl Fn(validation::log::LogLevel, &str) + '_ {
        |level: validation::log::LogLevel, message: &str| self.log.validation(level, message)
    }
}

impl logic::ports::Ports for SessionPorts<'_> {
    fn log(&self, level: logic::log::LogLevel, message: &str) {
        self.log.logic(level, message);
    }

    fn validate_search(&self, input: &str) -> Result<logic::ports::SearchText, String> {
        validation::search_text(input, &self.log_validation())
            .map(|text: validation::SearchText| logic::ports::SearchText(text.as_str().to_string()))
            .map_err(|error: validation::ValidationError| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::SessionPorts;
    use crate::log::LogBuffer;
    use crate::test_support::fixed_clock;
    use logic::log::{Layer, LogEntry, LogLevel};
    use logic::ports::{Ports, SearchText};

    fn with_ports(test: impl Fn(&SessionPorts<'_>, &LogBuffer)) {
        let log: LogBuffer = LogBuffer::new(fixed_clock);
        let git: git::Git = git::Git::new(std::path::Path::new("."));
        let ports: SessionPorts<'_> = SessionPorts {
            log: &log,
            git: &git,
        };
        test(&ports, &log);
    }

    #[test]
    fn logic_log_goes_to_buffer() {
        with_ports(|ports: &SessionPorts<'_>, log: &LogBuffer| {
            ports.log(LogLevel::Info, "hello");
            let entries: Vec<LogEntry> = log.drain();
            assert_eq!(entries[0].layer, Layer::Logic);
            assert_eq!(entries[0].message, "hello");
        });
    }

    #[test]
    fn search_is_validated_by_validation_layer() {
        with_ports(|ports: &SessionPorts<'_>, log: &LogBuffer| {
            assert_eq!(ports.validate_search("x"), Ok(SearchText("x".to_string())));
            assert_eq!(
                ports.validate_search(""),
                Err("search text is empty".to_string())
            );
            let entries: Vec<LogEntry> = log.drain();
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].layer, Layer::Validation);
            assert_eq!(entries[0].level, LogLevel::Error);
        });
    }
}
