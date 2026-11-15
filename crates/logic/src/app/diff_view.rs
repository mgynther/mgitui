//! The Diff view: staged and unstaged files and the diff of the selected file.

use std::collections::BTreeSet;

use super::{App, Confirmation, Mode, Note};
use crate::cursor::Cursor;
use crate::event::Event;
use crate::log::LogLevel;
use crate::model::{ChangeKind, ChangedFile, DiffRow, FileDiff, Side, WorkStatus, diff_rows};
use crate::patch::{LineRef, build_patch};
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
    file_diff: Option<FileDiff>,
    rows: Vec<DiffRow>,
    diff_cursor: Cursor,
    anchor: Option<usize>,
    draft: String,
    amend_draft: String,
    amend: bool,
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

    /// The first and last selected rows of the file diff, while lines are being selected.
    #[must_use]
    pub fn selection(&self) -> Option<(usize, usize)> {
        let cursor: usize = self.diff_cursor.selected();
        self.anchor
            .map(|anchor: usize| (anchor.min(cursor), anchor.max(cursor)))
    }

    /// The commit message shown in the message box: the amend draft in amend mode, the new
    /// commit draft otherwise.
    #[must_use]
    pub fn message(&self) -> &str {
        if self.amend {
            &self.amend_draft
        } else {
            &self.draft
        }
    }

    /// Whether committing amends the last commit.
    #[must_use]
    pub fn amend(&self) -> bool {
        self.amend
    }

    fn message_mut(&mut self) -> &mut String {
        if self.amend {
            &mut self.amend_draft
        } else {
            &mut self.draft
        }
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
        let diff: Option<Result<FileDiff, Failed>> =
            self.diff.selected_file().map(|file: &ChangedFile| {
                if file.kind == ChangeKind::Untracked {
                    ports.untracked_file_diff(&file.path)
                } else {
                    ports.file_diff(&file.paths(side), side)
                }
            });
        self.diff.file_diff = match diff {
            None => None,
            Some(Ok(diff)) => Some(diff),
            Some(Err(Failed)) => {
                self.note = Note::Problem("Loading the diff failed, see the Log view".to_string());
                None
            }
        };
        self.diff.rows = self
            .diff
            .file_diff
            .as_ref()
            .map_or_else(Vec::new, diff_rows);
        self.diff.diff_cursor = Cursor::default();
        self.diff.anchor = None;
    }

    pub(super) fn handle_diff(&mut self, event: Event, ports: &dyn Ports) {
        match (self.diff.pane, event) {
            (_, Event::EditMessage) => self.edit_message(ports),
            (_, Event::Commit) => self.commit(ports),
            (_, Event::ToggleAmend) => self.toggle_amend(ports),
            (_, Event::StageAll) => self.stage_all(ports),
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
            (DiffPane::Files, Event::Activate) => self.stage_whole_file(ports),
            (DiffPane::Files, Event::DiscardFile) => self.start_discard(ports),
            (DiffPane::Files, _) => self.move_in_file_list(event, ports),
            (DiffPane::FileDiff, Event::StartSelection) => {
                if self.line_staging_allowed(ports) {
                    self.diff.anchor = Some(self.diff.diff_cursor.selected());
                }
            }
            (DiffPane::FileDiff, Event::Cancel) => self.diff.anchor = None,
            (DiffPane::FileDiff, Event::Activate) => self.stage_lines(ports),
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

    /// The selected file, unless it has a conflict: then the action is refused and logged.
    fn file_for_action(&mut self, ports: &dyn Ports) -> Option<ChangedFile> {
        let file: ChangedFile = self.diff.selected_file()?.clone();
        if file.kind == ChangeKind::Conflict {
            let message: String = format!(
                "{} has a merge conflict; resolve it outside mgitui",
                file.path
            );
            ports.log(LogLevel::Error, &message);
            self.note = Note::Problem(message);
            return None;
        }
        Some(file)
    }

    fn line_staging_allowed(&mut self, ports: &dyn Ports) -> bool {
        let Some(file) = self.file_for_action(ports) else {
            return false;
        };
        let allowed: bool = file.supports_line_staging();
        if !allowed {
            self.note = Note::Info(
                "Only whole files can be staged here: use Enter in the file list".to_string(),
            );
        }
        allowed
    }

    fn stage_whole_file(&mut self, ports: &dyn Ports) {
        let Some(file) = self.file_for_action(ports) else {
            return;
        };
        let result: Result<(), Failed> = match self.diff.side {
            Side::Unstaged => {
                ports.log(LogLevel::Info, &format!("stage {}", file.path));
                ports.stage_paths(&file.paths(Side::Unstaged))
            }
            Side::Staged => {
                ports.log(LogLevel::Info, &format!("unstage {}", file.path));
                unstage_whole_file(&file, ports)
            }
        };
        self.after_staging(result, ports);
    }

    /// Stages all unstaged changes, untracked files included. Refused while a file has a
    /// merge conflict: staging it would mark it resolved, conflict markers and all.
    fn stage_all(&mut self, ports: &dyn Ports) {
        let status: Result<WorkStatus, Failed> = ports.status();
        let Ok(status) = status else {
            self.note = Note::Problem("Reading the status failed, see the Log view".to_string());
            return;
        };
        if status
            .unstaged
            .iter()
            .any(|file: &ChangedFile| file.kind == ChangeKind::Conflict)
        {
            let message: &str = "Resolve merge conflicts outside mgitui before staging all";
            ports.log(LogLevel::Error, message);
            self.note = Note::Problem(message.to_string());
            return;
        }
        if status.unstaged.is_empty() {
            self.note = Note::Info("Nothing to stage".to_string());
            return;
        }
        ports.log(LogLevel::Info, "stage all");
        let result: Result<(), Failed> = ports.stage_all();
        self.after_staging(result, ports);
    }

    fn stage_lines(&mut self, ports: &dyn Ports) {
        if !self.line_staging_allowed(ports) {
            return;
        }
        let cursor: usize = self.diff.diff_cursor.selected();
        let (first, last): (usize, usize) = self.diff.selection().unwrap_or((cursor, cursor));
        let selected: BTreeSet<LineRef> = self
            .diff
            .rows
            .iter()
            .skip(first)
            .take(last + 1 - first)
            .filter_map(|row: &DiffRow| row.line.map(|line: usize| (row.hunk, line)))
            .collect();
        let reverse: bool = self.diff.side == Side::Staged;
        let patch: Option<String> = self
            .diff
            .file_diff
            .as_ref()
            .and_then(|diff: &FileDiff| build_patch(diff, &selected, reverse));
        let Some(patch) = patch else {
            self.note = Note::Info("No changed lines selected".to_string());
            return;
        };
        let file_path: String = self
            .diff
            .selected_file()
            .map_or_else(String::new, |f: &ChangedFile| f.path.clone());
        let verb: &str = if reverse { "unstage" } else { "stage" };
        ports.log(LogLevel::Info, &format!("{verb} lines of {file_path}"));
        let result: Result<(), Failed> = ports.apply_to_index(&patch, reverse);
        self.after_staging(result, ports);
        let same_file: bool = self
            .diff
            .selected_file()
            .is_some_and(|f: &ChangedFile| f.path == file_path);
        if same_file {
            let len: usize = self.diff.rows.len();
            self.diff
                .diff_cursor
                .select(cursor, len, self.viewports.diff_file);
        }
    }

    /// Asks for confirmation before discarding the unstaged changes of the selected file.
    fn start_discard(&mut self, ports: &dyn Ports) {
        if self.diff.side == Side::Staged {
            self.note = Note::Info(
                "Only unstaged changes can be discarded: unstage the file first".to_string(),
            );
            return;
        }
        if let Some(file) = self.file_for_action(ports) {
            self.mode = Mode::Confirm(Confirmation::DiscardFile { file });
        }
    }

    pub(super) fn discard_file(&mut self, file: &ChangedFile, ports: &dyn Ports) {
        let untracked: bool = file.kind == ChangeKind::Untracked;
        let action: &str = if untracked {
            "delete"
        } else {
            "discard changes in"
        };
        ports.log(LogLevel::Info, &format!("{action} {}", file.path));
        let paths: Vec<String> = file.paths(Side::Unstaged);
        let result: Result<(), Failed> = if untracked {
            ports.delete_untracked(&paths)
        } else {
            ports.restore_paths(&paths)
        };
        self.refresh(ports);
        self.note = match (result, untracked) {
            (Ok(()), true) => Note::Info(format!("Deleted {}", file.path)),
            (Ok(()), false) => Note::Info(format!("Discarded changes in {}", file.path)),
            (Err(Failed), _) => Note::Problem("Discarding failed, see the Log view".to_string()),
        };
    }

    fn after_staging(&mut self, result: Result<(), Failed>, ports: &dyn Ports) {
        self.refresh(ports);
        if result.is_err() {
            self.note = Note::Problem("Staging failed, see the Log view".to_string());
        }
    }

    fn edit_message(&mut self, ports: &dyn Ports) {
        ports.log(LogLevel::Info, "edit commit message");
        match ports.edit_message(self.diff.message()) {
            Ok(text) => *self.diff.message_mut() = text,
            Err(Failed) => {
                self.note = Note::Problem("Editing failed, see the Log view".to_string());
            }
        }
    }

    fn toggle_amend(&mut self, ports: &dyn Ports) {
        if self.diff.amend {
            self.diff.amend = false;
            self.diff.amend_draft.clear();
            return;
        }
        match ports.has_commits() {
            Ok(false) => {
                let message: &str = "Nothing to amend: there are no commits yet";
                ports.log(LogLevel::Warn, message);
                self.note = Note::Problem(message.to_string());
            }
            Ok(true) => match ports.last_commit_message() {
                Ok(message) => {
                    self.diff.amend_draft = message;
                    self.diff.amend = true;
                }
                Err(Failed) => self.amend_failed(),
            },
            Err(Failed) => self.amend_failed(),
        }
    }

    fn amend_failed(&mut self) {
        self.note = Note::Problem("Reading the last commit failed, see the Log view".to_string());
    }

    fn commit(&mut self, ports: &dyn Ports) {
        let message: crate::ports::CommitMessage =
            match ports.validate_commit_message(self.diff.message()) {
                Ok(message) => message,
                Err(problem) => {
                    self.note = Note::Problem(problem);
                    return;
                }
            };
        let amend: bool = self.diff.amend;
        ports.log(LogLevel::Info, if amend { "amend" } else { "commit" });
        let result: Result<(), Failed> = ports.commit(&message, amend);
        if result.is_ok() {
            self.diff.message_mut().clear();
            self.diff.amend = false;
        }
        self.refresh(ports);
        if result.is_err() {
            self.note = Note::Problem("Commit failed, see the Log view".to_string());
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

/// Unstages all changes of a file in the staged list. Without commits there is nothing to
/// restore the index from, so the file is removed from the index instead.
fn unstage_whole_file(file: &ChangedFile, ports: &dyn Ports) -> Result<(), Failed> {
    let paths: Vec<String> = file.paths(Side::Staged);
    let has_commits: Result<bool, Failed> = ports.has_commits();
    match has_commits {
        Ok(true) => ports.unstage_paths(&paths),
        Ok(false) => ports.remove_from_index(&paths),
        Err(Failed) => Err(Failed),
    }
}
