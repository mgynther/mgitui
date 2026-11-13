//! The Branches view: the commit graph of all branches.

use std::collections::HashMap;

use super::{App, Note};
use crate::cursor::Cursor;
use crate::event::Event;
use crate::graph::{GraphBuilder, GraphRow};
use crate::model::{Commit, Label, Refs};
use crate::ports::{Failed, Ports};
use crate::search::{self, Direction, Found};

/// Commits are loaded in chunks of this size.
pub const CHUNK: usize = 2000;

/// More commits are loaded when the selection gets this close to the last loaded commit.
const LOAD_MARGIN: usize = 100;

/// The state of the Branches view.
#[derive(Debug, Clone, Default)]
pub struct BranchesState {
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
        let len: usize = self.branches.commits.len();
        self.branches
            .cursor
            .clamp(len, self.viewports.branches_graph);
    }

    pub(super) fn handle_branches(&mut self, event: Event, ports: &dyn Ports) {
        let len: usize = self.branches.commits.len();
        let height: usize = self.viewports.branches_graph;
        super::diff_view::move_cursor(&mut self.branches.cursor, event, len, height);
        self.load_more_if_needed(ports);
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
            self.load_more_if_needed(ports);
        }
        found
    }
}
