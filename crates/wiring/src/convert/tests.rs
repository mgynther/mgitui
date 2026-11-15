use std::collections::VecDeque;

use super::{
    branches_focus, commit_info, commit_rows, diff_focus, diff_row_kind, diff_title, event,
    file_label, graph_cell, input_mode, label_view, layer_name, log_view, logic_view, screen,
    status, ui_level, ui_view, viewports,
};
use logic::app::{
    App, BranchesPane, Confirmation, DiffPane, Mode, Note, Prompt, PromptKind, Viewports,
};
use logic::cursor::Cursor;
use logic::event::Event as L;
use logic::event::ViewId as LV;
use logic::graph::{Cell, GraphRow, Shape};
use logic::log::{Layer, LogEntry, LogLevel};
use logic::model::{ChangeKind, ChangedFile, Commit, Label, LineKind, RowKind, Side};
use ui::event::Event as U;
use ui::event::ViewId as UV;
use ui::screen::Status;
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
        status(&Mode::Normal, &Note::Info("i".to_string())),
        Status::Note("i".to_string())
    );
    assert_eq!(
        status(&Mode::Normal, &Note::Problem("p".to_string())),
        Status::Problem("p".to_string())
    );
    assert_eq!(
        status(
            &prompt(PromptKind::Search, "a"),
            &Note::Info("i".to_string())
        ),
        Status::Prompt {
            label: "/".to_string(),
            text: "a".to_string(),
            problem: None,
        }
    );
    assert_eq!(
        status(
            &prompt(PromptKind::Search, "a"),
            &Note::Problem("p".to_string())
        ),
        Status::Prompt {
            label: "/".to_string(),
            text: "a".to_string(),
            problem: Some("p".to_string()),
        }
    );
    assert_eq!(
        status(&prompt(PromptKind::Search, ""), &Note::None),
        Status::Prompt {
            label: "/".to_string(),
            text: String::new(),
            problem: None,
        }
    );
}

#[test]
fn prompt_labels() {
    let branches: Vec<String> = vec!["a".to_string(), "b".to_string()];
    let cases: [(PromptKind, &str); 7] = [
        (PromptKind::Search, "/"),
        (
            PromptKind::BranchName {
                at: "h".to_string(),
            },
            "New branch name: ",
        ),
        (
            PromptKind::CheckoutChoice {
                branches: branches.clone(),
            },
            "Check out (1 a, 2 b): ",
        ),
        (
            PromptKind::DeleteBranch {
                branches: branches.clone(),
            },
            "Delete which branch (1 a, 2 b): ",
        ),
        (
            PromptKind::RebaseBranch {
                branches: branches.clone(),
                interactive: false,
            },
            "Rebase which branch (1 a, 2 b): ",
        ),
        (
            PromptKind::RebaseBranch {
                branches: branches.clone(),
                interactive: true,
            },
            "Interactively rebase which branch (1 a, 2 b): ",
        ),
        (
            PromptKind::RebaseOnto {
                branch: "t".to_string(),
                onto: branches,
                interactive: false,
            },
            "Rebase t onto (1 a, 2 b): ",
        ),
    ];
    for (kind, label) in cases {
        assert_eq!(
            status(&prompt(kind, ""), &Note::None),
            Status::Prompt {
                label: label.to_string(),
                text: String::new(),
                problem: None,
            }
        );
    }
}

#[test]
fn confirmation_labels() {
    let cases: [(Confirmation, &str); 6] = [
        (Confirmation::Quit, "Quit? y/n"),
        (
            Confirmation::DeleteBranch {
                name: "old".to_string(),
            },
            "Delete branch old even if it is not merged? y/n",
        ),
        (
            Confirmation::ResetBranch {
                branch: "main".to_string(),
                hash: "1234567890".to_string(),
            },
            "Reset main to 1234567? y/n",
        ),
        (
            Confirmation::ResetBranch {
                branch: "main".to_string(),
                hash: "123".to_string(),
            },
            "Reset main to 123? y/n",
        ),
        (
            Confirmation::DiscardFile {
                file: changed("new.txt", ChangeKind::Untracked),
            },
            "Delete untracked file new.txt? y/n",
        ),
        (
            Confirmation::DiscardFile {
                file: changed("a.txt", ChangeKind::Modified),
            },
            "Discard unstaged changes in a.txt? y/n",
        ),
    ];
    for (confirmation, label) in cases {
        assert_eq!(
            status(
                &Mode::Confirm(confirmation),
                &Note::Problem("p".to_string())
            ),
            Status::Prompt {
                label: label.to_string(),
                text: String::new(),
                problem: None,
            }
        );
    }
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
                layer: "git".to_string(),
                level: ui::log::LogLevel::Info,
                message: "b".to_string(),
                selected: false,
            },
            ui::views::log::LogRow {
                time: 5,
                layer: "git".to_string(),
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
fn focus_and_titles() {
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
    assert_eq!(diff_title(None), "Diff");
    assert_eq!(
        diff_title(Some(&changed("a.txt", ChangeKind::Modified))),
        "Diff a.txt"
    );
}

#[test]
fn file_labels() {
    let file = |kind: ChangeKind| changed("p", kind);
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
