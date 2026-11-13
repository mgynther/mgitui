//! The Log view: browsing and searching the application log.

use super::App;
use crate::event::Event;
use crate::log::LogEntry;
use crate::search::{self, Direction, Found};

impl App {
    /// Adds an entry to the application log. When the last line was selected, the selection
    /// follows the new last line.
    pub fn push_log(&mut self, entry: &LogEntry) {
        let len: usize = self.log.entries().len();
        let following: bool = self.log_cursor.selected() + 1 >= len;
        self.log.push(entry, self.view == crate::event::ViewId::Log);
        let new_len: usize = self.log.entries().len();
        if following {
            self.log_cursor.bottom(new_len, self.viewports.log);
        } else {
            self.log_cursor.clamp(new_len, self.viewports.log);
        }
    }

    pub(super) fn handle_log(&mut self, event: Event) {
        let len: usize = self.log.entries().len();
        let height: usize = self.viewports.log;
        match event {
            Event::MoveDown => self.log_cursor.down(len, height),
            Event::MoveUp => self.log_cursor.up(len, height),
            Event::Top => self.log_cursor.top(),
            Event::Bottom => self.log_cursor.bottom(len, height),
            _ => {}
        }
    }

    pub(super) fn search_log(&mut self, term: &str, direction: Direction) -> Option<Found> {
        let entries: &std::collections::VecDeque<LogEntry> = self.log.entries();
        let found: Option<Found> = search::find(
            entries.len(),
            self.log_cursor.selected(),
            direction,
            |i: usize| {
                entries
                    .get(i)
                    .is_some_and(|e: &LogEntry| search::text_matches(&e.message, term))
            },
        );
        if let Some(Found { index, .. }) = found {
            self.log_cursor
                .select(index, self.log.entries().len(), self.viewports.log);
        }
        found
    }
}
