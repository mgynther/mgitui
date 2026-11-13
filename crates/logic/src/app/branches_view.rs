//! The Branches view: the commit graph of all branches.

use std::collections::HashMap;

use super::{App, Mode, Note, Prompt, PromptKind};
use crate::cursor::Cursor;
use crate::event::Event;
use crate::graph::{GraphBuilder, GraphRow};
use crate::log::LogLevel;
use crate::model::{ChangedFile, Commit, DiffRow, FileDiff, Label, Refs, diff_rows};
use crate::ports::{BranchName, Failed, Ports};
use crate::search::{self, Direction, Found};

/// Commits are loaded in chunks of this size.
pub const CHUNK: usize = 2000;

/// More commits are loaded when the selection gets this close to the last loaded commit.
const LOAD_MARGIN: usize = 100;

/// Which pane of the Branches view has focus.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BranchesPane {
    /// The commit graph.
    #[default]
    Graph,
    /// The files of the selected commit.
    Files,
    /// The diff of the selected file.
    FileDiff,
}

/// The state of the Branches view.
#[derive(Debug, Clone, Default)]
pub struct BranchesState {
    pane: BranchesPane,
    details_for: Option<String>,
    message: String,
    files: Vec<ChangedFile>,
    file_cursor: Cursor,
    diff_rows: Vec<DiffRow>,
    diff_cursor: Cursor,
    commits: Vec<Commit>,
    graph: GraphBuilder,
    rows: Vec<GraphRow>,
    all_loaded: bool,
    labels: HashMap<String, Vec<Label>>,
    head: Option<String>,
    cursor: Cursor,
}

impl BranchesState {
    /// The loaded commits, newest first.
    #[must_use]
    pub fn commits(&self) -> &[Commit] {
        &self.commits
    }

    /// The graph row of each loaded commit.
    #[must_use]
    pub fn rows(&self) -> &[GraphRow] {
        &self.rows
    }

    /// Whether every commit is loaded.
    #[must_use]
    pub fn all_loaded(&self) -> bool {
        self.all_loaded
    }

    /// The labels of a commit.
    #[must_use]
    pub fn labels(&self, hash: &str) -> &[Label] {
        self.labels.get(hash).map_or(&[], Vec::as_slice)
    }

    /// The commit `HEAD` points to.
    #[must_use]
    pub fn head(&self) -> Option<&str> {
        self.head.as_deref()
    }

    /// The selection in the commit list.
    #[must_use]
    pub fn cursor(&self) -> Cursor {
        self.cursor
    }

    /// The selected commit.
    #[must_use]
    pub fn selected(&self) -> Option<&Commit> {
        self.commits.get(self.cursor.selected())
    }

    /// The focused pane.
    #[must_use]
    pub fn pane(&self) -> BranchesPane {
        self.pane
    }

    /// The full message of the selected commit.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// The files changed by the selected commit.
    #[must_use]
    pub fn files(&self) -> &[ChangedFile] {
        &self.files
    }

    /// The selection in the file list.
    #[must_use]
    pub fn file_cursor(&self) -> Cursor {
        self.file_cursor
    }

    /// The selected file.
    #[must_use]
    pub fn selected_file(&self) -> Option<&ChangedFile> {
        self.files.get(self.file_cursor.selected())
    }

    /// The rows of the selected file's diff.
    #[must_use]
    pub fn diff_rows(&self) -> &[DiffRow] {
        &self.diff_rows
    }

    /// The cursor in the file diff.
    #[must_use]
    pub fn diff_cursor(&self) -> Cursor {
        self.diff_cursor
    }
}

impl App {
    /// The state of the Branches view.
    #[must_use]
    pub fn branches(&self) -> &BranchesState {
        &self.branches
    }

    /// Reloads refs and as many commits as were loaded before, keeping the selected commit
    /// when it still exists.
    pub(super) fn reload_branches(&mut self, ports: &dyn Ports) {
        let selected: Option<String> = self.branches.selected().map(|c: &Commit| c.hash.clone());
        let count: usize = self.branches.commits.len().max(CHUNK);
        let (Ok(refs), Ok(commits)) = (ports.refs(), ports.commits(0, count)) else {
            self.note = Note::Problem("Reloading commits failed, see the Log view".to_string());
            return;
        };
        let refs: Refs = refs;
        self.branches.labels = HashMap::new();
        for (hash, label) in refs.labels {
            self.branches.labels.entry(hash).or_default().push(label);
        }
        self.branches.head = refs.head;
        self.branches.all_loaded = commits.len() < count;
        self.branches.graph = GraphBuilder::default();
        self.branches.rows = Vec::new();
        self.branches.commits = Vec::new();
        self.add_commits(commits);
        let index: usize = selected
            .and_then(|hash: String| {
                self.branches
                    .commits
                    .iter()
                    .position(|c: &Commit| c.hash == hash)
            })
            .unwrap_or(0);
        let len: usize = self.branches.commits.len();
        self.branches
            .cursor
            .select(index, len, self.viewports.branches_graph);
        self.load_details(ports);
    }

    /// Loads the message and files of the selected commit, unless they are already loaded.
    fn load_details(&mut self, ports: &dyn Ports) {
        let hash: Option<String> = self.branches.selected().map(|c: &Commit| c.hash.clone());
        if hash == self.branches.details_for {
            return;
        }
        self.branches.details_for.clone_from(&hash);
        self.branches.file_cursor = Cursor::default();
        let details: Result<(String, Vec<ChangedFile>), Failed> = match &hash {
            None => Ok((String::new(), Vec::new())),
            Some(hash) => ports
                .commit_message(hash)
                .and_then(|message: String| ports.commit_files(hash).map(|files| (message, files))),
        };
        let (message, files): (String, Vec<ChangedFile>) = details.unwrap_or_else(|Failed| {
            self.note = Note::Problem("Loading the commit failed, see the Log view".to_string());
            (String::new(), Vec::new())
        });
        self.branches.message = message;
        self.branches.files = files;
        self.load_commit_file_diff(ports);
    }

    fn load_commit_file_diff(&mut self, ports: &dyn Ports) {
        self.branches.diff_cursor = Cursor::default();
        let diff: Option<Result<FileDiff, Failed>> = match (
            self.branches.details_for.as_deref(),
            self.branches.selected_file(),
        ) {
            (Some(hash), Some(file)) => Some(ports.commit_file_diff(hash, file)),
            _ => None,
        };
        self.branches.diff_rows = match diff {
            None => Vec::new(),
            Some(Ok(diff)) => diff_rows(&diff),
            Some(Err(Failed)) => {
                self.note = Note::Problem("Loading the diff failed, see the Log view".to_string());
                Vec::new()
            }
        };
    }

    /// Loads the next chunk of commits when the selection is near the last loaded commit.
    fn load_more_if_needed(&mut self, ports: &dyn Ports) {
        let len: usize = self.branches.commits.len();
        if self.branches.all_loaded || self.branches.cursor.selected() + LOAD_MARGIN < len {
            return;
        }
        match ports.commits(len, CHUNK) {
            Ok(more) => {
                self.branches.all_loaded = more.len() < CHUNK;
                self.add_commits(more);
            }
            Err(Failed) => {
                self.note =
                    Note::Problem("Loading more commits failed, see the Log view".to_string());
            }
        }
    }

    fn add_commits(&mut self, commits: Vec<Commit>) {
        for commit in commits {
            let row: GraphRow = self.branches.graph.push(&commit);
            self.branches.rows.push(row);
            self.branches.commits.push(commit);
        }
    }

    pub(super) fn clamp_branch_cursors(&mut self) {
        let commits: usize = self.branches.commits.len();
        self.branches
            .cursor
            .clamp(commits, self.viewports.branches_graph);
        let files: usize = self.branches.files.len();
        self.branches
            .file_cursor
            .clamp(files, self.viewports.branches_files);
        let rows: usize = self.branches.diff_rows.len();
        self.branches
            .diff_cursor
            .clamp(rows, self.viewports.branches_diff);
    }

    pub(super) fn handle_branches(&mut self, event: Event, ports: &dyn Ports) {
        let move_cursor = super::diff_view::move_cursor;
        match (self.branches.pane, event) {
            (_, Event::NextPane) => {
                self.branches.pane = match self.branches.pane {
                    BranchesPane::Graph => BranchesPane::Files,
                    BranchesPane::Files => BranchesPane::FileDiff,
                    BranchesPane::FileDiff => BranchesPane::Graph,
                };
            }
            (BranchesPane::Graph, _) => {
                let len: usize = self.branches.commits.len();
                let height: usize = self.viewports.branches_graph;
                move_cursor(&mut self.branches.cursor, event, len, height);
                self.load_more_if_needed(ports);
                self.load_details(ports);
            }
            (BranchesPane::Files, _) => {
                let before: usize = self.branches.file_cursor.selected();
                let len: usize = self.branches.files.len();
                let height: usize = self.viewports.branches_files;
                move_cursor(&mut self.branches.file_cursor, event, len, height);
                if self.branches.file_cursor.selected() != before {
                    self.load_commit_file_diff(ports);
                }
            }
            (BranchesPane::FileDiff, _) => {
                let len: usize = self.branches.diff_rows.len();
                let height: usize = self.viewports.branches_diff;
                move_cursor(&mut self.branches.diff_cursor, event, len, height);
            }
        }
    }

    pub(super) fn start_create_branch(&mut self) {
        match self.branches.selected() {
            Some(commit) => {
                let at: String = commit.hash.clone();
                self.open_prompt(PromptKind::BranchName { at });
            }
            None => self.note = Note::Info("No commit selected".to_string()),
        }
    }

    pub(super) fn create_branch(&mut self, input: &str, at: &str, ports: &dyn Ports) {
        let name: BranchName = match ports.validate_branch_name(input) {
            Ok(name) => name,
            Err(problem) => {
                self.note = Note::Problem(problem);
                return;
            }
        };
        let short: &str = at.get(..7).unwrap_or(at);
        ports.log(
            LogLevel::Info,
            &format!("create branch {} at {short}", name.0),
        );
        let result: Result<(), Failed> = ports.create_branch(&name, at);
        self.refresh(ports);
        self.note = match result {
            Ok(()) => Note::Info(format!("Created branch {}", name.0)),
            Err(Failed) => {
                Note::Problem("Creating the branch failed, see the Log view".to_string())
            }
        };
    }

    /// The local branches of the selected commit.
    fn local_branches(&self) -> Vec<String> {
        self.branches
            .selected()
            .map_or(&[][..], |c: &Commit| self.branches.labels(&c.hash))
            .iter()
            .filter_map(|label: &Label| match label {
                Label::Local { name, .. } => Some(name.clone()),
                Label::Remote { .. } | Label::Tag { .. } => None,
            })
            .collect()
    }

    pub(super) fn start_checkout(&mut self, ports: &dyn Ports) {
        let branches: Vec<String> = self.local_branches();
        match branches.as_slice() {
            [] => {
                let message: &str = "No local branch at this commit to check out";
                ports.log(LogLevel::Error, message);
                self.note = Note::Problem(message.to_string());
            }
            [branch] => self.checkout(&branch.clone(), ports),
            _ => self.open_prompt(PromptKind::CheckoutChoice { branches }),
        }
    }

    pub(super) fn choose_checkout(
        &mut self,
        prompt: &Prompt,
        branches: &[String],
        ports: &dyn Ports,
    ) {
        match ports.validate_checkout_choice(&prompt.text, branches.len()) {
            Ok(index) => match branches.get(index) {
                Some(branch) => self.checkout(&branch.clone(), ports),
                None => self.note = Note::Problem("Choice is not listed".to_string()),
            },
            Err(problem) => {
                self.note = Note::Problem(problem);
                self.mode = Mode::Prompt(prompt.clone());
            }
        }
    }

    fn checkout(&mut self, branch: &str, ports: &dyn Ports) {
        ports.log(LogLevel::Info, &format!("check out {branch}"));
        let result: Result<(), Failed> = ports.checkout(branch);
        self.refresh(ports);
        self.note = match result {
            Ok(()) => Note::Info(format!("Checked out {branch}")),
            Err(Failed) => Note::Problem("Checkout failed, see the Log view".to_string()),
        };
    }

    pub(super) fn search_branches(
        &mut self,
        term: &str,
        direction: Direction,
        ports: &dyn Ports,
    ) -> Option<Found> {
        let commits: &[Commit] = &self.branches.commits;
        let found: Option<Found> = search::find(
            commits.len(),
            self.branches.cursor.selected(),
            direction,
            |i: usize| {
                commits.get(i).is_some_and(|c: &Commit| {
                    search::text_matches(&c.subject, term)
                        || search::text_matches(&c.hash, term)
                        || search::text_matches(&c.author, term)
                })
            },
        );
        if let Some(Found { index, .. }) = found {
            let len: usize = commits.len();
            self.branches
                .cursor
                .select(index, len, self.viewports.branches_graph);
            self.branches.pane = BranchesPane::Graph;
            self.load_more_if_needed(ports);
            self.load_details(ports);
        }
        found
    }
}
