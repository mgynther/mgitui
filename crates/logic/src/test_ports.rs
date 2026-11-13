//! A test implementation of [`Ports`] that records calls.

use std::cell::RefCell;

use crate::log::LogLevel;
use crate::ports::{Ports, SearchText};

/// Records log messages as `Level: message` and answers with simple rules.
pub struct TestPorts {
    pub logged: RefCell<Vec<String>>,
}

impl TestPorts {
    pub fn new() -> Self {
        TestPorts {
            logged: RefCell::new(Vec::new()),
        }
    }
}

impl Ports for TestPorts {
    fn log(&self, level: LogLevel, message: &str) {
        self.logged
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
}
