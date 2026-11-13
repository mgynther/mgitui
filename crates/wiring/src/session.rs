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
mod tests {
    use super::Session;
    use crate::log::LogBuffer;
    use crate::test_support::{TempDir, fixed_clock, init_repo};
    use ui::layout::Size;

    const SIZE: Size = Size {
        width: 80,
        height: 24,
    };

    fn session_in(dir: &TempDir) -> Session {
        let log: LogBuffer = LogBuffer::new(fixed_clock);
        log.wiring(logic::log::LogLevel::Info, "started");
        let mut session: Session = Session::new(git::Git::new(dir.path()), log);
        session.resize(SIZE);
        session
    }

    fn session() -> (TempDir, Session) {
        let dir: TempDir = TempDir::new();
        init_repo(dir.path());
        let session: Session = session_in(&dir);
        (dir, session)
    }

    #[test]
    fn shows_tabs_and_quits_after_confirmation() {
        let (_dir, mut s): (TempDir, Session) = session();
        assert!(s.render(SIZE).row_text(0).starts_with(" F1 Branches "));
        s.input(b"q");
        assert_eq!(s.render(SIZE).row_text(23).trim_end(), "Quit? y/n");
        s.input(b"n");
        assert!(!s.should_quit());
        s.input(b"qy");
        assert!(s.should_quit());
    }

    #[test]
    fn function_keys_switch_views_and_unknown_keys_are_logged() {
        let (_dir, mut s): (TempDir, Session) = session();
        s.input(b"\x1bOR\x1b[Z");
        let screen: ui::buffer::Buffer = s.render(SIZE);
        assert!(screen.row_text(1).starts_with("─ Log ─"));
        s.input(b"x\x1bOQ");
        assert!(s.render(SIZE).row_text(1).starts_with("─ Unstaged ─"));
    }

    #[test]
    fn diff_view_shows_changes_and_reloads() {
        let (dir, mut s): (TempDir, Session) = session();
        s.input(b"\x1bOQ");
        assert!(s.render(SIZE).row_text(2).contains("No changes"));
        std::fs::write(dir.path().join("new.txt"), "hello\n").unwrap();
        s.input(b"r");
        let screen: ui::buffer::Buffer = s.render(SIZE);
        assert!(screen.row_text(1).contains("Diff new.txt"));
        assert!(screen.row_text(2).starts_with("? new.txt"));
        assert!(screen.row_text(3).contains("+hello"));
        assert_eq!(screen.cell(15, 0).unwrap().style.fg, ui::style::Color::Red);
        assert!(screen.cell(0, 2).unwrap().style.reverse);
        s.input(b"s");
        let staged: String = s.render(SIZE).row_text(1);
        assert!(staged.contains(" Diff ") && !staged.contains("new.txt"));
        s.input(b"sw");
        assert!(s.render(SIZE).row_text(2).contains("▶@@ -0,0 +1 @@"));
    }

    #[test]
    fn log_view_search_with_validation() {
        let (_dir, mut s): (TempDir, Session) = session();
        s.input(b"\x1bOR/");
        assert_eq!(s.render(SIZE).row_text(23).trim_end(), "/");
        s.input(b"\r");
        let screen: ui::buffer::Buffer = s.render(SIZE);
        assert_eq!(screen.row_text(23).trim_end(), "search text is empty");
        assert!(screen.row_text(2).contains("wiring     Info  started"));
        assert!(screen.row_text(3).contains("logic      Info  reload"));
        assert!(screen.row_text(4).contains("git        Info  git status"));
        assert!(
            screen
                .row_text(5)
                .contains("validation Error search text is empty")
        );
    }
}
