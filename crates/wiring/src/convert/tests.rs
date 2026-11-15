use super::{
    diff_row_kind, event, file_label, graph_cell, input_mode, label_view, layer_name, log_view,
    logic_view, screen, ui_level, ui_view, viewports,
};
use logic::app::{App, Confirmation, Mode, Viewports};
use logic::event::Event as L;
use logic::event::ViewId as LV;
use logic::log::{Layer, LogEntry, LogLevel};
use logic::model::{ChangeKind, ChangedFile, LineKind, RowKind};
use ui::event::Event as U;
use ui::event::ViewId as UV;

struct NoPorts;

impl logic::ports::Ports for NoPorts {
    fn log(&self, _level: LogLevel, _message: &str) {}

    fn validate_search(&self, input: &str) -> Result<logic::ports::SearchText, String> {
        Ok(logic::ports::SearchText(input.to_string()))
    }

    fn status(&self) -> Result<logic::model::WorkStatus, logic::ports::Failed> {
        Err(logic::ports::Failed)
    }

    fn file_diff(
        &self,
        _file: &logic::model::ChangedFile,
        _side: logic::model::Side,
    ) -> Result<logic::model::FileDiff, logic::ports::Failed> {
        Err(logic::ports::Failed)
    }

    fn stage_file(&self, _file: &logic::model::ChangedFile) -> Result<(), logic::ports::Failed> {
        Err(logic::ports::Failed)
    }

    fn unstage_file(&self, _file: &logic::model::ChangedFile) -> Result<(), logic::ports::Failed> {
        Err(logic::ports::Failed)
    }

    fn discard_file(&self, _file: &logic::model::ChangedFile) -> Result<(), logic::ports::Failed> {
        Err(logic::ports::Failed)
    }

    fn apply_to_index(&self, _patch: &str, _reverse: bool) -> Result<(), logic::ports::Failed> {
        Err(logic::ports::Failed)
    }

    fn has_commits(&self) -> Result<bool, logic::ports::Failed> {
        Err(logic::ports::Failed)
    }

    fn last_commit_message(&self) -> Result<String, logic::ports::Failed> {
        Err(logic::ports::Failed)
    }

    fn edit_message(&self, _text: &str) -> Result<String, logic::ports::Failed> {
        Err(logic::ports::Failed)
    }

    fn validate_commit_message(&self, _text: &str) -> Result<logic::ports::CommitMessage, String> {
        Err(String::new())
    }

    fn commit(
        &self,
        _message: &logic::ports::CommitMessage,
        _amend: bool,
    ) -> Result<(), logic::ports::Failed> {
        Err(logic::ports::Failed)
    }

    fn commits(
        &self,
        _skip: usize,
        _count: usize,
    ) -> Result<Vec<logic::model::Commit>, logic::ports::Failed> {
        Err(logic::ports::Failed)
    }

    fn refs(&self) -> Result<logic::model::Refs, logic::ports::Failed> {
        Err(logic::ports::Failed)
    }

    fn validate_branch_name(&self, _input: &str) -> Result<logic::ports::BranchName, String> {
        Err(String::new())
    }

    fn validate_checkout_choice(&self, _input: &str, _count: usize) -> Result<usize, String> {
        Err(String::new())
    }

    fn create_branch(
        &self,
        _name: &logic::ports::BranchName,
        _at: &str,
    ) -> Result<(), logic::ports::Failed> {
        Err(logic::ports::Failed)
    }

    fn delete_branch(&self, _name: &str) -> Result<(), logic::ports::Failed> {
        Err(logic::ports::Failed)
    }

    fn remote_update(&self) -> Result<(), logic::ports::Failed> {
        Err(logic::ports::Failed)
    }

    fn fetch_all_prune(&self) -> Result<(), logic::ports::Failed> {
        Err(logic::ports::Failed)
    }

    fn reset_keep(&self, _hash: &str) -> Result<(), logic::ports::Failed> {
        Err(logic::ports::Failed)
    }

    fn cherry_pick(&self, _hash: &str) -> Result<(), logic::ports::Failed> {
        Err(logic::ports::Failed)
    }

    fn rebase(
        &self,
        _branch: &str,
        _onto: &str,
        _interactive: bool,
    ) -> Result<(), logic::ports::Failed> {
        Err(logic::ports::Failed)
    }

    fn checkout(&self, _branch: &str) -> Result<(), logic::ports::Failed> {
        Err(logic::ports::Failed)
    }

    fn commit_message(&self, _hash: &str) -> Result<String, logic::ports::Failed> {
        Err(logic::ports::Failed)
    }

    fn commit_files(
        &self,
        _hash: &str,
    ) -> Result<Vec<logic::model::ChangedFile>, logic::ports::Failed> {
        Err(logic::ports::Failed)
    }

    fn commit_file_diff(
        &self,
        _hash: &str,
        _file: &logic::model::ChangedFile,
    ) -> Result<logic::model::FileDiff, logic::ports::Failed> {
        Err(logic::ports::Failed)
    }
}

#[test]
fn every_event_converts() {
    let cases: [(U, L); 34] = [
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
        (U::ResetBranch, L::ResetBranch),
        (U::DeleteBranch, L::DeleteBranch),
        (U::DiscardFile, L::DiscardFile),
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

#[test]
fn modes_convert() {
    let mut app: App = App::new();
    assert_eq!(input_mode(app.mode()), ui::event::InputMode::Normal);
    app.handle(L::StartSearch, &NoPorts);
    assert_eq!(input_mode(app.mode()), ui::event::InputMode::Text);
    assert_eq!(
        input_mode(&Mode::Confirm(Confirmation::Quit)),
        ui::event::InputMode::Confirm
    );
}

#[test]
fn layers_and_levels() {
    let layers: [(Layer, &str); 5] = [
        (Layer::Git, "git"),
        (Layer::Logic, "logic"),
        (Layer::Ui, "ui"),
        (Layer::Validation, "validation"),
        (Layer::Wiring, "wiring"),
    ];
    for (layer, name) in layers {
        assert_eq!(layer_name(layer), name);
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
fn log_screen_shows_visible_rows_and_status() {
    let mut app: App = App::new();
    app.set_viewports(Viewports {
        log: 2,
        ..Viewports::default()
    });
    for message in ["a", "b", "c"] {
        app.push_log(&LogEntry {
            time: 5,
            layer: Layer::Git,
            level: LogLevel::Info,
            message: message.to_string(),
        });
    }
    app.handle(L::SwitchView(LV::Log), &NoPorts);
    app.handle(L::StartSearch, &NoPorts);
    app.handle(L::Input('a'), &NoPorts);
    let prompt: ui::screen::Screen = screen(&app);
    assert_eq!(
        prompt.status,
        ui::screen::Status::Prompt {
            label: "/".to_string(),
            text: "a".to_string(),
            problem: None,
        }
    );
    assert!(matches!(prompt.body, ui::screen::Body::Log(_)));
    let shown: Vec<(String, bool)> = log_view(&app)
        .rows
        .iter()
        .map(|r: &ui::views::log::LogRow| (r.message.clone(), r.selected))
        .collect();
    assert_eq!(
        shown,
        vec![("b".to_string(), false), ("c".to_string(), true)]
    );
    app.handle(L::InputSubmit, &NoPorts);
    assert_eq!(
        screen(&app).status,
        ui::screen::Status::Note("search wrapped".to_string())
    );
    app.handle(L::StartSearch, &NoPorts);
    app.handle(L::Input('z'), &NoPorts);
    app.handle(L::InputSubmit, &NoPorts);
    assert_eq!(
        screen(&app).status,
        ui::screen::Status::Note("Pattern not found: z".to_string())
    );
    app.handle(L::SwitchView(LV::Diff), &NoPorts);
    assert!(matches!(screen(&app).body, ui::screen::Body::Diff(_)));
}

#[test]
fn file_labels() {
    let file = |kind: ChangeKind| ChangedFile {
        path: "p".to_string(),
        original_path: None,
        kind,
        mode_changed: false,
    };
    let cases: [(ChangeKind, &str); 8] = [
        (ChangeKind::Modified, "M p"),
        (ChangeKind::Added, "A p"),
        (ChangeKind::Deleted, "D p"),
        (ChangeKind::Renamed, "R p"),
        (ChangeKind::Copied, "C p"),
        (ChangeKind::TypeChanged, "T p"),
        (ChangeKind::Untracked, "? p"),
        (ChangeKind::Conflict, "U p"),
    ];
    for (kind, label) in cases {
        assert_eq!(file_label(&file(kind)), label);
    }
    let renamed: ChangedFile = ChangedFile {
        original_path: Some("o".to_string()),
        mode_changed: true,
        ..file(ChangeKind::Renamed)
    };
    assert_eq!(file_label(&renamed), "R o -> p (mode)");
}

#[test]
fn diff_row_kinds() {
    use ui::views::diff_text::DiffRowKind as K;
    let cases: [(RowKind, K); 6] = [
        (RowKind::HunkHeader, K::HunkHeader),
        (RowKind::Line(LineKind::Context), K::Context),
        (RowKind::Line(LineKind::Added), K::Added),
        (RowKind::Line(LineKind::Removed), K::Removed),
        (RowKind::Line(LineKind::NoNewline), K::NoNewline),
        (RowKind::Note, K::Note),
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
}
