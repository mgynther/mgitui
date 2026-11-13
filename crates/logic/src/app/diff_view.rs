//! The Diff view: staged and unstaged files and the diff of the selected file.

use super::{App, Note};
use crate::cursor::Cursor;
use crate::event::Event;
use crate::model::{ChangedFile, DiffRow, FileDiff, Side, WorkStatus, diff_rows};
use crate::ports::{Failed, Ports};
use crate::search::{self, Direction, Found};

/// Which pane of the Diff view has focus.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DiffPane {
    /// The staged and unstaged file lists.
    #[default]
    Files,
    /// The diff of the selected file.
    FileDiff,
}

/// The state of the Diff view.
#[derive(Debug, Clone, Default)]
pub struct DiffState {
    status: WorkStatus,
    pane: DiffPane,
    side: Side,
    unstaged: Cursor,
    staged: Cursor,
    rows: Vec<DiffRow>,
    diff_cursor: Cursor,
}

impl DiffState {
    /// The staged and unstaged changes.
    #[must_use]
    pub fn status(&self) -> &WorkStatus {
        &self.status
    }

    /// The focused pane.
    #[must_use]
    pub fn pane(&self) -> DiffPane {
        self.pane
    }

    /// The list the file diff is for.
    #[must_use]
    pub fn side(&self) -> Side {
        self.side
    }

    /// The selection in a file list.
    #[must_use]
    pub fn cursor(&self, side: Side) -> Cursor {
        match side {
            Side::Unstaged => self.unstaged,
            Side::Staged => self.staged,
        }
    }

    /// The files of a list.
    #[must_use]
    pub fn files(&self, side: Side) -> &[ChangedFile] {
        match side {
            Side::Unstaged => &self.status.unstaged,
            Side::Staged => &self.status.staged,
        }
    }

    /// The selected file of the active list.
    #[must_use]
    pub fn selected_file(&self) -> Option<&ChangedFile> {
        self.files(self.side).get(self.cursor(self.side).selected())
    }

    /// The rows of the selected file's diff.
    #[must_use]
    pub fn rows(&self) -> &[DiffRow] {
        &self.rows
    }

    /// The cursor in the file diff.
    #[must_use]
    pub fn diff_cursor(&self) -> Cursor {
        self.diff_cursor
    }

    fn cursor_mut(&mut self, side: Side) -> &mut Cursor {
        match side {
            Side::Unstaged => &mut self.unstaged,
            Side::Staged => &mut self.staged,
        }
    }
}

impl App {
    /// The state of the Diff view.
    #[must_use]
    pub fn diff(&self) -> &DiffState {
        &self.diff
    }

    /// Reloads the staged and unstaged changes, keeping the selected files when they still
    /// exist.
    pub(super) fn reload_status(&mut self, ports: &dyn Ports) {
        let Ok(status) = ports.status() else {
            self.note = Note::Problem("Reloading changes failed, see the Log view".to_string());
            return;
        };
        let status: WorkStatus = status;
        for side in [Side::Unstaged, Side::Staged] {
            let old: Cursor = self.diff.cursor(side);
            let old_path: Option<String> = self
                .diff
                .files(side)
                .get(old.selected())
                .map(|f: &ChangedFile| f.path.clone());
            let new_files: &[ChangedFile] = match side {
                Side::Unstaged => &status.unstaged,
                Side::Staged => &status.staged,
            };
            let index: usize = old_path
                .and_then(|path: String| {
                    new_files.iter().position(|f: &ChangedFile| f.path == path)
                })
                .unwrap_or(old.selected());
            let height: usize = self.list_height(side);
            self.diff
                .cursor_mut(side)
                .select(index, new_files.len(), height);
        }
        self.diff.status = status;
        self.load_file_diff(ports);
    }

    fn list_height(&self, side: Side) -> usize {
        match side {
            Side::Unstaged => self.viewports.diff_unstaged,
            Side::Staged => self.viewports.diff_staged,
        }
    }

    pub(super) fn clamp_diff_cursors(&mut self) {
        for side in [Side::Unstaged, Side::Staged] {
            let len: usize = self.diff.files(side).len();
            let height: usize = self.list_height(side);
            self.diff.cursor_mut(side).clamp(len, height);
        }
        let rows: usize = self.diff.rows.len();
        self.diff.diff_cursor.clamp(rows, self.viewports.diff_file);
    }

    /// Loads the diff of the selected file.
    fn load_file_diff(&mut self, ports: &dyn Ports) {
        let side: Side = self.diff.side;
        let diff: Option<Result<FileDiff, Failed>> = self
            .diff
            .selected_file()
            .map(|file: &ChangedFile| ports.file_diff(file, side));
        self.diff.rows = match diff {
            None => Vec::new(),
            Some(Ok(diff)) => diff_rows(&diff),
            Some(Err(Failed)) => {
                self.note = Note::Problem("Loading the diff failed, see the Log view".to_string());
                Vec::new()
            }
        };
        self.diff.diff_cursor = Cursor::default();
    }

    pub(super) fn handle_diff(&mut self, event: Event, ports: &dyn Ports) {
        match (self.diff.pane, event) {
            (_, Event::NextPane) => {
                self.diff.pane = match self.diff.pane {
                    DiffPane::Files => DiffPane::FileDiff,
                    DiffPane::FileDiff => DiffPane::Files,
                };
            }
            (DiffPane::Files, Event::SwitchList) => {
                self.diff.side = match self.diff.side {
                    Side::Unstaged => Side::Staged,
                    Side::Staged => Side::Unstaged,
                };
                self.load_file_diff(ports);
            }
            (DiffPane::Files, _) => self.move_in_file_list(event, ports),
            (DiffPane::FileDiff, _) => {
                let len: usize = self.diff.rows.len();
                let height: usize = self.viewports.diff_file;
                move_cursor(&mut self.diff.diff_cursor, event, len, height);
            }
        }
    }

    fn move_in_file_list(&mut self, event: Event, ports: &dyn Ports) {
        let side: Side = self.diff.side;
        let len: usize = self.diff.files(side).len();
        let height: usize = self.list_height(side);
        let before: Cursor = self.diff.cursor(side);
        move_cursor(self.diff.cursor_mut(side), event, len, height);
        if self.diff.cursor(side).selected() != before.selected() {
            self.load_file_diff(ports);
        }
    }

    pub(super) fn search_diff(
        &mut self,
        term: &str,
        direction: Direction,
        ports: &dyn Ports,
    ) -> Option<Found> {
        let side: Side = self.diff.side;
        let files: &[ChangedFile] = self.diff.files(side);
        let found: Option<Found> = search::find(
            files.len(),
            self.diff.cursor(side).selected(),
            direction,
            |i: usize| {
                files
                    .get(i)
                    .is_some_and(|f: &ChangedFile| search::text_matches(&f.path, term))
            },
        );
        if let Some(Found { index, .. }) = found {
            let len: usize = files.len();
            let height: usize = self.list_height(side);
            self.diff.cursor_mut(side).select(index, len, height);
            self.diff.pane = DiffPane::Files;
            self.load_file_diff(ports);
        }
        found
    }
}

/// Moves `cursor` for the navigation events j, k, g and G.
pub(super) fn move_cursor(cursor: &mut Cursor, event: Event, len: usize, height: usize) {
    match event {
        Event::MoveDown => cursor.down(len, height),
        Event::MoveUp => cursor.up(len, height),
        Event::Top => cursor.top(),
        Event::Bottom => cursor.bottom(len, height),
        _ => {}
    }
}
