use std::collections::VecDeque;

use super::{
    branches_focus, commit_info, commit_rows, diff_focus, diff_row_kind, event, file_change,
    file_rows, graph_cell, input_mode, label_view, log_view, logic_view, screen, status,
    ui_branch_name_problem, ui_info, ui_input_problem, ui_layer, ui_level, ui_operation,
    ui_problem, ui_size, ui_view, viewports,
};
use logic::app::{App, BranchesPane, Confirmation, DiffPane, Mode, Prompt, PromptKind, Viewports};
use logic::cursor::Cursor;
use logic::event::Event as L;
use logic::event::ViewId as LV;
use logic::graph::{Cell, GraphRow, Shape};
use logic::log::{Layer, LogEntry, LogLevel};
use logic::model::{ChangeKind, ChangedFile, Commit, Label, LineKind, RowKind, Side};
use logic::note::BranchNameProblem as LRule;
use logic::note::Info as LInfo;
use logic::note::InputProblem as LInput;
use logic::note::Note;
use logic::note::Operation as LOperation;
use logic::note::Problem as LProblem;
use ui::event::Event as U;
use ui::event::ViewId as UV;
use ui::screen::Status;
use ui::status::BranchNameProblem as URule;
use ui::status::Info as UInfo;
use ui::status::InputProblem as UInput;
use ui::status::Operation as UOperation;
use ui::status::Problem as UProblem;
use ui::status::PromptKind as UPrompt;
use ui::status::Question;
use ui::views::branches::{BranchesFocus, CommitInfo, GraphShape, LabelView};
use ui::views::diff::DiffFocus;

#[test]
fn every_event_converts() {
    let cases: [(U, L); 37] = [
        (U::SwitchView(UV::Log), L::SwitchView(LV::Log)),
        (U::Quit, L::Quit),
        (U::Yes, L::Yes),
        (U::No, L::No),
        (U::Reload, L::Reload),
        (U::MoveDown, L::MoveDown),
        (U::MoveUp, L::MoveUp),
        (U::Top, L::Top),
        (U::Bottom, L::Bottom),
        (U::NextPane, L::NextPane),
        (U::StartSearch, L::StartSearch),
        (U::SearchNext, L::SearchNext),
        (U::SearchPrevious, L::SearchPrevious),
        (U::Cancel, L::Cancel),
        (U::Activate, L::Activate),
        (U::CreateBranch, L::CreateBranch),
        (U::Checkout, L::Checkout),
        (U::CherryPick, L::CherryPick),
        (U::RemoteUpdate, L::RemoteUpdate),
        (U::FetchAllPrune, L::FetchAllPrune),
        (U::NextMainBranch, L::NextMainBranch),
        (U::ShowCommit, L::ShowCommit),
        (U::ResetBranch, L::ResetBranch),
        (U::DeleteBranch, L::DeleteBranch),
        (U::DiscardFile, L::DiscardFile),
        (U::StageAll, L::StageAll),
        (U::Rebase, L::Rebase),
        (U::InteractiveRebase, L::InteractiveRebase),
        (U::SwitchList, L::SwitchList),
        (U::StartSelection, L::StartSelection),
        (U::EditMessage, L::EditMessage),
        (U::Commit, L::Commit),
        (U::ToggleAmend, L::ToggleAmend),
        (U::Input('x'), L::Input('x')),
        (U::InputBackspace, L::InputBackspace),
        (U::InputSubmit, L::InputSubmit),
        (U::InputCancel, L::InputCancel),
    ];
    for (from, to) in cases {
        assert_eq!(event(from), to);
    }
}

#[test]
fn views_convert_both_ways() {
    for (u, l) in [
        (UV::Branches, LV::Branches),
        (UV::Diff, LV::Diff),
        (UV::Log, LV::Log),
    ] {
        assert_eq!(logic_view(u), l);
        assert_eq!(ui_view(l), u);
    }
}

fn prompt(kind: PromptKind, text: &str) -> Mode {
    Mode::Prompt(Prompt {
        kind,
        text: text.to_string(),
    })
}

/// A cursor on item `selected` of `len` items, scrolled so that it shows in `height` rows.
fn cursor_on(selected: usize, len: usize, height: usize) -> Cursor {
    let mut cursor: Cursor = Cursor::default();
    cursor.select(selected, len, height);
    cursor
}

fn changed(path: &str, kind: ChangeKind) -> ChangedFile {
    ChangedFile {
        path: path.to_string(),
        original_path: None,
        kind,
        mode_changed: false,
    }
}

fn commit(hash: &str, subject: &str) -> Commit {
    Commit {
        hash: hash.to_string(),
        parents: vec![],
        author: "Ada".to_string(),
        author_email: "ada@example.com".to_string(),
        date: "2025-01-02 03:04".to_string(),
        committer: "Cy".to_string(),
        committer_email: "cy@example.com".to_string(),
        committer_date: "2025-01-03 04:05".to_string(),
        subject: subject.to_string(),
    }
}

#[test]
fn modes_convert() {
    assert_eq!(input_mode(&Mode::Normal), ui::event::InputMode::Normal);
    assert_eq!(
        input_mode(&prompt(PromptKind::Search, "")),
        ui::event::InputMode::Text
    );
    assert_eq!(
        input_mode(&Mode::Confirm(Confirmation::Quit)),
        ui::event::InputMode::Confirm
    );
}

#[test]
fn status_line_for_notes_and_prompts() {
    assert_eq!(status(&Mode::Normal, &Note::None), Status::Empty);
    assert_eq!(
        status(&Mode::Normal, &Note::Info(LInfo::SearchWrapped)),
        Status::Note(UInfo::SearchWrapped)
    );
    assert_eq!(
        status(&Mode::Normal, &Note::Problem(LProblem::ChoiceNotListed)),
        Status::Problem(UProblem::ChoiceNotListed)
    );
    assert_eq!(
        status(
            &prompt(PromptKind::Search, "a"),
            &Note::Info(LInfo::SearchWrapped)
        ),
        Status::Prompt {
            kind: UPrompt::Search,
            text: "a".to_string(),
            problem: None,
        }
    );
    assert_eq!(
        status(
            &prompt(PromptKind::Search, "a"),
            &Note::Problem(LProblem::ChoiceNotListed)
        ),
        Status::Prompt {
            kind: UPrompt::Search,
            text: "a".to_string(),
            problem: Some(UProblem::ChoiceNotListed),
        }
    );
    assert_eq!(
        status(&prompt(PromptKind::Search, ""), &Note::None),
        Status::Prompt {
            kind: UPrompt::Search,
            text: String::new(),
            problem: None,
        }
    );
}

#[test]
fn prompt_kinds_convert() {
    let branches: Vec<String> = vec!["a".to_string(), "b".to_string()];
    let cases: [(PromptKind, UPrompt); 6] = [
        (PromptKind::Search, UPrompt::Search),
        (
            PromptKind::BranchName {
                at: "h".to_string(),
            },
            UPrompt::BranchName,
        ),
        (
            PromptKind::CheckoutChoice {
                branches: branches.clone(),
            },
            UPrompt::CheckoutChoice(branches.clone()),
        ),
        (
            PromptKind::DeleteBranch {
                branches: branches.clone(),
            },
            UPrompt::DeleteBranch(branches.clone()),
        ),
        (
            PromptKind::RebaseBranch {
                branches: branches.clone(),
                interactive: true,
            },
            UPrompt::RebaseBranch {
                branches: branches.clone(),
                interactive: true,
            },
        ),
        (
            PromptKind::RebaseOnto {
                branch: "t".to_string(),
                onto: branches.clone(),
                interactive: false,
            },
            UPrompt::RebaseOnto {
                branch: "t".to_string(),
                onto: branches,
                interactive: false,
            },
        ),
    ];
    for (kind, expected) in cases {
        assert_eq!(
            status(&prompt(kind, ""), &Note::None),
            Status::Prompt {
                kind: expected,
                text: String::new(),
                problem: None,
            }
        );
    }
}

#[test]
fn confirmations_become_questions() {
    let cases: [(Confirmation, Question); 8] = [
        (Confirmation::Quit, Question::Quit),
        (
            Confirmation::DeleteBranch {
                name: "old".to_string(),
            },
            Question::DeleteBranch {
                name: "old".to_string(),
            },
        ),
        (
            Confirmation::ResetBranch {
                branch: "main".to_string(),
                hash: "1234567890".to_string(),
            },
            Question::ResetBranch {
                branch: "main".to_string(),
                hash: "1234567890".to_string(),
            },
        ),
        (
            Confirmation::ResetExistingBranch {
                branch: "topic".to_string(),
                hash: "1234567890".to_string(),
                checked_out: false,
            },
            Question::ResetExistingBranch {
                branch: "topic".to_string(),
                hash: "1234567890".to_string(),
            },
        ),
        (
            Confirmation::ResetExistingBranch {
                branch: "main".to_string(),
                hash: "1234567890".to_string(),
                checked_out: true,
            },
            Question::ResetExistingBranch {
                branch: "main".to_string(),
                hash: "1234567890".to_string(),
            },
        ),
        (
            Confirmation::DeleteUntracked {
                file: changed("new.txt", ChangeKind::Untracked),
            },
            Question::DeleteUntracked {
                path: "new.txt".to_string(),
            },
        ),
        (
            Confirmation::DiscardChanges {
                file: changed("a.txt", ChangeKind::Modified),
            },
            Question::DiscardChanges {
                path: "a.txt".to_string(),
            },
        ),
        (
            Confirmation::StageWithConflictMarkers {
                file: changed("c.txt", ChangeKind::Conflict),
            },
            Question::StageWithConflictMarkers {
                path: "c.txt".to_string(),
            },
        ),
    ];
    for (confirmation, expected) in cases {
        assert_eq!(
            status(
                &Mode::Confirm(confirmation),
                &Note::Problem(LProblem::ChoiceNotListed)
            ),
            Status::Question(expected)
        );
    }
}

#[test]
fn infos_without_data_convert() {
    let cases: [(LInfo, UInfo); 13] = [
        (LInfo::NoCommitSelected, UInfo::NoCommitSelected),
        (
            LInfo::NoOtherBranchToRebaseOnto,
            UInfo::NoOtherBranchToRebaseOnto,
        ),
        (LInfo::UpdatedRemotes, UInfo::UpdatedRemotes),
        (LInfo::FetchedAndPruned, UInfo::FetchedAndPruned),
        (LInfo::NoMainOrMasterBranch, UInfo::NoMainOrMasterBranch),
        (
            LInfo::OnlyWholeFilesCanBeStagedHere,
            UInfo::OnlyWholeFilesCanBeStagedHere,
        ),
        (
            LInfo::OnlyUnstagedChangesCanBeDiscarded,
            UInfo::OnlyUnstagedChangesCanBeDiscarded,
        ),
        (LInfo::NothingToStage, UInfo::NothingToStage),
        (
            LInfo::EditingMessageCancelled,
            UInfo::EditingMessageCancelled,
        ),
        (
            LInfo::InteractiveRebaseCancelled,
            UInfo::InteractiveRebaseCancelled,
        ),
        (LInfo::NoChangedLinesSelected, UInfo::NoChangedLinesSelected),
        (LInfo::NoPreviousSearch, UInfo::NoPreviousSearch),
        (LInfo::SearchWrapped, UInfo::SearchWrapped),
    ];
    for (from, to) in cases {
        assert_eq!(ui_info(&from), to);
    }
}

#[test]
fn branch_infos_with_data_convert() {
    let cases: [(LInfo, UInfo); 7] = [
        (
            LInfo::CreatedBranch {
                name: "name".to_string(),
            },
            UInfo::CreatedBranch {
                name: "name".to_string(),
            },
        ),
        (
            LInfo::BranchAlreadyPointsTo {
                branch: "topic".to_string(),
                hash: "1234567890".to_string(),
            },
            UInfo::BranchAlreadyPointsTo {
                branch: "topic".to_string(),
                hash: "1234567890".to_string(),
            },
        ),
        (
            LInfo::DeletedBranch {
                name: "name".to_string(),
            },
            UInfo::DeletedBranch {
                name: "name".to_string(),
            },
        ),
        (
            LInfo::Rebased {
                branch: "branch".to_string(),
                onto: "onto".to_string(),
            },
            UInfo::Rebased {
                branch: "branch".to_string(),
                onto: "onto".to_string(),
            },
        ),
        (
            LInfo::RebasedInteractively {
                branch: "branch".to_string(),
                onto: "onto".to_string(),
            },
            UInfo::RebasedInteractively {
                branch: "branch".to_string(),
                onto: "onto".to_string(),
            },
        ),
        (
            LInfo::CheckedOut {
                branch: "branch".to_string(),
            },
            UInfo::CheckedOut {
                branch: "branch".to_string(),
            },
        ),
        (
            LInfo::ResetBranch {
                branch: "branch".to_string(),
                hash: "hash".to_string(),
            },
            UInfo::ResetBranch {
                branch: "branch".to_string(),
                hash: "hash".to_string(),
            },
        ),
    ];
    for (from, to) in cases {
        assert_eq!(ui_info(&from), to);
    }
}

#[test]
fn other_infos_with_data_convert() {
    let cases: [(LInfo, UInfo); 4] = [
        (
            LInfo::CherryPicked {
                hash: "hash".to_string(),
            },
            UInfo::CherryPicked {
                hash: "hash".to_string(),
            },
        ),
        (
            LInfo::DeletedFile {
                path: "path".to_string(),
            },
            UInfo::DeletedFile {
                path: "path".to_string(),
            },
        ),
        (
            LInfo::DiscardedChanges {
                path: "path".to_string(),
            },
            UInfo::DiscardedChanges {
                path: "path".to_string(),
            },
        ),
        (
            LInfo::PatternNotFound {
                term: "term".to_string(),
            },
            UInfo::PatternNotFound {
                term: "term".to_string(),
            },
        ),
    ];
    for (from, to) in cases {
        assert_eq!(ui_info(&from), to);
    }
}

#[test]
fn problems_convert() {
    let cases: [(LProblem, UProblem); 15] = [
        (LProblem::ChoiceNotListed, UProblem::ChoiceNotListed),
        (
            LProblem::NoLocalBranchToCheckOut,
            UProblem::NoLocalBranchToCheckOut,
        ),
        (
            LProblem::NoLocalBranchToDelete,
            UProblem::NoLocalBranchToDelete,
        ),
        (
            LProblem::CheckedOutBranchCannotBeDeleted,
            UProblem::CheckedOutBranchCannotBeDeleted,
        ),
        (
            LProblem::NoLocalBranchToRebase,
            UProblem::NoLocalBranchToRebase,
        ),
        (
            LProblem::NoBranchCheckedOutToReset,
            UProblem::NoBranchCheckedOutToReset,
        ),
        (
            LProblem::UncommittedChangesBeforeReset,
            UProblem::UncommittedChangesBeforeReset,
        ),
        (
            LProblem::UncommittedChangesBeforeRebase,
            UProblem::UncommittedChangesBeforeRebase,
        ),
        (LProblem::RebaseStopped, UProblem::RebaseStopped),
        (
            LProblem::MergeConflict {
                path: "path".to_string(),
            },
            UProblem::MergeConflict {
                path: "path".to_string(),
            },
        ),
        (
            LProblem::MergeConflictsBeforeStagingAll,
            UProblem::MergeConflictsBeforeStagingAll,
        ),
        (LProblem::NothingToAmend, UProblem::NothingToAmend),
        (
            LProblem::AmendedCommitWouldBeEmpty,
            UProblem::AmendedCommitWouldBeEmpty,
        ),
        (
            LProblem::Failed(LOperation::Committing),
            UProblem::Failed(UOperation::Committing),
        ),
        (
            LProblem::Invalid(LInput::EmptySearch),
            UProblem::Invalid(UInput::EmptySearch),
        ),
    ];
    for (from, to) in cases {
        assert_eq!(ui_problem(&from), to);
    }
}

#[test]
fn operations_convert() {
    let cases: [(LOperation, UOperation); 23] = [
        (LOperation::ReloadingCommits, UOperation::ReloadingCommits),
        (LOperation::LoadingCommit, UOperation::LoadingCommit),
        (LOperation::LoadingDiff, UOperation::LoadingDiff),
        (
            LOperation::LoadingMoreCommits,
            UOperation::LoadingMoreCommits,
        ),
        (LOperation::CreatingBranch, UOperation::CreatingBranch),
        (LOperation::DeletingBranch, UOperation::DeletingBranch),
        (LOperation::Rebasing, UOperation::Rebasing),
        (LOperation::CheckingOut, UOperation::CheckingOut),
        (LOperation::UpdatingRemotes, UOperation::UpdatingRemotes),
        (
            LOperation::FetchingAndPruning,
            UOperation::FetchingAndPruning,
        ),
        (LOperation::ShowingCommit, UOperation::ShowingCommit),
        (LOperation::ReadingStatus, UOperation::ReadingStatus),
        (LOperation::ReadingBranches, UOperation::ReadingBranches),
        (
            LOperation::CheckingConflictMarkers,
            UOperation::CheckingConflictMarkers,
        ),
        (LOperation::ResettingBranch, UOperation::ResettingBranch),
        (LOperation::CherryPicking, UOperation::CherryPicking),
        (LOperation::ReloadingChanges, UOperation::ReloadingChanges),
        (LOperation::DeletingFile, UOperation::DeletingFile),
        (LOperation::DiscardingChanges, UOperation::DiscardingChanges),
        (LOperation::Staging, UOperation::Staging),
        (LOperation::EditingMessage, UOperation::EditingMessage),
        (LOperation::ReadingLastCommit, UOperation::ReadingLastCommit),
        (LOperation::Committing, UOperation::Committing),
    ];
    for (from, to) in cases {
        assert_eq!(ui_operation(from), to);
    }
}

#[test]
fn input_problems_convert() {
    let cases: [(LInput, UInput); 5] = [
        (LInput::NotANumber, UInput::NotANumber),
        (
            LInput::OutOfRange { count: 3 },
            UInput::OutOfRange { count: 3 },
        ),
        (LInput::EmptySearch, UInput::EmptySearch),
        (LInput::EmptyCommitMessage, UInput::EmptyCommitMessage),
        (
            LInput::BranchName(LRule::DoubleDot),
            UInput::BranchName(URule::DoubleDot),
        ),
    ];
    for (from, to) in cases {
        assert_eq!(ui_input_problem(&from), to);
    }
}

#[test]
fn branch_name_problems_convert() {
    let cases: [(LRule, URule); 10] = [
        (LRule::Empty, URule::Empty),
        (LRule::Reserved, URule::Reserved),
        (LRule::StartsWithDash, URule::StartsWithDash),
        (LRule::BadSlash, URule::BadSlash),
        (LRule::ComponentStartsWithDot, URule::ComponentStartsWithDot),
        (LRule::ComponentEndsWithLock, URule::ComponentEndsWithLock),
        (LRule::EndsWithDot, URule::EndsWithDot),
        (LRule::DoubleDot, URule::DoubleDot),
        (LRule::AtBrace, URule::AtBrace),
        (
            LRule::ForbiddenCharacter(' '),
            URule::ForbiddenCharacter(' '),
        ),
    ];
    for (from, to) in cases {
        assert_eq!(ui_branch_name_problem(from), to);
    }
}

#[test]
fn layers_and_levels() {
    use ui::views::log::Layer as U;
    let layers: [(Layer, U); 5] = [
        (Layer::Git, U::Git),
        (Layer::Logic, U::Logic),
        (Layer::Ui, U::Ui),
        (Layer::Validation, U::Validation),
        (Layer::Wiring, U::Wiring),
    ];
    for (layer, expected) in layers {
        assert_eq!(ui_layer(layer), expected);
    }
    let levels: [(LogLevel, ui::log::LogLevel); 4] = [
        (LogLevel::Error, ui::log::LogLevel::Error),
        (LogLevel::Warn, ui::log::LogLevel::Warn),
        (LogLevel::Info, ui::log::LogLevel::Info),
        (LogLevel::Debug, ui::log::LogLevel::Debug),
    ];
    for (level, ui) in levels {
        assert_eq!(ui_level(level), ui);
    }
}

#[test]
fn terminal_size_converts() {
    assert_eq!(
        ui_size(terminal::size::Size {
            rows: 24,
            columns: 80
        }),
        ui::layout::Size {
            width: 80,
            height: 24
        }
    );
}

#[test]
fn viewports_follow_layout() {
    let size: ui::layout::Size = ui::layout::Size {
        width: 80,
        height: 24,
    };
    assert_eq!(
        viewports(size),
        Viewports {
            branches_graph: 6,
            branches_files: 14,
            branches_diff: 7,
            log: 21,
            diff_unstaged: 10,
            diff_staged: 10,
            diff_file: 13,
        }
    );
}

#[test]
fn startup_screen() {
    let s: ui::screen::Screen = screen(&App::new());
    assert_eq!(s.tabs.active, UV::Branches);
    assert_eq!(s.status, ui::screen::Status::Empty);
    assert!(matches!(s.body, ui::screen::Body::Branches(_)));
}

#[test]
fn log_view_shows_the_visible_entries() {
    let entries: VecDeque<LogEntry> = ["a", "b", "c"]
        .iter()
        .map(|message: &&str| LogEntry {
            time: 5,
            layer: Layer::Git,
            level: LogLevel::Info,
            message: (*message).to_string(),
        })
        .collect();
    let rows: Vec<ui::views::log::LogRow> = log_view(&entries, cursor_on(2, 3, 2), 2).rows;
    assert_eq!(
        rows,
        vec![
            ui::views::log::LogRow {
                time: 5,
                layer: ui::views::log::Layer::Git,
                level: ui::log::LogLevel::Info,
                message: "b".to_string(),
                selected: false,
            },
            ui::views::log::LogRow {
                time: 5,
                layer: ui::views::log::Layer::Git,
                level: ui::log::LogLevel::Info,
                message: "c".to_string(),
                selected: true,
            },
        ]
    );
}

#[test]
fn commit_rows_show_the_visible_commits() {
    let commits: Vec<Commit> = vec![
        commit("c2", "two"),
        commit("c1", "one"),
        commit("c0", "zero"),
    ];
    let graph: Vec<GraphRow> = ["c2", "c1", "c0"]
        .iter()
        .map(|_| GraphRow {
            cells: vec![Cell {
                shape: Shape::Commit,
                lane: 0,
                link_right: None,
            }],
        })
        .collect();
    let main: Vec<Label> = vec![Label::Local {
        name: "main".to_string(),
        head: true,
    }];
    let labels = |hash: &str| -> &[Label] { if hash == "c1" { &main } else { &[] } };
    let rows: Vec<ui::views::branches::CommitRow> =
        commit_rows(&commits, &graph, labels, Some("c1"), cursor_on(1, 3, 2), 2);
    let shown: Vec<(&str, bool, bool, usize)> = rows
        .iter()
        .map(|r: &ui::views::branches::CommitRow| {
            (r.subject.as_str(), r.selected, r.head, r.labels.len())
        })
        .collect();
    assert_eq!(
        shown,
        vec![("two", false, false, 0), ("one", true, true, 1)]
    );
    assert_eq!(rows[1].cells[0].shape, GraphShape::Commit);
    assert_eq!(
        rows[1].labels,
        vec![LabelView::Local {
            name: "main".to_string(),
            head: true
        }]
    );
    assert_eq!(
        (rows[0].author.as_str(), rows[0].date.as_str()),
        ("Ada", "2025-01-02 03:04")
    );
}

#[test]
fn commit_info_has_every_field_and_the_message_lines() {
    assert_eq!(
        commit_info(&commit("abc", "s"), Some(2), "Subject\n\nBody\n"),
        CommitInfo {
            hash: "abc".to_string(),
            lane: Some(2),
            author: "Ada".to_string(),
            author_email: "ada@example.com".to_string(),
            author_date: "2025-01-02 03:04".to_string(),
            committer: "Cy".to_string(),
            committer_email: "cy@example.com".to_string(),
            committer_date: "2025-01-03 04:05".to_string(),
            message: vec!["Subject".to_string(), String::new(), "Body".to_string()],
        }
    );
}

#[test]
fn focus() {
    assert_eq!(branches_focus(BranchesPane::Graph), BranchesFocus::Graph);
    assert_eq!(branches_focus(BranchesPane::Files), BranchesFocus::Files);
    assert_eq!(
        branches_focus(BranchesPane::FileDiff),
        BranchesFocus::FileDiff
    );
    assert_eq!(
        diff_focus(DiffPane::FileDiff, Side::Staged),
        DiffFocus::FileDiff
    );
    assert_eq!(
        diff_focus(DiffPane::Files, Side::Unstaged),
        DiffFocus::Unstaged
    );
    assert_eq!(diff_focus(DiffPane::Files, Side::Staged), DiffFocus::Staged);
}

#[test]
fn file_changes_convert() {
    use ui::views::file_list::FileChange as F;
    let cases: [(ChangeKind, F); 8] = [
        (ChangeKind::Modified, F::Modified),
        (ChangeKind::Added, F::Added),
        (ChangeKind::Deleted, F::Deleted),
        (ChangeKind::Renamed, F::Renamed),
        (ChangeKind::Copied, F::Copied),
        (ChangeKind::TypeChanged, F::TypeChanged),
        (ChangeKind::Untracked, F::Untracked),
        (ChangeKind::Conflict, F::Conflict),
    ];
    for (kind, expected) in cases {
        assert_eq!(file_change(kind), expected);
    }
}

#[test]
fn file_rows_show_the_visible_files() {
    let renamed: ChangedFile = ChangedFile {
        path: "new".to_string(),
        original_path: Some("old".to_string()),
        kind: ChangeKind::Renamed,
        mode_changed: true,
    };
    let files: Vec<ChangedFile> = vec![changed("a", ChangeKind::Modified), renamed];
    assert_eq!(
        file_rows(&files, cursor_on(1, 2, 1), 1),
        vec![ui::views::file_list::FileRow {
            change: ui::views::file_list::FileChange::Renamed,
            path: "new".to_string(),
            original_path: Some("old".to_string()),
            mode_changed: true,
            selected: true,
        }]
    );
}

#[test]
fn diff_row_kinds() {
    use ui::views::diff_text::DiffRowKind as K;
    let cases: [(RowKind, K); 7] = [
        (RowKind::HunkHeader, K::HunkHeader),
        (RowKind::Line(LineKind::Context), K::Context),
        (RowKind::Line(LineKind::Added), K::Added),
        (RowKind::Line(LineKind::Removed), K::Removed),
        (RowKind::Line(LineKind::NoNewline), K::NoNewline),
        (RowKind::BinaryFile, K::BinaryFile),
        (RowKind::NoContentChanges, K::NoContentChanges),
    ];
    for (from, to) in cases {
        assert_eq!(diff_row_kind(from), to);
    }
}

#[test]
fn graph_cells_and_labels() {
    use logic::graph::{Cell, CommitSide, Shape};
    use logic::model::Label;
    use ui::views::branches::LabelView;
    use ui::views::branches::{CommitSide as S, GraphShape as G};
    let cases: [(Shape, G); 11] = [
        (Shape::Empty, G::Empty),
        (Shape::Commit, G::Commit),
        (Shape::Vertical, G::Vertical),
        (Shape::Horizontal, G::Horizontal),
        (Shape::Crossing, G::Crossing),
        (Shape::Join(CommitSide::Left), G::Join(S::Left)),
        (Shape::Join(CommitSide::Right), G::Join(S::Right)),
        (Shape::Fork(CommitSide::Left), G::Fork(S::Left)),
        (Shape::Fork(CommitSide::Right), G::Fork(S::Right)),
        (Shape::Tee(CommitSide::Left), G::Tee(S::Left)),
        (Shape::Tee(CommitSide::Right), G::Tee(S::Right)),
    ];
    for (shape, expected) in cases {
        let converted: ui::views::branches::GraphCell = graph_cell(&Cell {
            shape,
            lane: 3,
            link_right: Some(4),
        });
        assert_eq!(
            (converted.shape, converted.lane, converted.link_right),
            (expected, 3, Some(4))
        );
    }
    assert_eq!(
        label_view(&Label::Local {
            name: "m".to_string(),
            head: true
        }),
        LabelView::Local {
            name: "m".to_string(),
            head: true
        }
    );
    assert_eq!(
        label_view(&Label::Remote {
            remote: "o".to_string(),
            branch: "m".to_string()
        }),
        LabelView::Remote {
            remote: "o".to_string(),
            branch: "m".to_string()
        }
    );
    assert_eq!(
        label_view(&Label::Tag {
            name: "v".to_string()
        }),
        LabelView::Tag("v".to_string())
    );
    assert_eq!(label_view(&Label::RebaseHead), LabelView::RebaseHead);
}
