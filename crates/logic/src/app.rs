//! The application state and how events change it.

use crate::event::{Event, ViewId};
use crate::log::{DEFAULT_CAPACITY, LogEntry, LogStore};
use crate::ports::Ports;

/// What key presses are currently used for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    /// Keys are commands.
    Normal,
    /// The quit prompt is shown.
    QuitConfirm,
}

/// The whole application state.
#[derive(Debug, Clone)]
pub struct App {
    view: ViewId,
    mode: Mode,
    quit: bool,
    log: LogStore,
}

impl Default for App {
    fn default() -> Self {
        App::new()
    }
}

impl App {
    /// The state at startup: the Branches view is shown.
    #[must_use]
    pub fn new() -> Self {
        App {
            view: ViewId::Branches,
            mode: Mode::Normal,
            quit: false,
            log: LogStore::new(DEFAULT_CAPACITY),
        }
    }

    /// The active view.
    #[must_use]
    pub fn view(&self) -> ViewId {
        self.view
    }

    /// The input mode.
    #[must_use]
    pub fn mode(&self) -> &Mode {
        &self.mode
    }

    /// Whether the user confirmed quitting.
    #[must_use]
    pub fn should_quit(&self) -> bool {
        self.quit
    }

    /// The application log.
    #[must_use]
    pub fn log(&self) -> &LogStore {
        &self.log
    }

    /// Adds an entry to the application log.
    pub fn push_log(&mut self, entry: &LogEntry) {
        self.log.push(entry, self.view == ViewId::Log);
    }

    /// Changes the state for `event`, using `ports` for everything outside the state.
    pub fn handle(&mut self, event: Event, ports: &dyn Ports) {
        let _ = ports;
        match (&self.mode, event) {
            (Mode::QuitConfirm, Event::ConfirmQuit) => self.quit = true,
            (Mode::QuitConfirm, _) => self.mode = Mode::Normal,
            (Mode::Normal, Event::Quit) => self.mode = Mode::QuitConfirm,
            (Mode::Normal, Event::SwitchView(view)) => self.switch_view(view),
            (Mode::Normal, _) => {}
        }
    }

    fn switch_view(&mut self, view: ViewId) {
        if self.view == ViewId::Log && view != ViewId::Log {
            self.log.mark_seen();
        }
        self.view = view;
    }
}

#[cfg(test)]
mod tests {
    use super::{App, Mode};
    use crate::event::{Event, ViewId};
    use crate::log::{Layer, LogEntry, LogLevel};
    use crate::test_ports::TestPorts;

    fn error() -> LogEntry {
        LogEntry {
            time: 0,
            layer: Layer::Git,
            level: LogLevel::Error,
            message: "bad".to_string(),
        }
    }

    #[test]
    fn starts_in_branches_view() {
        let app: App = App::default();
        assert_eq!(app.view(), ViewId::Branches);
        assert_eq!(app.mode(), &Mode::Normal);
        assert!(!app.should_quit());
    }

    #[test]
    fn quit_needs_confirmation() {
        let ports: TestPorts = TestPorts::new();
        let mut app: App = App::new();
        app.handle(Event::Quit, &ports);
        assert_eq!(app.mode(), &Mode::QuitConfirm);
        app.handle(Event::CancelQuit, &ports);
        assert_eq!(app.mode(), &Mode::Normal);
        assert!(!app.should_quit());
        app.handle(Event::Quit, &ports);
        app.handle(Event::ConfirmQuit, &ports);
        assert!(app.should_quit());
    }

    #[test]
    fn switching_views_and_unseen_problems() {
        let ports: TestPorts = TestPorts::new();
        let mut app: App = App::new();
        app.push_log(&error());
        assert!(app.log().has_unseen_problems());
        app.handle(Event::SwitchView(ViewId::Log), &ports);
        assert_eq!(app.view(), ViewId::Log);
        assert!(app.log().has_unseen_problems());
        app.push_log(&error());
        app.handle(Event::SwitchView(ViewId::Log), &ports);
        assert!(app.log().has_unseen_problems());
        app.handle(Event::SwitchView(ViewId::Diff), &ports);
        assert!(!app.log().has_unseen_problems());
        app.handle(Event::SwitchView(ViewId::Branches), &ports);
        assert_eq!(app.view(), ViewId::Branches);
        assert_eq!(app.log().entries().len(), 2);
    }

    #[test]
    fn unknown_events_change_nothing() {
        let ports: TestPorts = TestPorts::new();
        let mut app: App = App::new();
        app.handle(Event::MoveDown, &ports);
        assert_eq!(app.view(), ViewId::Branches);
        assert_eq!(app.mode(), &Mode::Normal);
        ports.log(LogLevel::Info, "x");
        assert_eq!(ports.logged.borrow().clone(), vec!["Info: x".to_string()]);
    }

    use crate::ports::Ports;
}
