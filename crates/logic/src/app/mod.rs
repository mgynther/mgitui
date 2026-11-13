//! The application state and how events change it.

mod branches_view;
mod diff_view;
mod log_view;

pub use branches_view::{BranchesState, CHUNK};
pub use diff_view::{DiffPane, DiffState};

#[cfg(test)]
mod branches_tests;
#[cfg(test)]
mod commit_tests;
#[cfg(test)]
mod diff_tests;
#[cfg(test)]
mod stage_tests;
#[cfg(test)]
mod tests;

use crate::cursor::Cursor;
use crate::event::{Event, ViewId};
use crate::log::{DEFAULT_CAPACITY, LogLevel, LogStore};
use crate::ports::{Ports, SearchText};
use crate::search::{Direction, Found};

/// What key presses are currently used for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    /// Keys are commands.
    Normal,
    /// The quit prompt is shown.
    QuitConfirm,
    /// Keys are typed into a prompt.
    Prompt(Prompt),
}

/// A text prompt in the status line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prompt {
    /// What the prompt is for.
    pub kind: PromptKind,
    /// The text typed so far.
    pub text: String,
}

/// What a prompt is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptKind {
    /// A search in the active view.
    Search,
}

/// A short message for the status line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Note {
    /// Nothing to show.
    None,
    /// Information.
    Info(String),
    /// A problem.
    Problem(String),
}

/// How many rows each list is shown in. Logic needs these to scroll.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Viewports {
    /// Rows of the commit graph.
    pub branches_graph: usize,
    /// Rows of the log list.
    pub log: usize,
    /// Rows of the unstaged file list.
    pub diff_unstaged: usize,
    /// Rows of the staged file list.
    pub diff_staged: usize,
    /// Rows of the file diff in the Diff view.
    pub diff_file: usize,
}

/// The last search term of each view.
#[derive(Debug, Default, Clone)]
struct SearchTerms {
    branches: Option<SearchText>,
    diff: Option<SearchText>,
    log: Option<SearchText>,
}

impl SearchTerms {
    fn get(&self, view: ViewId) -> Option<&SearchText> {
        match view {
            ViewId::Branches => self.branches.as_ref(),
            ViewId::Diff => self.diff.as_ref(),
            ViewId::Log => self.log.as_ref(),
        }
    }

    fn set(&mut self, view: ViewId, term: SearchText) {
        let slot: &mut Option<SearchText> = match view {
            ViewId::Branches => &mut self.branches,
            ViewId::Diff => &mut self.diff,
            ViewId::Log => &mut self.log,
        };
        *slot = Some(term);
    }
}

/// The whole application state.
#[derive(Debug, Clone)]
pub struct App {
    view: ViewId,
    mode: Mode,
    note: Note,
    quit: bool,
    log: LogStore,
    log_cursor: Cursor,
    branches: BranchesState,
    diff: DiffState,
    terms: SearchTerms,
    viewports: Viewports,
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
            note: Note::None,
            quit: false,
            log: LogStore::new(DEFAULT_CAPACITY),
            log_cursor: Cursor::default(),
            branches: BranchesState::default(),
            diff: DiffState::default(),
            terms: SearchTerms::default(),
            viewports: Viewports::default(),
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

    /// The note for the status line.
    #[must_use]
    pub fn note(&self) -> &Note {
        &self.note
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

    /// The selection in the log list.
    #[must_use]
    pub fn log_cursor(&self) -> Cursor {
        self.log_cursor
    }

    /// How many rows each list is shown in.
    #[must_use]
    pub fn viewports(&self) -> Viewports {
        self.viewports
    }

    /// Sets how many rows each list is shown in, after the terminal size changed.
    pub fn set_viewports(&mut self, viewports: Viewports) {
        self.viewports = viewports;
        self.log_cursor
            .clamp(self.log.entries().len(), viewports.log);
        self.clamp_diff_cursors();
        self.clamp_branch_cursors();
    }

    /// Changes the state for `event`, using `ports` for everything outside the state.
    pub fn handle(&mut self, event: Event, ports: &dyn Ports) {
        self.note = Note::None;
        match &mut self.mode {
            Mode::QuitConfirm => {
                if event == Event::ConfirmQuit {
                    self.quit = true;
                } else {
                    self.mode = Mode::Normal;
                }
            }
            Mode::Prompt(prompt) => match event {
                Event::Input(c) => prompt.text.push(c),
                Event::InputBackspace => {
                    prompt.text.pop();
                }
                Event::InputCancel => self.mode = Mode::Normal,
                Event::InputSubmit => {
                    let submitted: Prompt = prompt.clone();
                    self.mode = Mode::Normal;
                    self.submit(&submitted, ports);
                }
                _ => {}
            },
            Mode::Normal => self.handle_normal(event, ports),
        }
    }

    fn handle_normal(&mut self, event: Event, ports: &dyn Ports) {
        match event {
            Event::Quit => self.mode = Mode::QuitConfirm,
            Event::SwitchView(view) => self.switch_view(view),
            Event::Reload => self.reload(ports),
            Event::StartSearch => self.open_prompt(PromptKind::Search),
            Event::SearchNext => self.search_again(Direction::Forward, ports),
            Event::SearchPrevious => self.search_again(Direction::Backward, ports),
            _ => match self.view {
                ViewId::Log => self.handle_log(event),
                ViewId::Diff => self.handle_diff(event, ports),
                ViewId::Branches => self.handle_branches(event, ports),
            },
        }
    }

    /// Reloads everything from git, as asked by the user.
    fn reload(&mut self, ports: &dyn Ports) {
        ports.log(LogLevel::Info, "reload");
        self.refresh(ports);
    }

    /// Reloads everything from git after an action.
    fn refresh(&mut self, ports: &dyn Ports) {
        self.reload_branches(ports);
        self.reload_status(ports);
    }

    fn open_prompt(&mut self, kind: PromptKind) {
        self.mode = Mode::Prompt(Prompt {
            kind,
            text: String::new(),
        });
    }

    fn submit(&mut self, prompt: &Prompt, ports: &dyn Ports) {
        match prompt.kind {
            PromptKind::Search => match ports.validate_search(&prompt.text) {
                Ok(term) => {
                    self.terms.set(self.view, term.clone());
                    self.search(&term, Direction::Forward, ports);
                }
                Err(problem) => self.note = Note::Problem(problem),
            },
        }
    }

    fn search_again(&mut self, direction: Direction, ports: &dyn Ports) {
        match self.terms.get(self.view).cloned() {
            Some(term) => self.search(&term, direction, ports),
            None => self.note = Note::Info("No previous search".to_string()),
        }
    }

    fn search(&mut self, term: &SearchText, direction: Direction, ports: &dyn Ports) {
        let found: Option<Found> = match self.view {
            ViewId::Log => self.search_log(&term.0, direction),
            ViewId::Diff => self.search_diff(&term.0, direction, ports),
            ViewId::Branches => self.search_branches(&term.0, direction, ports),
        };
        self.note = match found {
            None => Note::Info(format!("Pattern not found: {}", term.0)),
            Some(Found { wrapped: true, .. }) => Note::Info("search wrapped".to_string()),
            Some(Found { wrapped: false, .. }) => Note::None,
        };
    }

    fn switch_view(&mut self, view: ViewId) {
        if self.view == ViewId::Log && view != ViewId::Log {
            self.log.mark_seen();
        }
        self.view = view;
    }
}
