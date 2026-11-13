use super::{App, DiffPane, Note, Viewports};
use crate::event::{Event, ViewId};
use crate::model::{ChangeKind, ChangedFile, DiffLine, FileDiff, Hunk, LineKind, Side, WorkStatus};
use crate::test_ports::{TestPorts, modified};

fn viewports() -> Viewports {
    Viewports {
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
        vec!["Info: stage u", "stage u", "status", "diff Unstaged u"]
    );
    ports.clear();
    run(&mut app, &ports, &[Event::SwitchList, Event::Activate]);
    assert_eq!(
        ports.calls(),
        vec![
            "diff Staged s",
            "Info: unstage s",
            "unstage s",
            "status",
            "diff Staged s"
        ]
    );
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
