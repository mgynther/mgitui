//! The application log: entries from every layer, kept in memory.

use std::collections::VecDeque;

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

impl LogLevel {
    /// Whether this level turns the Log tab red.
    #[must_use]
    pub fn is_problem(self) -> bool {
        matches!(self, LogLevel::Error | LogLevel::Warn)
    }
}

/// The layer a log entry comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    /// The git layer.
    Git,
    /// The logic layer.
    Logic,
    /// The ui layer.
    Ui,
    /// The validation layer.
    Validation,
    /// The wiring layer.
    Wiring,
}

/// The logging function the logic layer uses.
pub type Log<'a> = dyn Fn(LogLevel, &str) + 'a;

/// One line in the log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    /// Seconds since 1970-01-01 00:00 UTC.
    pub time: u64,
    /// The layer that logged the entry.
    pub layer: Layer,
    /// The severity.
    pub level: LogLevel,
    /// The message, a single line.
    pub message: String,
}

/// The default number of lines kept in the log.
pub const DEFAULT_CAPACITY: usize = 10_000;

/// The log of the application, with a limited number of lines.
#[derive(Debug, Clone)]
pub struct LogStore {
    entries: VecDeque<LogEntry>,
    capacity: usize,
    unseen_problems: bool,
}

impl LogStore {
    /// Creates an empty log keeping at most `capacity` lines.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        LogStore {
            entries: VecDeque::new(),
            capacity,
            unseen_problems: false,
        }
    }

    /// Adds an entry. Multi-line messages become one entry per line. A carriage return also
    /// starts a line: git overwrites progress lines with it on a terminal, and here every
    /// state of the progress is kept, as in a log file. When the log is full, the oldest
    /// lines are dropped. Warnings and errors added while the log is not being viewed are
    /// remembered as unseen.
    pub fn push(&mut self, entry: &LogEntry, viewing: bool) {
        if entry.level.is_problem() && !viewing {
            self.unseen_problems = true;
        }
        for line in entry.message.lines() {
            // Git writes `\r\r` before its last progress line; empty pieces add nothing.
            let pieces: Vec<&str> = line
                .split('\r')
                .filter(|piece: &&str| !piece.is_empty())
                .collect();
            let pieces: Vec<&str> = if pieces.is_empty() { vec![""] } else { pieces };
            for piece in pieces {
                self.entries.push_back(LogEntry {
                    message: piece.to_string(),
                    ..entry.clone()
                });
            }
        }
        if entry.message.is_empty() {
            self.entries.push_back(entry.clone());
        }
        while self.entries.len() > self.capacity {
            self.entries.pop_front();
        }
    }

    /// All lines, oldest first.
    #[must_use]
    pub fn entries(&self) -> &VecDeque<LogEntry> {
        &self.entries
    }

    /// Whether warnings or errors were added since the log was last viewed.
    #[must_use]
    pub fn has_unseen_problems(&self) -> bool {
        self.unseen_problems
    }

    /// Marks all warnings and errors as seen.
    pub fn mark_seen(&mut self) {
        self.unseen_problems = false;
    }
}

#[cfg(test)]
mod tests {
    use super::{Layer, LogEntry, LogLevel, LogStore};

    fn entry(level: LogLevel, message: &str) -> LogEntry {
        LogEntry {
            time: 7,
            layer: Layer::Git,
            level,
            message: message.to_string(),
        }
    }

    fn messages(store: &LogStore) -> Vec<String> {
        store
            .entries()
            .iter()
            .map(|e: &LogEntry| e.message.clone())
            .collect()
    }

    #[test]
    fn problem_levels() {
        assert!(LogLevel::Error.is_problem());
        assert!(LogLevel::Warn.is_problem());
        assert!(!LogLevel::Info.is_problem());
        assert!(!LogLevel::Debug.is_problem());
    }

    #[test]
    fn multi_line_messages_become_one_entry_per_line() {
        let mut store: LogStore = LogStore::new(10);
        store.push(&entry(LogLevel::Info, "first\nsecond"), false);
        assert_eq!(messages(&store), vec!["first", "second"]);
        assert_eq!(store.entries()[1].time, 7);
        assert_eq!(store.entries()[1].layer, Layer::Git);
    }

    #[test]
    fn carriage_returns_start_lines() {
        let mut store: LogStore = LogStore::new(10);
        store.push(
            &entry(
                LogLevel::Info,
                "Rebasing (1/2)\rRebasing (2/2)\r\r\x1b[KDone.\n\nwindows\r\nend",
            ),
            false,
        );
        assert_eq!(
            messages(&store),
            vec![
                "Rebasing (1/2)",
                "Rebasing (2/2)",
                "\x1b[KDone.",
                "",
                "windows",
                "end"
            ]
        );
    }

    #[test]
    fn empty_message_is_kept_as_one_line() {
        let mut store: LogStore = LogStore::new(10);
        store.push(&entry(LogLevel::Info, ""), false);
        assert_eq!(messages(&store), vec![""]);
    }

    #[test]
    fn oldest_lines_are_dropped_when_full() {
        let mut store: LogStore = LogStore::new(2);
        store.push(&entry(LogLevel::Info, "a\nb\nc"), false);
        assert_eq!(messages(&store), vec!["b", "c"]);
    }

    #[test]
    fn unseen_problems_are_tracked() {
        let mut store: LogStore = LogStore::new(10);
        store.push(&entry(LogLevel::Info, "fine"), false);
        assert!(!store.has_unseen_problems());
        store.push(&entry(LogLevel::Warn, "hmm"), true);
        assert!(!store.has_unseen_problems());
        store.push(&entry(LogLevel::Error, "bad"), false);
        assert!(store.has_unseen_problems());
        store.mark_seen();
        assert!(!store.has_unseen_problems());
    }
}
