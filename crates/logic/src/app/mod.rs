//! The application state and how events change it.

mod branches_view;
mod diff_view;
mod log_view;

pub use branches_view::{BranchesPane, BranchesState, CHUNK};
pub use diff_view::{DiffPane, DiffState};

#[cfg(test)]
mod branch_action_tests;
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
use crate::model::ChangedFile;
use crate::ports::{Ports, SearchText};
use crate::search::{Direction, Found};

/// What key presses are currently used for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    /// Keys are commands.
    Normal,
    /// A y/n confirmation is shown.
    Confirm(Confirmation),
    /// Keys are typed into a prompt.
    Prompt(Prompt),
}

/// What a y/n confirmation is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Confirmation {
    /// Quitting the application.
    Quit,
    /// Deleting a local branch, also when it is not merged.
    DeleteBranch {
        /// The branch name.
        name: String,
    },
    /// Resetting the checked out branch to a commit.
    ResetBranch {
        /// The checked out branch.
        branch: String,
        /// The commit hash.
        hash: String,
    },
    /// Discarding the unstaged changes of a file, or deleting it when untracked.
    DiscardFile {
        /// The file in the unstaged list.
        file: ChangedFile,
    },
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
    /// The name of a new branch at a commit.
    BranchName {
        /// The commit hash.
        at: String,
    },
    /// The number of the branch to check out.
    CheckoutChoice {
        /// The local branches to choose from.
        branches: Vec<String>,
    },
    /// The number of the branch to delete.
    DeleteBranch {
        /// The local branches that can be deleted.
        branches: Vec<String>,
    },
    /// The number of the branch to rebase.
    RebaseBranch {
        /// The local branches of the selected commit.
        branches: Vec<String>,
        /// Whether the rebase is interactive.
        interactive: bool,
    },
    /// The number of the branch to rebase onto.
    RebaseOnto {
        /// The branch being rebased.
        branch: String,
        /// The branches to choose from.
        onto: Vec<String>,
        /// Whether the rebase is interactive.
        interactive: bool,
    },
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
    /// Rows of the file list in the Branches view.
    pub branches_files: usize,
    /// Rows of the file diff in the Branches view.
    pub branches_diff: usize,
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
            Mode::Confirm(confirmation) => {
                let confirmation: Confirmation = confirmation.clone();
                self.mode = Mode::Normal;
                if event == Event::Yes {
                    match confirmation {
                        Confirmation::Quit => self.quit = true,
                        Confirmation::DeleteBranch { name } => self.delete_branch(&name, ports),
                        Confirmation::ResetBranch { branch, hash } => {
                            self.reset_branch(&branch, &hash, ports);
                        }
                        Confirmation::DiscardFile { file } => self.discard_file(&file, ports),
                    }
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
            Event::Quit => self.mode = Mode::Confirm(Confirmation::Quit),
            Event::SwitchView(view) => self.switch_view(view, ports),
            Event::Reload => self.reload(ports),
            Event::StartSearch => self.open_prompt(PromptKind::Search),
            Event::SearchNext => self.search_again(Direction::Forward, ports),
            Event::SearchPrevious => self.search_again(Direction::Backward, ports),
            _ => match self.view {
                ViewId::Log => self.handle_log(event),
                ViewId::Diff => self.handle_diff(event, ports),
                ViewId::Branches => match event {
                    Event::CreateBranch => self.start_create_branch(),
                    Event::Checkout => self.start_checkout(ports),
                    Event::CherryPick => self.cherry_pick(ports),
                    Event::RemoteUpdate => self.remote_update(ports),
                    Event::FetchAllPrune => self.fetch_all_prune(ports),
                    Event::ShowCommit => self.show_commit(ports),
                    Event::ResetBranch => self.start_reset_branch(ports),
                    Event::DeleteBranch => self.start_delete_branch(ports),
                    Event::Rebase => self.start_rebase(false, ports),
                    Event::InteractiveRebase => self.start_rebase(true, ports),
                    _ => self.handle_branches(event, ports),
                },
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
        match &prompt.kind {
            PromptKind::BranchName { at } => self.create_branch(&prompt.text, at, ports),
            PromptKind::CheckoutChoice { branches } => {
                if let Some(branch) = self.choose(prompt, branches, ports) {
                    self.checkout(&branch, ports);
                }
            }
            PromptKind::DeleteBranch { branches } => {
                if let Some(branch) = self.choose(prompt, branches, ports) {
                    self.confirm_delete_branch(branch);
                }
            }
            PromptKind::RebaseBranch {
                branches,
                interactive,
            } => {
                if let Some(branch) = self.choose(prompt, branches, ports) {
                    self.ask_rebase_onto(branch, *interactive);
                }
            }
            PromptKind::RebaseOnto {
                branch,
                onto,
                interactive,
            } => {
                if let Some(target) = self.choose(prompt, onto, ports) {
                    self.rebase(branch, &target, *interactive, ports);
                }
            }
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

    /// Shows `view`, reloading from git first unless it is the Log view, which shows no git
    /// data. Reloading also when `view` is already shown makes F1 and F2 work like r.
    fn switch_view(&mut self, view: ViewId, ports: &dyn Ports) {
        if self.view == ViewId::Log && view != ViewId::Log {
            self.log.mark_seen();
        }
        self.view = view;
        if view != ViewId::Log {
            ports.log(LogLevel::Info, "reload on view switch");
            self.refresh(ports);
        }
    }
}
