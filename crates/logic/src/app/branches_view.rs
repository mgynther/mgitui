//! The Branches view: the commit graph of all branches.

use std::collections::HashMap;

use super::{App, Confirmation, Mode, Prompt, PromptKind};
use crate::cursor::Cursor;
use crate::event::Event;
use crate::graph::{GraphBuilder, GraphRow};
use crate::log::LogLevel;
use crate::model::{
    ChangedFile, Commit, DiffRow, FileDiff, Label, Refs, Side, WorkStatus, diff_rows,
};
use crate::note::{Info, Note, Operation, Problem};
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

    /// The lane of the selected commit in the graph, whose color its line has.
    #[must_use]
    pub fn selected_lane(&self) -> Option<usize> {
        self.rows
            .get(self.cursor.selected())
            .and_then(GraphRow::commit_lane)
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
            self.note = Note::Problem(Problem::Failed(Operation::ReloadingCommits));
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
            self.note = Note::Problem(Problem::Failed(Operation::LoadingCommit));
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
            (Some(hash), Some(file)) => {
                Some(ports.commit_file_diff(hash, &file.paths(Side::Staged)))
            }
            _ => None,
        };
        self.branches.diff_rows = match diff {
            None => Vec::new(),
            Some(Ok(diff)) => diff_rows(&diff),
            Some(Err(Failed)) => {
                self.note = Note::Problem(Problem::Failed(Operation::LoadingDiff));
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
                self.note = Note::Problem(Problem::Failed(Operation::LoadingMoreCommits));
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
            None => self.note = Note::Info(Info::NoCommitSelected),
        }
    }

    pub(super) fn create_branch(&mut self, input: &str, at: &str, ports: &dyn Ports) {
        let name: BranchName = match ports.validate_branch_name(input) {
            Ok(name) => name,
            Err(problem) => {
                self.note = Note::Problem(Problem::Invalid(problem));
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
            Ok(()) => Note::Info(Info::CreatedBranch {
                name: name.0.clone(),
            }),
            Err(Failed) => Note::Problem(Problem::Failed(Operation::CreatingBranch)),
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
                ports.log(
                    LogLevel::Info,
                    "No local branch at this commit to check out",
                );
                self.note = Note::Problem(Problem::NoLocalBranchToCheckOut);
            }
            [branch] => self.checkout(&branch.clone(), ports),
            _ => self.open_prompt(PromptKind::CheckoutChoice { branches }),
        }
    }

    /// The item chosen in a numbered prompt. An invalid number keeps the prompt open with the
    /// problem shown.
    pub(super) fn choose(
        &mut self,
        prompt: &Prompt,
        items: &[String],
        ports: &dyn Ports,
    ) -> Option<String> {
        match ports.validate_checkout_choice(&prompt.text, items.len()) {
            Ok(index) => {
                let item: Option<String> = items.get(index).cloned();
                if item.is_none() {
                    self.note = Note::Problem(Problem::ChoiceNotListed);
                }
                item
            }
            Err(problem) => {
                self.note = Note::Problem(Problem::Invalid(problem));
                self.mode = Mode::Prompt(prompt.clone());
                None
            }
        }
    }

    pub(super) fn start_delete_branch(&mut self, ports: &dyn Ports) {
        let labels: &[Label] = self
            .branches
            .selected()
            .map_or(&[][..], |c: &Commit| self.branches.labels(&c.hash));
        let checked_out: bool = labels
            .iter()
            .any(|label: &Label| matches!(label, Label::Local { head: true, .. }));
        let branches: Vec<String> = labels
            .iter()
            .filter_map(|label: &Label| match label {
                Label::Local { name, head: false } => Some(name.clone()),
                Label::Local { .. } | Label::Remote { .. } | Label::Tag { .. } => None,
            })
            .collect();
        match branches.as_slice() {
            [] => {
                let (problem, message): (Problem, &str) = if checked_out {
                    (
                        Problem::CheckedOutBranchCannotBeDeleted,
                        "The checked out branch cannot be deleted",
                    )
                } else {
                    (
                        Problem::NoLocalBranchToDelete,
                        "No local branch at this commit to delete",
                    )
                };
                ports.log(LogLevel::Info, message);
                self.note = Note::Problem(problem);
            }
            [branch] => self.confirm_delete_branch(branch.clone()),
            _ => self.open_prompt(PromptKind::DeleteBranch { branches }),
        }
    }

    /// Asks for confirmation before deleting `branch`: it is deleted also when not merged.
    pub(super) fn confirm_delete_branch(&mut self, name: String) {
        self.mode = Mode::Confirm(Confirmation::DeleteBranch { name });
    }

    pub(super) fn delete_branch(&mut self, branch: &str, ports: &dyn Ports) {
        ports.log(LogLevel::Info, &format!("delete branch {branch}"));
        let result: Result<(), Failed> = ports.delete_branch(branch);
        self.refresh(ports);
        self.note = match result {
            Ok(()) => Note::Info(Info::DeletedBranch {
                name: branch.to_string(),
            }),
            Err(Failed) => Note::Problem(Problem::Failed(Operation::DeletingBranch)),
        };
    }

    pub(super) fn start_rebase(&mut self, interactive: bool, ports: &dyn Ports) {
        let branches: Vec<String> = self.local_branches();
        match branches.as_slice() {
            [] => {
                ports.log(LogLevel::Info, "No local branch at this commit to rebase");
                self.note = Note::Problem(Problem::NoLocalBranchToRebase);
            }
            [branch] => self.ask_rebase_onto(branch.clone(), interactive),
            _ => self.open_prompt(PromptKind::RebaseBranch {
                branches,
                interactive,
            }),
        }
    }

    /// Asks which branch to rebase `branch` onto: every other local branch, `main` and
    /// `master` first. A plain rebase can also go onto remote `main` and `master` branches,
    /// listed after the local ones; other remote branches are left out because work
    /// repositories often have many stale ones. An interactive rebase only offers local
    /// branches: rebase onto the remote first, then rewrite history.
    pub(super) fn ask_rebase_onto(&mut self, branch: String, interactive: bool) {
        let mut onto: Vec<(u8, String)> = self
            .branches
            .labels
            .values()
            .flatten()
            .filter_map(|label: &Label| match label {
                Label::Local { name, .. } if *name != branch => {
                    let rank: u8 = match name.as_str() {
                        "main" => 0,
                        "master" => 1,
                        _ => 3,
                    };
                    Some((rank, name.clone()))
                }
                Label::Remote {
                    remote,
                    branch: remote_branch,
                } if !interactive && is_main(remote_branch) => {
                    Some((2, format!("{remote}/{remote_branch}")))
                }
                Label::Local { .. } | Label::Remote { .. } | Label::Tag { .. } => None,
            })
            .collect();
        onto.sort();
        let onto: Vec<String> = onto
            .into_iter()
            .map(|(_, name): (u8, String)| name)
            .collect();
        if onto.is_empty() {
            self.note = Note::Info(Info::NoOtherBranchToRebaseOnto);
        } else {
            self.open_prompt(PromptKind::RebaseOnto {
                branch,
                onto,
                interactive,
            });
        }
    }

    pub(super) fn rebase(
        &mut self,
        branch: &str,
        onto: &str,
        interactive: bool,
        ports: &dyn Ports,
    ) {
        let action: &str = if interactive {
            "interactive rebase"
        } else {
            "rebase"
        };
        ports.log(LogLevel::Info, &format!("{action} {branch} onto {onto}"));
        let result: Result<(), Failed> = if interactive {
            ports.rebase_interactive(branch, onto)
        } else {
            ports.rebase(branch, onto)
        };
        self.refresh(ports);
        self.note = match result {
            Ok(()) if interactive => Note::Info(Info::RebasedInteractively {
                branch: branch.to_string(),
                onto: onto.to_string(),
            }),
            Ok(()) => Note::Info(Info::Rebased {
                branch: branch.to_string(),
                onto: onto.to_string(),
            }),
            Err(Failed) => Note::Problem(Problem::Failed(Operation::Rebasing)),
        };
    }

    pub(super) fn checkout(&mut self, branch: &str, ports: &dyn Ports) {
        ports.log(LogLevel::Info, &format!("check out {branch}"));
        let result: Result<(), Failed> = ports.checkout(branch);
        self.refresh(ports);
        self.note = match result {
            Ok(()) => Note::Info(Info::CheckedOut {
                branch: branch.to_string(),
            }),
            Err(Failed) => Note::Problem(Problem::Failed(Operation::CheckingOut)),
        };
    }

    pub(super) fn remote_update(&mut self, ports: &dyn Ports) {
        ports.log(LogLevel::Info, "remote update");
        let result: Result<(), Failed> = ports.remote_update();
        self.refresh(ports);
        self.note = match result {
            Ok(()) => Note::Info(Info::UpdatedRemotes),
            Err(Failed) => Note::Problem(Problem::Failed(Operation::UpdatingRemotes)),
        };
    }

    pub(super) fn fetch_all_prune(&mut self, ports: &dyn Ports) {
        ports.log(LogLevel::Info, "fetch --all --prune");
        let result: Result<(), Failed> = ports.fetch_all_prune();
        self.refresh(ports);
        self.note = match result {
            Ok(()) => Note::Info(Info::FetchedAndPruned),
            Err(Failed) => Note::Problem(Problem::Failed(Operation::FetchingAndPruning)),
        };
    }

    /// Shows the selected commit with `git show` in the user's pager. Nothing changes, so
    /// nothing is reloaded.
    pub(super) fn show_commit(&mut self, ports: &dyn Ports) {
        let Some(hash) = self.branches.selected().map(|c: &Commit| c.hash.clone()) else {
            self.note = Note::Info(Info::NoCommitSelected);
            return;
        };
        let short: &str = hash.get(..7).unwrap_or(&hash);
        ports.log(LogLevel::Info, &format!("show {short}"));
        let result: Result<(), Failed> = ports.show_commit(&hash);
        if result.is_err() {
            self.note = Note::Problem(Problem::Failed(Operation::ShowingCommit));
        }
    }

    /// The checked out local branch, or `None` when `HEAD` is detached.
    fn checked_out_branch(&self) -> Option<String> {
        self.branches
            .labels
            .values()
            .flatten()
            .find_map(|label: &Label| match label {
                Label::Local { name, head: true } => Some(name.clone()),
                Label::Local { .. } | Label::Remote { .. } | Label::Tag { .. } => None,
            })
    }

    /// Asks for confirmation before resetting the checked out branch to the selected commit.
    /// Refuses when files git tracks have changes, because the reset would lose or mix them
    /// up; untracked files are left to git, which refuses if one is in the way.
    pub(super) fn start_reset_branch(&mut self, ports: &dyn Ports) {
        let Some(hash) = self.branches.selected().map(|c: &Commit| c.hash.clone()) else {
            self.note = Note::Info(Info::NoCommitSelected);
            return;
        };
        let Some(branch) = self.checked_out_branch() else {
            ports.log(LogLevel::Info, "No branch is checked out to reset");
            self.note = Note::Problem(Problem::NoBranchCheckedOutToReset);
            return;
        };
        let status: Result<WorkStatus, Failed> = ports.status();
        match status {
            Ok(status) if status.has_tracked_changes() => {
                ports.log(
                    LogLevel::Info,
                    "Commit or discard uncommitted changes before resetting",
                );
                self.note = Note::Problem(Problem::UncommittedChangesBeforeReset);
            }
            Ok(_) => self.mode = Mode::Confirm(Confirmation::ResetBranch { branch, hash }),
            Err(Failed) => {
                self.note = Note::Problem(Problem::Failed(Operation::ReadingStatus));
            }
        }
    }

    pub(super) fn reset_branch(&mut self, branch: &str, hash: &str, ports: &dyn Ports) {
        let short: &str = hash.get(..7).unwrap_or(hash);
        ports.log(LogLevel::Info, &format!("reset {branch} to {short}"));
        let result: Result<(), Failed> = ports.reset_keep(hash);
        self.refresh(ports);
        self.note = match result {
            Ok(()) => Note::Info(Info::ResetBranch {
                branch: branch.to_string(),
                hash: hash.to_string(),
            }),
            Err(Failed) => Note::Problem(Problem::Failed(Operation::ResettingBranch)),
        };
    }

    pub(super) fn cherry_pick(&mut self, ports: &dyn Ports) {
        let Some(hash) = self.branches.selected().map(|c: &Commit| c.hash.clone()) else {
            self.note = Note::Info(Info::NoCommitSelected);
            return;
        };
        let short: &str = hash.get(..7).unwrap_or(&hash);
        ports.log(LogLevel::Info, &format!("cherry-pick {short}"));
        let result: Result<(), Failed> = ports.cherry_pick(&hash);
        self.refresh(ports);
        self.note = match result {
            Ok(()) => Note::Info(Info::CherryPicked { hash: hash.clone() }),
            Err(Failed) => Note::Problem(Problem::Failed(Operation::CherryPicking)),
        };
    }

    pub(super) fn search_branches(
        &mut self,
        term: &str,
        direction: Direction,
        ports: &dyn Ports,
    ) -> Option<Found> {
        let branches: &BranchesState = &self.branches;
        let found: Option<Found> = search::find(
            branches.commits.len(),
            branches.cursor.selected(),
            direction,
            |i: usize| {
                branches.commits.get(i).is_some_and(|c: &Commit| {
                    search::text_matches(&c.subject, term)
                        || search::text_matches(&c.hash, term)
                        || search::text_matches(&c.author, term)
                        || branches
                            .labels(&c.hash)
                            .iter()
                            .any(|label: &Label| label_matches(label, term))
                })
            },
        );
        if let Some(Found { index, .. }) = found {
            self.select_commit(index, ports);
        }
        found
    }

    /// Selects the next commit with a local or remote branch named exactly `main` or
    /// `master`, wrapping around, to find the main branches among long feature branches.
    pub(super) fn next_main_branch(&mut self, ports: &dyn Ports) {
        let branches: &BranchesState = &self.branches;
        let found: Option<Found> = search::find(
            branches.commits.len(),
            branches.cursor.selected(),
            Direction::Forward,
            |i: usize| {
                branches
                    .commits
                    .get(i)
                    .is_some_and(|c: &Commit| branches.labels(&c.hash).iter().any(is_main_label))
            },
        );
        match found {
            Some(Found { index, .. }) => self.select_commit(index, ports),
            None => self.note = Note::Info(Info::NoMainOrMasterBranch),
        }
    }

    /// Selects commit `index` in the graph and loads what it needs.
    fn select_commit(&mut self, index: usize, ports: &dyn Ports) {
        let len: usize = self.branches.commits.len();
        self.branches
            .cursor
            .select(index, len, self.viewports.branches_graph);
        self.branches.pane = BranchesPane::Graph;
        self.load_more_if_needed(ports);
        self.load_details(ports);
    }
}

/// Whether a branch or tag label contains `term`, ignoring case. A remote branch is matched as
/// `remote/branch`, the way the graph shows it.
fn label_matches(label: &Label, term: &str) -> bool {
    match label {
        Label::Local { name, .. } | Label::Tag { name } => search::text_matches(name, term),
        Label::Remote { remote, branch } => {
            search::text_matches(&format!("{remote}/{branch}"), term)
        }
    }
}

/// Whether a label is a local or remote branch named exactly `main` or `master`. The remote
/// name is ignored, so `origin/main` and `upstream/master` count.
fn is_main_label(label: &Label) -> bool {
    match label {
        Label::Local { name, .. } => is_main(name),
        Label::Remote { branch, .. } => is_main(branch),
        Label::Tag { .. } => false,
    }
}

/// Whether a branch name is one of the usual main branch names.
fn is_main(name: &str) -> bool {
    name == "main" || name == "master"
}
