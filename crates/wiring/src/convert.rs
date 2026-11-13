//! Conversions between the types of the logic and ui layers.

use logic::app::{
    App, BranchesPane, BranchesState, DiffPane, DiffState, Mode, Note, Prompt, PromptKind,
    Viewports,
};
use logic::cursor::Cursor;
use logic::graph::{Cell, CommitSide, Shape};
use logic::log::{Layer, LogEntry, LogLevel};
use logic::model::{ChangeKind, ChangedFile, Commit, DiffRow, Label, LineKind, RowKind, Side};

/// The logic event for a ui event.
#[must_use]
pub fn event(event: ui::event::Event) -> logic::event::Event {
    use logic::event::Event as L;
    use ui::event::Event as U;
    match event {
        U::SwitchView(view) => L::SwitchView(logic_view(view)),
        U::Quit => L::Quit,
        U::ConfirmQuit => L::ConfirmQuit,
        U::CancelQuit => L::CancelQuit,
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
        U::DeleteBranch => L::DeleteBranch,
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
        Mode::QuitConfirm => ui::event::InputMode::QuitConfirm,
        Mode::Prompt(_) => ui::event::InputMode::Text,
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

/// The screen model for the application state.
#[must_use]
pub fn screen(app: &App) -> ui::screen::Screen {
    ui::screen::Screen {
        tabs: ui::screen::Tabs {
            active: ui_view(app.view()),
            diff_has_changes: app.diff().status().has_changes(),
            log_has_problems: app.log().has_unseen_problems(),
        },
        status: status(app),
        body: match app.view() {
            logic::event::ViewId::Branches => ui::screen::Body::Branches(branches_view(app)),
            logic::event::ViewId::Diff => ui::screen::Body::Diff(diff_view(app)),
            logic::event::ViewId::Log => ui::screen::Body::Log(log_view(app)),
        },
    }
}

fn status(app: &App) -> ui::screen::Status {
    match (app.mode(), app.note()) {
        (Mode::QuitConfirm, _) => ui::screen::Status::Prompt {
            label: "Quit? y/n".to_string(),
            text: String::new(),
            problem: None,
        },
        (Mode::Prompt(prompt), note) => ui::screen::Status::Prompt {
            label: prompt_label(prompt),
            text: prompt.text.clone(),
            problem: match note {
                Note::Problem(problem) => Some(problem.clone()),
                Note::None | Note::Info(_) => None,
            },
        },
        (Mode::Normal, Note::None) => ui::screen::Status::Empty,
        (Mode::Normal, Note::Info(text)) => ui::screen::Status::Note(text.clone()),
        (Mode::Normal, Note::Problem(text)) => ui::screen::Status::Problem(text.clone()),
    }
}

fn prompt_label(prompt: &Prompt) -> String {
    match &prompt.kind {
        PromptKind::Search => "/".to_string(),
        PromptKind::BranchName { .. } => "New branch name: ".to_string(),
        PromptKind::CheckoutChoice { branches } => {
            format!("Check out ({}): ", numbered(branches))
        }
        PromptKind::DeleteBranch { branches } => {
            format!("Delete which branch ({}): ", numbered(branches))
        }
        PromptKind::RebaseBranch {
            branches,
            interactive,
        } => format!(
            "{} which branch ({}): ",
            rebase_verb(*interactive),
            numbered(branches)
        ),
        PromptKind::RebaseOnto {
            branch,
            onto,
            interactive,
        } => format!(
            "{} {branch} onto ({}): ",
            rebase_verb(*interactive),
            numbered(onto)
        ),
    }
}

fn rebase_verb(interactive: bool) -> &'static str {
    if interactive {
        "Interactively rebase"
    } else {
        "Rebase"
    }
}

/// `1 a, 2 b, ...` for a numbered prompt.
fn numbered(items: &[String]) -> String {
    let choices: Vec<String> = items
        .iter()
        .enumerate()
        .map(|(index, item): (usize, &String)| format!("{} {item}", index + 1))
        .collect();
    choices.join(", ")
}

fn log_view(app: &App) -> ui::views::log::LogView {
    let cursor: logic::cursor::Cursor = app.log_cursor();
    let rows: Vec<ui::views::log::LogRow> = app
        .log()
        .entries()
        .iter()
        .enumerate()
        .skip(cursor.offset())
        .take(app.viewports().log)
        .map(
            |(index, entry): (usize, &LogEntry)| ui::views::log::LogRow {
                time: entry.time,
                layer: layer_name(entry.layer).to_string(),
                level: ui_level(entry.level),
                message: entry.message.clone(),
                selected: index == cursor.selected(),
            },
        )
        .collect();
    ui::views::log::LogView { rows }
}

fn branches_view(app: &App) -> ui::views::branches::BranchesView {
    use ui::views::branches::{BranchesFocus, BranchesView, CommitRow};
    let branches: &BranchesState = app.branches();
    let viewports: Viewports = app.viewports();
    let cursor: Cursor = branches.cursor();
    let rows: Vec<CommitRow> = branches
        .commits()
        .iter()
        .zip(branches.rows())
        .enumerate()
        .skip(cursor.offset())
        .take(viewports.branches_graph)
        .map(
            |(index, (commit, row)): (usize, (&Commit, &logic::graph::GraphRow))| CommitRow {
                cells: row.cells.iter().map(graph_cell).collect(),
                labels: branches
                    .labels(&commit.hash)
                    .iter()
                    .map(label_view)
                    .collect(),
                subject: commit.subject.clone(),
                author: commit.author.clone(),
                date: commit.date.clone(),
                selected: index == cursor.selected(),
                head: branches.head() == Some(commit.hash.as_str()),
            },
        )
        .collect();
    let info: Vec<String> = branches
        .selected()
        .map_or_else(Vec::new, |commit: &Commit| {
            let mut lines: Vec<String> = vec![
                format!("commit {}", commit.hash),
                format!("Author: {}  Date: {}", commit.author, commit.date),
            ];
            lines.extend(branches.message().lines().map(str::to_string));
            lines
        });
    BranchesView {
        focus: match branches.pane() {
            BranchesPane::Graph => BranchesFocus::Graph,
            BranchesPane::Files => BranchesFocus::Files,
            BranchesPane::FileDiff => BranchesFocus::FileDiff,
        },
        rows,
        info,
        files: file_rows(
            branches.files(),
            branches.file_cursor(),
            viewports.branches_files,
        ),
        diff_title: branches.selected_file().map_or_else(
            || "Diff".to_string(),
            |f: &ChangedFile| format!("Diff {}", f.path),
        ),
        diff: diff_rows(
            branches.diff_rows(),
            branches.diff_cursor(),
            None,
            viewports.branches_diff,
        ),
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
    let focus: ui::views::diff::DiffFocus = match (diff.pane(), diff.side()) {
        (DiffPane::FileDiff, _) => ui::views::diff::DiffFocus::FileDiff,
        (DiffPane::Files, Side::Unstaged) => ui::views::diff::DiffFocus::Unstaged,
        (DiffPane::Files, Side::Staged) => ui::views::diff::DiffFocus::Staged,
    };
    ui::views::diff::DiffView {
        focus,
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
        diff_title: diff.selected_file().map_or_else(
            || "Diff".to_string(),
            |f: &ChangedFile| format!("Diff {}", f.path),
        ),
        diff: diff_rows(
            diff.rows(),
            diff.diff_cursor(),
            diff.selection(),
            viewports.diff_file,
        ),
        message_title: if diff.amend() {
            "Amend message (previous commit)".to_string()
        } else {
            "Commit message".to_string()
        },
        message: diff.message().to_string(),
        no_changes: !diff.status().has_changes(),
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
                label: file_label(file),
                selected: index == cursor.selected(),
            },
        )
        .collect()
}

/// A status letter and the path, for example `M src/main.rs` or `R old -> new`.
fn file_label(file: &ChangedFile) -> String {
    let letter: char = match file.kind {
        ChangeKind::Modified => 'M',
        ChangeKind::Added => 'A',
        ChangeKind::Deleted => 'D',
        ChangeKind::Renamed => 'R',
        ChangeKind::Copied => 'C',
        ChangeKind::TypeChanged => 'T',
        ChangeKind::Untracked => '?',
        ChangeKind::Conflict => 'U',
    };
    let path: String = match &file.original_path {
        Some(original) => format!("{original} -> {}", file.path),
        None => file.path.clone(),
    };
    let mode: &str = if file.mode_changed { " (mode)" } else { "" };
    format!("{letter} {path}{mode}")
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
        RowKind::Note => K::Note,
    }
}

fn layer_name(layer: Layer) -> &'static str {
    match layer {
        Layer::Git => "git",
        Layer::Logic => "logic",
        Layer::Ui => "ui",
        Layer::Validation => "validation",
        Layer::Wiring => "wiring",
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
