//! A running application: state, git and log, driven by terminal input.

use logic::app::App;

use crate::convert;
use crate::log::LogBuffer;
use crate::ports::SessionPorts;

/// The application connected to a repository.
pub struct Session {
    app: App,
    git: git::Git,
    log: LogBuffer,
}

impl Session {
    /// Starts the application for the repository `git` runs in. Entries already in `log` are
    /// moved into the application log.
    #[must_use]
    pub fn new(git: git::Git, log: LogBuffer) -> Self {
        let mut session: Session = Session {
            app: App::new(),
            git,
            log,
        };
        session.handle(logic::event::Event::Reload);
        session
    }

    /// Handles the bytes of one read from the terminal.
    pub fn input(&mut self, bytes: &[u8]) {
        let log_ui = |level: ui::log::LogLevel, message: &str| self.log.ui(level, message);
        let keys: Vec<ui::key::Key> = ui::key::decode(bytes, &log_ui);
        for key in keys {
            let view: ui::event::ViewId = convert::ui_view(self.app.view());
            let mode: ui::event::InputMode = convert::input_mode(self.app.mode());
            if let Some(event) = ui::event::event_for(key, view, mode) {
                self.handle(convert::event(event));
            }
        }
        self.collect_log();
    }

    fn handle(&mut self, event: logic::event::Event) {
        let ports: SessionPorts<'_> = SessionPorts {
            log: &self.log,
            git: &self.git,
        };
        self.app.handle(event, &ports);
        self.collect_log();
    }

    /// Adapts the list sizes to a terminal of `size`.
    pub fn resize(&mut self, size: ui::layout::Size) {
        self.app.set_viewports(convert::viewports(size));
    }

    /// Draws the current state.
    #[must_use]
    pub fn render(&self, size: ui::layout::Size) -> ui::buffer::Buffer {
        ui::screen::render(&convert::screen(&self.app), size)
    }

    /// Whether the user confirmed quitting.
    #[must_use]
    pub fn should_quit(&self) -> bool {
        self.app.should_quit()
    }

    fn collect_log(&mut self) {
        for entry in self.log.drain() {
            self.app.push_log(&entry);
        }
    }
}

#[cfg(test)]
mod tests;
