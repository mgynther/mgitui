use super::{App, DiffPane, Note, Viewports};
use crate::event::{Event, ViewId};
use crate::model::{RowKind, Side, WorkStatus};
use crate::test_ports::{TestPorts, modified};

fn viewports() -> Viewports {
    Viewports {
        branches_graph: 5,
        branches_files: 5,
        branches_diff: 5,
        log: 5,
        diff_unstaged: 2,
        diff_staged: 2,
        diff_file: 2,
    }
}

fn diff_app(ports: &TestPorts) -> App {
    let mut app: App = App::new();
    app.set_viewports(viewports());
    app.handle(Event::SwitchView(ViewId::Diff), ports);
    app.handle(Event::Reload, ports);
    app
}

fn run(app: &mut App, ports: &TestPorts, events: &[Event]) {
    for event in events {
        app.handle(*event, ports);
    }
}

fn selected_path(app: &App) -> Option<String> {
    app.diff().selected_file().map(|f| f.path.clone())
}

#[test]
fn reload_loads_status_and_first_diff() {
    let ports: TestPorts = TestPorts::with_files(&["a", "b"], &["s"]);
    let app: App = diff_app(&ports);
    assert_eq!(
        ports.calls(),
        vec![
            "Info: reload",
            "refs",
            "commits 0 2000",
            "status",
            "diff Unstaged a"
        ]
    );
    assert!(app.diff().status().has_changes());
    assert_eq!(app.diff().side(), Side::Unstaged);
    assert_eq!(app.diff().pane(), DiffPane::Files);
    assert_eq!(app.diff().files(Side::Staged).len(), 1);
    assert_eq!(app.diff().rows()[1].text, "a");
    assert_eq!(app.diff().rows()[0].kind, RowKind::HunkHeader);
}

#[test]
fn moving_in_the_list_loads_diffs() {
    let ports: TestPorts = TestPorts::with_files(&["a", "b", "c"], &[]);
    let mut app: App = diff_app(&ports);
    ports.clear();
    run(
        &mut app,
        &ports,
        &[Event::MoveDown, Event::Bottom, Event::MoveDown],
    );
    assert_eq!(ports.calls(), vec!["diff Unstaged b", "diff Unstaged c"]);
    assert_eq!(app.diff().cursor(Side::Unstaged).offset(), 1);
    run(&mut app, &ports, &[Event::Top, Event::MoveUp]);
    assert_eq!(selected_path(&app), Some("a".to_string()));
}

#[test]
fn switching_lists_and_panes() {
    let ports: TestPorts = TestPorts::with_files(&["a"], &["s|t|u"]);
    let mut app: App = diff_app(&ports);
    ports.clear();
    run(&mut app, &ports, &[Event::SwitchList]);
    assert_eq!(app.diff().side(), Side::Staged);
    assert_eq!(ports.calls(), vec!["diff Staged s|t|u"]);
    run(&mut app, &ports, &[Event::NextPane]);
    assert_eq!(app.diff().pane(), DiffPane::FileDiff);
    run(&mut app, &ports, &[Event::Bottom, Event::SwitchList]);
    assert_eq!(app.diff().diff_cursor().selected(), 3);
    assert_eq!(app.diff().diff_cursor().offset(), 2);
    assert_eq!(app.diff().side(), Side::Staged);
    run(&mut app, &ports, &[Event::NextPane, Event::SwitchList]);
    assert_eq!(app.diff().pane(), DiffPane::Files);
    assert_eq!(app.diff().side(), Side::Unstaged);
}

#[test]
fn empty_list_has_no_diff() {
    let ports: TestPorts = TestPorts::with_files(&["a"], &[]);
    let mut app: App = diff_app(&ports);
    run(&mut app, &ports, &[Event::SwitchList]);
    assert_eq!(app.diff().rows(), &[]);
    assert_eq!(selected_path(&app), None);
}

#[test]
fn reload_keeps_selected_file_or_moves_to_next() {
    let ports: TestPorts = TestPorts::with_files(&["a", "b", "c"], &[]);
    let mut app: App = diff_app(&ports);
    run(&mut app, &ports, &[Event::MoveDown]);
    *ports.status.borrow_mut() = Some(WorkStatus {
        unstaged: vec![modified("0"), modified("a"), modified("b"), modified("c")],
        staged: vec![],
    });
    run(&mut app, &ports, &[Event::Reload]);
    assert_eq!(selected_path(&app), Some("b".to_string()));
    *ports.status.borrow_mut() = Some(WorkStatus {
        unstaged: vec![modified("0"), modified("a"), modified("c")],
        staged: vec![],
    });
    run(&mut app, &ports, &[Event::Reload]);
    assert_eq!(selected_path(&app), Some("c".to_string()));
}

#[test]
fn failures_are_shown_in_the_status_line() {
    let ports: TestPorts = TestPorts::with_files(&["a"], &[]);
    let mut app: App = diff_app(&ports);
    *ports.status.borrow_mut() = None;
    run(&mut app, &ports, &[Event::Reload]);
    assert_eq!(
        app.note(),
        &Note::Problem("Reloading changes failed, see the Log view".to_string())
    );
    assert_eq!(selected_path(&app), Some("a".to_string()));
    let failing: TestPorts = TestPorts {
        diff_fails: true,
        ..TestPorts::with_files(&["a"], &[])
    };
    run(&mut app, &failing, &[Event::Reload]);
    assert_eq!(
        app.note(),
        &Note::Problem("Loading the diff failed, see the Log view".to_string())
    );
    assert_eq!(app.diff().rows(), &[]);
}

#[test]
fn search_selects_matching_file() {
    let ports: TestPorts = TestPorts::with_files(&["src/a.rs", "doc/b.md", "src/c.rs"], &[]);
    let mut app: App = diff_app(&ports);
    run(&mut app, &ports, &[Event::NextPane, Event::StartSearch]);
    for c in "MD".chars() {
        app.handle(Event::Input(c), &ports);
    }
    run(&mut app, &ports, &[Event::InputSubmit]);
    assert_eq!(selected_path(&app), Some("doc/b.md".to_string()));
    assert_eq!(app.diff().pane(), DiffPane::Files);
    run(
        &mut app,
        &ports,
        &[Event::StartSearch, Event::Input('z'), Event::InputSubmit],
    );
    assert_eq!(app.note(), &Note::Info("Pattern not found: z".to_string()));
}

#[test]
fn resize_clamps_selections() {
    let ports: TestPorts = TestPorts::with_files(&["a", "b", "c"], &["s"]);
    let mut app: App = diff_app(&ports);
    run(
        &mut app,
        &ports,
        &[Event::Bottom, Event::NextPane, Event::Bottom],
    );
    app.set_viewports(Viewports {
        diff_unstaged: 1,
        diff_file: 1,
        ..viewports()
    });
    assert_eq!(app.diff().cursor(Side::Unstaged).offset(), 2);
    assert_eq!(app.diff().diff_cursor().offset(), 1);
}
