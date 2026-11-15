use super::App;
use crate::event::{Event, ViewId};
use crate::model::{ChangedFile, Side, WorkStatus};
use crate::note::{Note, Operation, Problem};
use crate::test_ports::{TestPorts, modified};

/// Ports where `s` is staged and the amended commit would contain `a` and `s`. The selection
/// stays on `s` when amend mode is switched on, because reloading keeps the selected file.
fn amendable() -> TestPorts {
    let ports: TestPorts = TestPorts {
        amended: Some(vec![modified("a"), modified("s")]),
        ..TestPorts::new()
    };
    *ports.status.borrow_mut() = Some(WorkStatus {
        staged: vec![modified("s")],
        unstaged: vec![],
    });
    ports
}

fn diff_app(ports: &TestPorts) -> App {
    let mut app: App = App::new();
    app.handle(Event::SwitchView(ViewId::Diff), ports);
    ports.clear();
    app
}

fn run(app: &mut App, ports: &TestPorts, events: &[Event]) {
    for event in events {
        app.handle(*event, ports);
    }
}

fn staged_paths(app: &App) -> Vec<String> {
    app.diff()
        .files(Side::Staged)
        .iter()
        .map(|f: &ChangedFile| f.path.clone())
        .collect()
}

#[test]
fn amend_mode_lists_what_the_amended_commit_will_contain() {
    let ports: TestPorts = amendable();
    let mut app: App = diff_app(&ports);
    assert_eq!(staged_paths(&app), vec!["s"]);
    run(&mut app, &ports, &[Event::ToggleAmend]);
    assert_eq!(staged_paths(&app), vec!["a", "s"]);
    let calls: Vec<String> = ports.calls();
    assert!(calls.contains(&"has parent".to_string()));
    assert!(calls.contains(&"amended files since ParentCommit".to_string()));
    ports.clear();
    run(&mut app, &ports, &[Event::SwitchList]);
    assert_eq!(ports.calls(), vec!["diff amended since ParentCommit s"]);
    run(&mut app, &ports, &[Event::ToggleAmend]);
    assert_eq!(staged_paths(&app), vec!["s"]);
    assert_eq!(ports.calls().last(), Some(&"diff Staged s".to_string()));
}

#[test]
fn unstaging_in_amend_mode_takes_the_file_out_of_the_commit() {
    let ports: TestPorts = amendable();
    let mut app: App = diff_app(&ports);
    run(&mut app, &ports, &[Event::ToggleAmend, Event::SwitchList]);
    ports.clear();
    run(&mut app, &ports, &[Event::Activate]);
    let calls: Vec<String> = ports.calls();
    assert_eq!(calls[..2], ["Info: unstage s", "unstage from amend s"]);
    assert!(
        calls[2..].contains(&"has parent".to_string()),
        "the base is asked again on reload: {calls:?}"
    );
}

#[test]
fn amending_the_first_commit_compares_with_nothing() {
    let ports: TestPorts = TestPorts {
        has_parent: Some(false),
        ..amendable()
    };
    let mut app: App = diff_app(&ports);
    run(&mut app, &ports, &[Event::ToggleAmend, Event::SwitchList]);
    assert!(
        ports
            .calls()
            .contains(&"diff amended since Nothing s".to_string())
    );
    ports.clear();
    run(&mut app, &ports, &[Event::Activate]);
    assert_eq!(ports.calls()[1], "remove from index s");
}

#[test]
fn reloading_the_amended_commit_can_fail() {
    for ports in [
        TestPorts {
            has_parent: None,
            ..amendable()
        },
        TestPorts {
            amended: None,
            ..amendable()
        },
    ] {
        let mut app: App = diff_app(&ports);
        run(&mut app, &ports, &[Event::ToggleAmend]);
        assert_eq!(
            app.note(),
            &Note::Problem(Problem::Failed(Operation::ReloadingChanges))
        );
        assert!(app.diff().amend());
    }
}

#[test]
fn an_amend_that_would_leave_the_commit_empty_is_refused() {
    let ports: TestPorts = TestPorts {
        amended: Some(vec![]),
        ..amendable()
    };
    let mut app: App = diff_app(&ports);
    run(&mut app, &ports, &[Event::ToggleAmend]);
    ports.clear();
    run(&mut app, &ports, &[Event::Commit]);
    let message: &str = "The amended commit would be empty: stage changes first";
    assert_eq!(ports.calls(), vec![format!("Error: {message}")]);
    assert_eq!(
        app.note(),
        &Note::Problem(Problem::AmendedCommitWouldBeEmpty)
    );
    assert!(app.diff().amend());
}
