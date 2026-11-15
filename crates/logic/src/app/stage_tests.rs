use super::{App, Confirmation, DiffPane, Mode, Note, Viewports};
use crate::event::{Event, ViewId};
use crate::model::{ChangeKind, ChangedFile, DiffLine, FileDiff, Hunk, LineKind, Side, WorkStatus};
use crate::test_ports::{TestPorts, modified};

fn viewports() -> Viewports {
    Viewports {
        branches_graph: 5,
        branches_files: 5,
        branches_diff: 5,
        log: 5,
        diff_unstaged: 5,
        diff_staged: 5,
        diff_file: 10,
    }
}

/// `a` changed to `A` and `b` changed to `B`.
fn two_changes() -> FileDiff {
    let line = |kind: LineKind, text: &str| DiffLine {
        kind,
        text: text.to_string(),
    };
    FileDiff {
        header: vec!["--- a/f".to_string(), "+++ b/f".to_string()],
        hunks: vec![Hunk {
            header: "@@ -1,2 +1,2 @@".to_string(),
            old_start: 1,
            old_count: 2,
            new_start: 1,
            new_count: 2,
            lines: vec![
                line(LineKind::Removed, "a"),
                line(LineKind::Added, "A"),
                line(LineKind::Removed, "b"),
                line(LineKind::Added, "B"),
            ],
        }],
        binary: false,
    }
}

fn ports_with(unstaged: Vec<ChangedFile>, staged: Vec<ChangedFile>) -> TestPorts {
    let ports: TestPorts = TestPorts {
        diff: Some(two_changes()),
        ..TestPorts::new()
    };
    *ports.status.borrow_mut() = Some(WorkStatus { staged, unstaged });
    ports
}

fn app(ports: &TestPorts) -> App {
    let mut app: App = App::new();
    app.set_viewports(viewports());
    run(
        &mut app,
        ports,
        &[Event::SwitchView(ViewId::Diff), Event::Reload],
    );
    ports.clear();
    app
}

fn run(app: &mut App, ports: &TestPorts, events: &[Event]) {
    for event in events {
        app.handle(*event, ports);
    }
}

fn file(path: &str, kind: ChangeKind) -> ChangedFile {
    ChangedFile {
        kind,
        ..modified(path)
    }
}

#[test]
fn enter_stages_and_unstages_whole_files() {
    let ports: TestPorts = ports_with(vec![modified("u")], vec![modified("s")]);
    let mut app: App = app(&ports);
    run(&mut app, &ports, &[Event::Activate]);
    assert_eq!(
        ports.calls(),
        vec![
            "Info: stage u",
            "stage u",
            "refs",
            "commits 0 2000",
            "status",
            "diff Unstaged u"
        ]
    );
    ports.clear();
    run(&mut app, &ports, &[Event::SwitchList, Event::Activate]);
    assert_eq!(
        ports.calls(),
        vec![
            "diff Staged s",
            "Info: unstage s",
            "has commits",
            "unstage s",
            "refs",
            "commits 0 2000",
            "status",
            "diff Staged s"
        ]
    );
}

#[test]
fn unstaging_without_commits_removes_from_the_index() {
    let renamed: ChangedFile = ChangedFile {
        original_path: Some("old".to_string()),
        ..file("new", ChangeKind::Renamed)
    };
    let ports: TestPorts = TestPorts {
        has_commits: Some(false),
        ..ports_with(vec![], vec![renamed])
    };
    let mut no_commits_app: App = app(&ports);
    run(
        &mut no_commits_app,
        &ports,
        &[Event::SwitchList, Event::Activate],
    );
    assert_eq!(
        ports.calls()[..4],
        [
            "diff Staged old new",
            "Info: unstage new",
            "has commits",
            "remove from index old new"
        ]
    );
    let unknown: TestPorts = TestPorts {
        has_commits: None,
        ..ports_with(vec![], vec![modified("s")])
    };
    let mut unknown_app: App = app(&unknown);
    run(
        &mut unknown_app,
        &unknown,
        &[Event::SwitchList, Event::Activate],
    );
    assert!(
        !unknown
            .calls()
            .iter()
            .any(|c: &String| c.starts_with("unstage"))
    );
    assert_eq!(
        unknown_app.note(),
        &Note::Problem("Staging failed, see the Log view".to_string())
    );
}

#[test]
fn untracked_files_have_their_own_diff() {
    let ports: TestPorts = ports_with(vec![file("n", ChangeKind::Untracked)], vec![]);
    let mut app: App = app(&ports);
    run(&mut app, &ports, &[Event::Reload]);
    assert_eq!(ports.calls().last(), Some(&"diff untracked n".to_string()));
}

#[test]
fn enter_on_empty_list_does_nothing() {
    let ports: TestPorts = ports_with(vec![], vec![]);
    let mut app: App = app(&ports);
    run(&mut app, &ports, &[Event::Activate]);
    assert_eq!(ports.calls(), Vec::<String>::new());
}

#[test]
fn conflicts_are_refused_and_logged() {
    let ports: TestPorts = ports_with(vec![file("c", ChangeKind::Conflict)], vec![]);
    let mut app: App = app(&ports);
    run(&mut app, &ports, &[Event::Activate]);
    let message: &str = "c has a merge conflict; resolve it outside mgitui";
    assert_eq!(ports.calls(), vec![format!("Error: {message}")]);
    assert_eq!(app.note(), &Note::Problem(message.to_string()));
    run(&mut app, &ports, &[Event::NextPane, Event::StartSelection]);
    assert_eq!(app.diff().selection(), None);
}

#[test]
fn staging_failure_is_shown() {
    let ports: TestPorts = TestPorts {
        stage_fails: true,
        ..ports_with(vec![modified("u")], vec![])
    };
    let mut app: App = app(&ports);
    run(&mut app, &ports, &[Event::Activate]);
    assert_eq!(
        app.note(),
        &Note::Problem("Staging failed, see the Log view".to_string())
    );
}

#[test]
fn line_selection_stages_selected_lines() {
    let ports: TestPorts = ports_with(vec![modified("u")], vec![]);
    let mut app: App = app(&ports);
    run(
        &mut app,
        &ports,
        &[Event::NextPane, Event::MoveDown, Event::StartSelection],
    );
    assert_eq!(app.diff().pane(), DiffPane::FileDiff);
    run(&mut app, &ports, &[Event::MoveDown]);
    assert_eq!(app.diff().selection(), Some((1, 2)));
    run(&mut app, &ports, &[Event::Activate]);
    assert_eq!(
        ports.calls(),
        vec![
            "Info: stage lines of u",
            "apply reverse=false\n--- a/f\n+++ b/f\n@@ -1,2 +1,2 @@\n-a\n+A\n b\n",
            "refs",
            "commits 0 2000",
            "status",
            "diff Unstaged u",
        ]
    );
    assert_eq!(app.diff().selection(), None);
    assert_eq!(app.diff().diff_cursor().selected(), 2);
}

#[test]
fn enter_without_selection_stages_the_cursor_line_and_unstages_in_reverse() {
    let ports: TestPorts = ports_with(vec![], vec![modified("s")]);
    let mut app: App = app(&ports);
    run(
        &mut app,
        &ports,
        &[Event::SwitchList, Event::NextPane, Event::Bottom],
    );
    ports.clear();
    run(&mut app, &ports, &[Event::Activate]);
    assert_eq!(
        ports.calls()[..2],
        [
            "Info: unstage lines of s".to_string(),
            "apply reverse=true\n--- a/f\n+++ b/f\n@@ -1,1 +1,2 @@\n A\n+B\n".to_string(),
        ]
    );
}

#[test]
fn selecting_only_context_stages_nothing_and_escape_clears() {
    let ports: TestPorts = ports_with(vec![modified("u")], vec![]);
    let mut app: App = app(&ports);
    run(&mut app, &ports, &[Event::NextPane, Event::Activate]);
    assert_eq!(
        app.note(),
        &Note::Info("No changed lines selected".to_string())
    );
    run(
        &mut app,
        &ports,
        &[Event::StartSelection, Event::MoveDown, Event::Cancel],
    );
    assert_eq!(app.diff().selection(), None);
}

#[test]
fn only_whole_files_for_other_kinds() {
    let ports: TestPorts = ports_with(vec![file("n", ChangeKind::Untracked)], vec![]);
    let mut app: App = app(&ports);
    run(&mut app, &ports, &[Event::NextPane, Event::StartSelection]);
    assert_eq!(
        app.note(),
        &Note::Info("Only whole files can be staged here: use Enter in the file list".to_string())
    );
    run(&mut app, &ports, &[Event::Activate]);
    assert_eq!(ports.calls(), Vec::<String>::new());
}

#[test]
fn cursor_resets_when_staged_file_leaves_the_list() {
    let ports: TestPorts = ports_with(vec![modified("u"), modified("v")], vec![]);
    let mut app: App = app(&ports);
    run(&mut app, &ports, &[Event::NextPane, Event::Bottom]);
    *ports.status.borrow_mut() = Some(WorkStatus {
        unstaged: vec![modified("v")],
        staged: vec![modified("u")],
    });
    run(&mut app, &ports, &[Event::Activate]);
    assert_eq!(
        app.diff().selected_file().map(|f| f.path.clone()),
        Some("v".to_string())
    );
    assert_eq!(app.diff().diff_cursor().selected(), 0);
    assert_eq!(app.diff().side(), Side::Unstaged);
}

#[test]
fn d_discards_unstaged_changes_after_confirming() {
    let ports: TestPorts = ports_with(vec![modified("u")], vec![]);
    let mut app: App = app(&ports);
    run(&mut app, &ports, &[Event::DiscardFile]);
    assert_eq!(
        app.mode(),
        &Mode::Confirm(Confirmation::DiscardChanges {
            file: modified("u")
        })
    );
    run(&mut app, &ports, &[Event::No]);
    assert_eq!(app.mode(), &Mode::Normal);
    assert_eq!(ports.calls(), Vec::<String>::new());
    run(&mut app, &ports, &[Event::DiscardFile, Event::Yes]);
    assert_eq!(
        ports.calls(),
        vec![
            "Info: discard changes in u",
            "restore u",
            "refs",
            "commits 0 2000",
            "status",
            "diff Unstaged u"
        ]
    );
    assert_eq!(
        app.note(),
        &Note::Info("Discarded changes in u".to_string())
    );
}

#[test]
fn d_deletes_untracked_files_after_confirming() {
    let ports: TestPorts = ports_with(vec![file("n", ChangeKind::Untracked)], vec![]);
    let mut app: App = app(&ports);
    run(&mut app, &ports, &[Event::DiscardFile]);
    assert_eq!(
        app.mode(),
        &Mode::Confirm(Confirmation::DeleteUntracked {
            file: file("n", ChangeKind::Untracked)
        })
    );
    run(&mut app, &ports, &[Event::Yes]);
    assert_eq!(ports.calls()[..2], ["Info: delete n", "delete untracked n"]);
    assert_eq!(app.note(), &Note::Info("Deleted n".to_string()));
}

#[test]
fn discard_failure_is_shown() {
    for changed in [modified("u"), file("n", ChangeKind::Untracked)] {
        let ports: TestPorts = TestPorts {
            stage_fails: true,
            ..ports_with(vec![changed], vec![])
        };
        let mut failing_app: App = app(&ports);
        run(&mut failing_app, &ports, &[Event::DiscardFile, Event::Yes]);
        assert_eq!(
            failing_app.note(),
            &Note::Problem("Discarding failed, see the Log view".to_string())
        );
    }
}

#[test]
fn d_is_refused_on_staged_conflicted_and_missing_files_and_in_the_diff() {
    let ports: TestPorts = ports_with(vec![file("c", ChangeKind::Conflict)], vec![modified("s")]);
    let mut app: App = app(&ports);
    run(&mut app, &ports, &[Event::DiscardFile]);
    assert_eq!(app.mode(), &Mode::Normal);
    assert_eq!(
        ports.calls(),
        vec!["Error: c has a merge conflict; resolve it outside mgitui"]
    );
    run(&mut app, &ports, &[Event::SwitchList, Event::DiscardFile]);
    assert_eq!(app.mode(), &Mode::Normal);
    assert_eq!(
        app.note(),
        &Note::Info("Only unstaged changes can be discarded: unstage the file first".to_string())
    );
    run(&mut app, &ports, &[Event::SwitchList, Event::NextPane]);
    assert_eq!(app.diff().pane(), DiffPane::FileDiff);
    run(&mut app, &ports, &[Event::DiscardFile]);
    assert_eq!(app.mode(), &Mode::Normal);
    let empty: TestPorts = ports_with(vec![], vec![]);
    let mut empty_app: App = self::app(&empty);
    run(&mut empty_app, &empty, &[Event::DiscardFile]);
    assert_eq!(empty_app.mode(), &Mode::Normal);
    assert_eq!(empty.calls(), Vec::<String>::new());
}

#[test]
fn shift_a_stages_all_unstaged_changes_from_either_pane() {
    let ports: TestPorts = ports_with(
        vec![modified("a"), file("new", ChangeKind::Untracked)],
        vec![],
    );
    let mut app: App = app(&ports);
    run(&mut app, &ports, &[Event::StageAll]);
    let calls: Vec<String> = ports.calls();
    assert_eq!(calls[..3], ["status", "Info: stage all", "stage all"]);
    assert!(calls[3..].contains(&"status".to_string()));
    assert_eq!(app.note(), &Note::None);
    ports.clear();
    run(&mut app, &ports, &[Event::NextPane, Event::StageAll]);
    assert!(ports.calls().contains(&"stage all".to_string()));
}

#[test]
fn stage_all_problems() {
    let nothing: TestPorts = ports_with(vec![], vec![modified("s")]);
    let mut nothing_app: App = app(&nothing);
    run(&mut nothing_app, &nothing, &[Event::StageAll]);
    assert_eq!(nothing.calls(), vec!["status"]);
    assert_eq!(
        nothing_app.note(),
        &Note::Info("Nothing to stage".to_string())
    );

    let conflicted: TestPorts =
        ports_with(vec![file("c", ChangeKind::Conflict), modified("a")], vec![]);
    let mut conflicted_app: App = app(&conflicted);
    run(&mut conflicted_app, &conflicted, &[Event::StageAll]);
    let message: &str = "Resolve merge conflicts outside mgitui before staging all";
    assert_eq!(
        conflicted.calls(),
        vec!["status".to_string(), format!("Error: {message}")]
    );
    assert_eq!(conflicted_app.note(), &Note::Problem(message.to_string()));

    let failing: TestPorts = TestPorts {
        stage_fails: true,
        ..ports_with(vec![modified("a")], vec![])
    };
    let mut failing_app: App = app(&failing);
    run(&mut failing_app, &failing, &[Event::StageAll]);
    assert_eq!(
        failing_app.note(),
        &Note::Problem("Staging failed, see the Log view".to_string())
    );

    *failing.status.borrow_mut() = None;
    failing.clear();
    run(&mut failing_app, &failing, &[Event::StageAll]);
    assert_eq!(failing.calls(), vec!["status"]);
    assert_eq!(
        failing_app.note(),
        &Note::Problem("Reading the status failed, see the Log view".to_string())
    );
}
