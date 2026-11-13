//! A test implementation of [`Ports`] that records calls and returns prepared answers.

use std::cell::RefCell;

use crate::log::LogLevel;
use crate::model::{ChangeKind, ChangedFile, DiffLine, FileDiff, Hunk, LineKind, Side, WorkStatus};
use crate::ports::{Failed, Ports, SearchText};

/// Records calls as text and answers from its fields.
pub struct TestPorts {
    /// Calls and log messages, in order.
    pub calls: RefCell<Vec<String>>,
    /// The answer to `status`; `None` fails.
    pub status: RefCell<Option<WorkStatus>>,
    /// Whether `file_diff` fails.
    pub diff_fails: bool,
}

impl TestPorts {
    pub fn new() -> Self {
        TestPorts {
            calls: RefCell::new(Vec::new()),
            status: RefCell::new(Some(WorkStatus::default())),
            diff_fails: false,
        }
    }

    /// Ports whose status has the given unstaged and staged paths, all modified.
    pub fn with_files(unstaged: &[&str], staged: &[&str]) -> Self {
        let ports: TestPorts = TestPorts::new();
        *ports.status.borrow_mut() = Some(WorkStatus {
            unstaged: unstaged.iter().map(|p: &&str| modified(p)).collect(),
            staged: staged.iter().map(|p: &&str| modified(p)).collect(),
        });
        ports
    }

    pub fn calls(&self) -> Vec<String> {
        self.calls.borrow().clone()
    }

    pub fn clear(&self) {
        self.calls.borrow_mut().clear();
    }
}

pub fn modified(path: &str) -> ChangedFile {
    ChangedFile {
        path: path.to_string(),
        original_path: None,
        kind: ChangeKind::Modified,
        mode_changed: false,
    }
}

/// A diff with one hunk whose lines are `text` split at `|`, all added.
pub fn diff_of(text: &str) -> FileDiff {
    FileDiff {
        header: vec!["diff --git a/f b/f".to_string()],
        hunks: vec![Hunk {
            header: "@@ -1 +1 @@".to_string(),
            old_start: 1,
            old_count: 0,
            new_start: 1,
            new_count: 1,
            lines: text
                .split('|')
                .map(|t: &str| DiffLine {
                    kind: LineKind::Added,
                    text: t.to_string(),
                })
                .collect(),
        }],
        binary: false,
    }
}

impl Ports for TestPorts {
    fn log(&self, level: LogLevel, message: &str) {
        self.calls
            .borrow_mut()
            .push(format!("{level:?}: {message}"));
    }

    fn validate_search(&self, input: &str) -> Result<SearchText, String> {
        if input.is_empty() {
            Err("search text is empty".to_string())
        } else {
            Ok(SearchText(input.to_string()))
        }
    }

    fn status(&self) -> Result<WorkStatus, Failed> {
        self.calls.borrow_mut().push("status".to_string());
        self.status.borrow().clone().ok_or(Failed)
    }

    fn file_diff(&self, file: &ChangedFile, side: Side) -> Result<FileDiff, Failed> {
        self.calls
            .borrow_mut()
            .push(format!("diff {side:?} {}", file.path));
        if self.diff_fails {
            Err(Failed)
        } else {
            Ok(diff_of(&file.path))
        }
    }
}
