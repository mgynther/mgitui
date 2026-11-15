//! Conversions between the types of the logic and ui layers.

use std::collections::VecDeque;

use logic::app::{
    App, BranchesPane, BranchesState, Confirmation, DiffPane, DiffState, Mode, PromptKind,
    Viewports,
};
use logic::cursor::Cursor;
use logic::graph::{Cell, CommitSide, GraphRow, Shape};
use logic::log::{Layer, LogEntry, LogLevel};
use logic::model::{ChangeKind, ChangedFile, Commit, DiffRow, Label, LineKind, RowKind, Side};
use logic::note::Note;

/// The logic event for a ui event.
#[must_use]
pub fn event(event: ui::event::Event) -> logic::event::Event {
    use logic::event::Event as L;
    use ui::event::Event as U;
    match event {
        U::SwitchView(view) => L::SwitchView(logic_view(view)),
        U::Quit => L::Quit,
        U::Yes => L::Yes,
        U::No => L::No,
        U::Reload => L::Reload,
        U::MoveDown => L::MoveDown,
        U::MoveUp => L::MoveUp,
        U::Top => L::Top,
        U::Bottom => L::Bottom,
        U::NextPane => L::NextPane,
        U::StartSearch => L::StartSearch,
        U::SearchNext => L::SearchNext,
        U::SearchPrevious => L::SearchPrevious,
        U::Cancel => L::Cancel,
        U::Activate => L::Activate,
        U::CreateBranch => L::CreateBranch,
        U::Checkout => L::Checkout,
        U::CherryPick => L::CherryPick,
        U::RemoteUpdate => L::RemoteUpdate,
        U::FetchAllPrune => L::FetchAllPrune,
        U::NextMainBranch => L::NextMainBranch,
        U::ShowCommit => L::ShowCommit,
        U::ResetBranch => L::ResetBranch,
        U::DeleteBranch => L::DeleteBranch,
        U::DiscardFile => L::DiscardFile,
        U::StageAll => L::StageAll,
        U::Rebase => L::Rebase,
        U::InteractiveRebase => L::InteractiveRebase,
        U::SwitchList => L::SwitchList,
        U::StartSelection => L::StartSelection,
        U::EditMessage => L::EditMessage,
        U::Commit => L::Commit,
        U::ToggleAmend => L::ToggleAmend,
        U::Input(c) => L::Input(c),
        U::InputBackspace => L::InputBackspace,
        U::InputSubmit => L::InputSubmit,
        U::InputCancel => L::InputCancel,
    }
}

/// The logic view for a ui view.
#[must_use]
pub fn logic_view(view: ui::event::ViewId) -> logic::event::ViewId {
    match view {
        ui::event::ViewId::Branches => logic::event::ViewId::Branches,
        ui::event::ViewId::Diff => logic::event::ViewId::Diff,
        ui::event::ViewId::Log => logic::event::ViewId::Log,
    }
}

/// The ui view for a logic view.
#[must_use]
pub fn ui_view(view: logic::event::ViewId) -> ui::event::ViewId {
    match view {
        logic::event::ViewId::Branches => ui::event::ViewId::Branches,
        logic::event::ViewId::Diff => ui::event::ViewId::Diff,
        logic::event::ViewId::Log => ui::event::ViewId::Log,
    }
}

/// The ui input mode for a logic mode.
#[must_use]
pub fn input_mode(mode: &Mode) -> ui::event::InputMode {
    match mode {
        Mode::Normal => ui::event::InputMode::Normal,
        Mode::Confirm(_) => ui::event::InputMode::Confirm,
        Mode::Prompt(_) => ui::event::InputMode::Text,
    }
}

/// The ui size for the terminal's size.
#[must_use]
pub fn ui_size(size: terminal::size::Size) -> ui::layout::Size {
    ui::layout::Size {
        width: size.columns,
        height: size.rows,
    }
}

/// How many rows each list has for a terminal of `size`.
#[must_use]
pub fn viewports(size: ui::layout::Size) -> Viewports {
    let diff: ui::layout::DiffLayout = ui::layout::diff_layout(size);
    let branches: ui::layout::BranchesLayout = ui::layout::branches_layout(size);
    Viewports {
        branches_graph: branches.graph.content().height,
        branches_files: branches.files.content().height,
        branches_diff: branches.file_diff.content().height,
        log: ui::layout::log_layout(size).list.content().height,
        diff_unstaged: diff.unstaged.content().height,
        diff_staged: diff.staged.content().height,
        diff_file: diff.file_diff.content().height,
    }
}

/// The screen model for the application state. This and the view functions that take the
/// [`App`] only pick values out of it for the conversions, which take those values, so that each
/// conversion can be tested without driving an [`App`] into a state.
#[must_use]
pub fn screen(app: &App) -> ui::screen::Screen {
    ui::screen::Screen {
        tabs: ui::screen::Tabs {
            active: ui_view(app.view()),
            diff_has_changes: app.diff().status().has_changes(),
            log_has_problems: app.log().has_unseen_problems(),
        },
        status: status(app.mode(), app.note()),
        body: match app.view() {
            logic::event::ViewId::Branches => ui::screen::Body::Branches(branches_view(app)),
            logic::event::ViewId::Diff => ui::screen::Body::Diff(diff_view(app)),
            logic::event::ViewId::Log => ui::screen::Body::Log(log_view(
                app.log().entries(),
                app.log_cursor(),
                app.viewports().log,
            )),
        },
    }
}

fn status(mode: &Mode, note: &Note) -> ui::screen::Status {
    match (mode, note) {
        (Mode::Confirm(confirmation), _) => ui::screen::Status::Question(question(confirmation)),
        (Mode::Prompt(prompt), note) => ui::screen::Status::Prompt {
            kind: prompt_kind(&prompt.kind),
            text: prompt.text.clone(),
            problem: match note {
                Note::Problem(problem) => Some(ui_problem(problem)),
                Note::None | Note::Info(_) => None,
            },
        },
        (Mode::Normal, Note::None) => ui::screen::Status::Empty,
        (Mode::Normal, Note::Info(info)) => ui::screen::Status::Note(ui_info(info)),
        (Mode::Normal, Note::Problem(problem)) => ui::screen::Status::Problem(ui_problem(problem)),
    }
}

fn ui_info(info: &logic::note::Info) -> ui::status::Info {
    use logic::note::Info as I;
    use ui::status::Info as U;
    match info {
        I::NoCommitSelected => U::NoCommitSelected,
        I::CreatedBranch { name } => U::CreatedBranch { name: name.clone() },
        I::DeletedBranch { name } => U::DeletedBranch { name: name.clone() },
        I::NoOtherBranchToRebaseOnto => U::NoOtherBranchToRebaseOnto,
        I::Rebased { branch, onto } => U::Rebased {
            branch: branch.clone(),
            onto: onto.clone(),
        },
        I::RebasedInteractively { branch, onto } => U::RebasedInteractively {
            branch: branch.clone(),
            onto: onto.clone(),
        },
        I::CheckedOut { branch } => U::CheckedOut {
            branch: branch.clone(),
        },
        I::UpdatedRemotes => U::UpdatedRemotes,
        I::FetchedAndPruned => U::FetchedAndPruned,
        I::ResetBranch { branch, hash } => U::ResetBranch {
            branch: branch.clone(),
            hash: hash.clone(),
        },
        I::CherryPicked { hash } => U::CherryPicked { hash: hash.clone() },
        I::NoMainOrMasterBranch => U::NoMainOrMasterBranch,
        I::OnlyWholeFilesCanBeStagedHere => U::OnlyWholeFilesCanBeStagedHere,
        I::OnlyUnstagedChangesCanBeDiscarded => U::OnlyUnstagedChangesCanBeDiscarded,
        I::NothingToStage => U::NothingToStage,
        I::NoChangedLinesSelected => U::NoChangedLinesSelected,
        I::DeletedFile { path } => U::DeletedFile { path: path.clone() },
        I::DiscardedChanges { path } => U::DiscardedChanges { path: path.clone() },
        I::NoPreviousSearch => U::NoPreviousSearch,
        I::PatternNotFound { term } => U::PatternNotFound { term: term.clone() },
        I::SearchWrapped => U::SearchWrapped,
    }
}

fn ui_problem(problem: &logic::note::Problem) -> ui::status::Problem {
    use logic::note::Problem as P;
    use ui::status::Problem as U;
    match problem {
        P::Failed(operation) => U::Failed(ui_operation(*operation)),
        P::Invalid(input) => U::Invalid(ui_input_problem(input)),
        P::ChoiceNotListed => U::ChoiceNotListed,
        P::NoLocalBranchToCheckOut => U::NoLocalBranchToCheckOut,
        P::NoLocalBranchToDelete => U::NoLocalBranchToDelete,
        P::CheckedOutBranchCannotBeDeleted => U::CheckedOutBranchCannotBeDeleted,
        P::NoLocalBranchToRebase => U::NoLocalBranchToRebase,
        P::NoBranchCheckedOutToReset => U::NoBranchCheckedOutToReset,
        P::UncommittedChangesBeforeReset => U::UncommittedChangesBeforeReset,
        P::MergeConflict { path } => U::MergeConflict { path: path.clone() },
        P::MergeConflictsBeforeStagingAll => U::MergeConflictsBeforeStagingAll,
        P::NothingToAmend => U::NothingToAmend,
        P::AmendedCommitWouldBeEmpty => U::AmendedCommitWouldBeEmpty,
    }
}

fn ui_operation(operation: logic::note::Operation) -> ui::status::Operation {
    use logic::note::Operation as O;
    use ui::status::Operation as U;
    match operation {
        O::ReloadingCommits => U::ReloadingCommits,
        O::LoadingCommit => U::LoadingCommit,
        O::LoadingDiff => U::LoadingDiff,
        O::LoadingMoreCommits => U::LoadingMoreCommits,
        O::CreatingBranch => U::CreatingBranch,
        O::DeletingBranch => U::DeletingBranch,
        O::Rebasing => U::Rebasing,
        O::CheckingOut => U::CheckingOut,
        O::UpdatingRemotes => U::UpdatingRemotes,
        O::FetchingAndPruning => U::FetchingAndPruning,
        O::ShowingCommit => U::ShowingCommit,
        O::ReadingStatus => U::ReadingStatus,
        O::ResettingBranch => U::ResettingBranch,
        O::CherryPicking => U::CherryPicking,
        O::ReloadingChanges => U::ReloadingChanges,
        O::DeletingFile => U::DeletingFile,
        O::DiscardingChanges => U::DiscardingChanges,
        O::Staging => U::Staging,
        O::EditingMessage => U::EditingMessage,
        O::ReadingLastCommit => U::ReadingLastCommit,
        O::Committing => U::Committing,
    }
}

fn ui_input_problem(problem: &logic::note::InputProblem) -> ui::status::InputProblem {
    use logic::note::InputProblem as I;
    use ui::status::InputProblem as U;
    match problem {
        I::BranchName(rule) => U::BranchName(ui_branch_name_problem(*rule)),
        I::NotANumber => U::NotANumber,
        I::OutOfRange { count } => U::OutOfRange { count: *count },
        I::EmptySearch => U::EmptySearch,
        I::EmptyCommitMessage => U::EmptyCommitMessage,
    }
}

fn ui_branch_name_problem(rule: logic::note::BranchNameProblem) -> ui::status::BranchNameProblem {
    use logic::note::BranchNameProblem as B;
    use ui::status::BranchNameProblem as U;
    match rule {
        B::Empty => U::Empty,
        B::Reserved => U::Reserved,
        B::StartsWithDash => U::StartsWithDash,
        B::BadSlash => U::BadSlash,
        B::ComponentStartsWithDot => U::ComponentStartsWithDot,
        B::ComponentEndsWithLock => U::ComponentEndsWithLock,
        B::EndsWithDot => U::EndsWithDot,
        B::DoubleDot => U::DoubleDot,
        B::AtBrace => U::AtBrace,
        B::ForbiddenCharacter(c) => U::ForbiddenCharacter(c),
    }
}

fn question(confirmation: &Confirmation) -> ui::status::Question {
    use ui::status::Question;
    match confirmation {
        Confirmation::Quit => Question::Quit,
        Confirmation::DeleteBranch { name } => Question::DeleteBranch { name: name.clone() },
        Confirmation::ResetBranch { branch, hash } => Question::ResetBranch {
            branch: branch.clone(),
            hash: hash.clone(),
        },
        Confirmation::DeleteUntracked { file } => Question::DeleteUntracked {
            path: file.path.clone(),
        },
        Confirmation::DiscardChanges { file } => Question::DiscardChanges {
            path: file.path.clone(),
        },
    }
}

fn prompt_kind(kind: &PromptKind) -> ui::status::PromptKind {
    use ui::status::PromptKind as K;
    match kind {
        PromptKind::Search => K::Search,
        PromptKind::BranchName { .. } => K::BranchName,
        PromptKind::CheckoutChoice { branches } => K::CheckoutChoice(branches.clone()),
        PromptKind::DeleteBranch { branches } => K::DeleteBranch(branches.clone()),
        PromptKind::RebaseBranch {
            branches,
            interactive,
        } => K::RebaseBranch {
            branches: branches.clone(),
            interactive: *interactive,
        },
        PromptKind::RebaseOnto {
            branch,
            onto,
            interactive,
        } => K::RebaseOnto {
            branch: branch.clone(),
            onto: onto.clone(),
            interactive: *interactive,
        },
    }
}

fn log_view(
    entries: &VecDeque<LogEntry>,
    cursor: Cursor,
    height: usize,
) -> ui::views::log::LogView {
    let rows: Vec<ui::views::log::LogRow> = entries
        .iter()
        .enumerate()
        .skip(cursor.offset())
        .take(height)
        .map(
            |(index, entry): (usize, &LogEntry)| ui::views::log::LogRow {
                time: entry.time,
                layer: ui_layer(entry.layer),
                level: ui_level(entry.level),
                message: entry.message.clone(),
                selected: index == cursor.selected(),
            },
        )
        .collect();
    ui::views::log::LogView { rows }
}

fn branches_view(app: &App) -> ui::views::branches::BranchesView {
    let branches: &BranchesState = app.branches();
    let viewports: Viewports = app.viewports();
    ui::views::branches::BranchesView {
        focus: branches_focus(branches.pane()),
        rows: commit_rows(
            branches.commits(),
            branches.rows(),
            |hash: &str| branches.labels(hash),
            branches.head(),
            branches.cursor(),
            viewports.branches_graph,
        ),
        info: branches.selected().map(|commit: &Commit| {
            commit_info(commit, branches.selected_lane(), branches.message())
        }),
        files: file_rows(
            branches.files(),
            branches.file_cursor(),
            viewports.branches_files,
        ),
        diff_file: branches
            .selected_file()
            .map(|f: &ChangedFile| f.path.clone()),
        diff: diff_rows(
            branches.diff_rows(),
            branches.diff_cursor(),
            None,
            viewports.branches_diff,
        ),
    }
}

fn branches_focus(pane: BranchesPane) -> ui::views::branches::BranchesFocus {
    use ui::views::branches::BranchesFocus;
    match pane {
        BranchesPane::Graph => BranchesFocus::Graph,
        BranchesPane::Files => BranchesFocus::Files,
        BranchesPane::FileDiff => BranchesFocus::FileDiff,
    }
}

/// The visible rows of the commit graph: `commits` with their `graph` rows, the `labels` of
/// each commit by hash, and `head`, the commit `HEAD` points to.
fn commit_rows<'a>(
    commits: &[Commit],
    graph: &[GraphRow],
    labels: impl Fn(&str) -> &'a [Label],
    head: Option<&str>,
    cursor: Cursor,
    height: usize,
) -> Vec<ui::views::branches::CommitRow> {
    commits
        .iter()
        .zip(graph)
        .enumerate()
        .skip(cursor.offset())
        .take(height)
        .map(|(index, (commit, row)): (usize, (&Commit, &GraphRow))| {
            ui::views::branches::CommitRow {
                cells: row.cells.iter().map(graph_cell).collect(),
                labels: labels(&commit.hash).iter().map(label_view).collect(),
                subject: commit.subject.clone(),
                author: commit.author.clone(),
                date: commit.date.clone(),
                selected: index == cursor.selected(),
                head: head == Some(commit.hash.as_str()),
            }
        })
        .collect()
}

fn commit_info(
    commit: &Commit,
    lane: Option<usize>,
    message: &str,
) -> ui::views::branches::CommitInfo {
    ui::views::branches::CommitInfo {
        hash: commit.hash.clone(),
        lane,
        author: commit.author.clone(),
        author_email: commit.author_email.clone(),
        author_date: commit.date.clone(),
        committer: commit.committer.clone(),
        committer_email: commit.committer_email.clone(),
        committer_date: commit.committer_date.clone(),
        message: message.lines().map(str::to_string).collect(),
    }
}

fn graph_cell(cell: &Cell) -> ui::views::branches::GraphCell {
    use ui::views::branches::{CommitSide as S, GraphShape as G};
    let side = |side: CommitSide| match side {
        CommitSide::Left => S::Left,
        CommitSide::Right => S::Right,
    };
    ui::views::branches::GraphCell {
        shape: match cell.shape {
            Shape::Empty => G::Empty,
            Shape::Commit => G::Commit,
            Shape::Vertical => G::Vertical,
            Shape::Horizontal => G::Horizontal,
            Shape::Crossing => G::Crossing,
            Shape::Join(s) => G::Join(side(s)),
            Shape::Fork(s) => G::Fork(side(s)),
            Shape::Tee(s) => G::Tee(side(s)),
        },
        lane: cell.lane,
        link_right: cell.link_right,
    }
}

fn label_view(label: &Label) -> ui::views::branches::LabelView {
    use ui::views::branches::LabelView;
    match label {
        Label::Local { name, head } => LabelView::Local {
            name: name.clone(),
            head: *head,
        },
        Label::Remote { remote, branch } => LabelView::Remote {
            remote: remote.clone(),
            branch: branch.clone(),
        },
        Label::Tag { name } => LabelView::Tag(name.clone()),
    }
}

fn diff_view(app: &App) -> ui::views::diff::DiffView {
    let diff: &DiffState = app.diff();
    let viewports: Viewports = app.viewports();
    ui::views::diff::DiffView {
        focus: diff_focus(diff.pane(), diff.side()),
        unstaged: file_rows(
            diff.files(Side::Unstaged),
            diff.cursor(Side::Unstaged),
            viewports.diff_unstaged,
        ),
        staged: file_rows(
            diff.files(Side::Staged),
            diff.cursor(Side::Staged),
            viewports.diff_staged,
        ),
        diff_file: diff.selected_file().map(|f: &ChangedFile| f.path.clone()),
        diff: diff_rows(
            diff.rows(),
            diff.diff_cursor(),
            diff.selection(),
            viewports.diff_file,
        ),
        amend: diff.amend(),
        message: diff.message().to_string(),
        no_changes: !diff.status().has_changes(),
    }
}

fn diff_focus(pane: DiffPane, side: Side) -> ui::views::diff::DiffFocus {
    match (pane, side) {
        (DiffPane::FileDiff, _) => ui::views::diff::DiffFocus::FileDiff,
        (DiffPane::Files, Side::Unstaged) => ui::views::diff::DiffFocus::Unstaged,
        (DiffPane::Files, Side::Staged) => ui::views::diff::DiffFocus::Staged,
    }
}

fn file_rows(
    files: &[ChangedFile],
    cursor: Cursor,
    height: usize,
) -> Vec<ui::views::file_list::FileRow> {
    files
        .iter()
        .enumerate()
        .skip(cursor.offset())
        .take(height)
        .map(
            |(index, file): (usize, &ChangedFile)| ui::views::file_list::FileRow {
                change: file_change(file.kind),
                path: file.path.clone(),
                original_path: file.original_path.clone(),
                mode_changed: file.mode_changed,
                selected: index == cursor.selected(),
            },
        )
        .collect()
}

/// The visible rows of a file diff.
pub(crate) fn diff_rows(
    rows: &[DiffRow],
    cursor: Cursor,
    selection: Option<(usize, usize)>,
    height: usize,
) -> Vec<ui::views::diff_text::DiffRowView> {
    rows.iter()
        .enumerate()
        .skip(cursor.offset())
        .take(height)
        .map(
            |(index, row): (usize, &DiffRow)| ui::views::diff_text::DiffRowView {
                kind: diff_row_kind(row.kind),
                text: row.text.clone(),
                cursor: index == cursor.selected(),
                marked: selection
                    .is_some_and(|(first, last): (usize, usize)| (first..=last).contains(&index)),
            },
        )
        .collect()
}

fn diff_row_kind(kind: RowKind) -> ui::views::diff_text::DiffRowKind {
    use ui::views::diff_text::DiffRowKind as K;
    match kind {
        RowKind::HunkHeader => K::HunkHeader,
        RowKind::Line(LineKind::Context) => K::Context,
        RowKind::Line(LineKind::Added) => K::Added,
        RowKind::Line(LineKind::Removed) => K::Removed,
        RowKind::Line(LineKind::NoNewline) => K::NoNewline,
        RowKind::BinaryFile => K::BinaryFile,
        RowKind::NoContentChanges => K::NoContentChanges,
    }
}

fn file_change(kind: ChangeKind) -> ui::views::file_list::FileChange {
    use ui::views::file_list::FileChange as F;
    match kind {
        ChangeKind::Modified => F::Modified,
        ChangeKind::Added => F::Added,
        ChangeKind::Deleted => F::Deleted,
        ChangeKind::Renamed => F::Renamed,
        ChangeKind::Copied => F::Copied,
        ChangeKind::TypeChanged => F::TypeChanged,
        ChangeKind::Untracked => F::Untracked,
        ChangeKind::Conflict => F::Conflict,
    }
}

fn ui_layer(layer: Layer) -> ui::views::log::Layer {
    use ui::views::log::Layer as U;
    match layer {
        Layer::Git => U::Git,
        Layer::Logic => U::Logic,
        Layer::Ui => U::Ui,
        Layer::Validation => U::Validation,
        Layer::Wiring => U::Wiring,
    }
}

fn ui_level(level: LogLevel) -> ui::log::LogLevel {
    match level {
        LogLevel::Error => ui::log::LogLevel::Error,
        LogLevel::Warn => ui::log::LogLevel::Warn,
        LogLevel::Info => ui::log::LogLevel::Info,
        LogLevel::Debug => ui::log::LogLevel::Debug,
    }
}

#[cfg(test)]
mod tests;
