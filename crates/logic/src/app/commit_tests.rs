use super::App;
use crate::event::{Event, ViewId};
use crate::note::{InputProblem, Note, Operation, Problem};
use crate::test_ports::{TestPorts, modified};

fn app(ports: &TestPorts) -> App {
    let mut app: App = App::new();
    run(&mut app, ports, &[Event::SwitchView(ViewId::Diff)]);
    ports.clear();
    app
}

fn run(app: &mut App, ports: &TestPorts, events: &[Event]) {
    for event in events {
        app.handle(*event, ports);
    }
}

#[test]
fn edit_and_commit() {
    let ports: TestPorts = TestPorts::new();
    let mut app: App = app(&ports);
    run(&mut app, &ports, &[Event::EditMessage]);
    assert_eq!(app.diff().message(), "edited\n");
    run(&mut app, &ports, &[Event::NextPane, Event::EditMessage]);
    assert_eq!(app.diff().message(), "edited\nedited\n");
    run(&mut app, &ports, &[Event::Commit]);
    assert_eq!(
        ports.calls(),
        vec![
            "Info: edit commit message",
            "edit \"\"",
            "Info: edit commit message",
            "edit \"edited\\n\"",
            "Info: commit",
            "commit amend=false \"edited\\nedited\\n\"",
            "refs",
            "commits 0 2000",
            "status",
        ]
    );
    assert_eq!(app.diff().message(), "");
    assert!(!app.diff().amend());
}

#[test]
fn empty_message_is_refused() {
    let ports: TestPorts = TestPorts::new();
    let mut app: App = app(&ports);
    run(&mut app, &ports, &[Event::Commit]);
    assert_eq!(
        app.note(),
        &Note::Problem(Problem::Invalid(InputProblem::EmptyCommitMessage))
    );
    assert_eq!(ports.calls(), Vec::<String>::new());
}

#[test]
fn failures_keep_the_message() {
    let ports: TestPorts = TestPorts {
        editor_appends: None,
        ..TestPorts::new()
    };
    let mut app: App = app(&ports);
    run(&mut app, &ports, &[Event::EditMessage]);
    assert_eq!(
        app.note(),
        &Note::Problem(Problem::Failed(Operation::EditingMessage))
    );
    let failing: TestPorts = TestPorts {
        commit_fails: true,
        ..TestPorts::new()
    };
    run(&mut app, &failing, &[Event::EditMessage, Event::Commit]);
    assert_eq!(
        app.note(),
        &Note::Problem(Problem::Failed(Operation::Committing))
    );
    assert_eq!(app.diff().message(), "edited\n");
}

#[test]
fn amend_mode_has_its_own_draft() {
    let ports: TestPorts = TestPorts {
        amended: Some(vec![modified("a")]),
        ..TestPorts::new()
    };
    let mut app: App = app(&ports);
    run(&mut app, &ports, &[Event::EditMessage, Event::ToggleAmend]);
    assert!(app.diff().amend());
    assert_eq!(app.diff().message(), "Last\n");
    run(&mut app, &ports, &[Event::EditMessage]);
    assert_eq!(app.diff().message(), "Last\nedited\n");
    run(&mut app, &ports, &[Event::ToggleAmend]);
    assert!(!app.diff().amend());
    assert_eq!(app.diff().message(), "edited\n");
    run(&mut app, &ports, &[Event::ToggleAmend]);
    assert_eq!(app.diff().message(), "Last\n");
    ports.clear();
    run(&mut app, &ports, &[Event::Commit]);
    assert_eq!(
        ports.calls(),
        vec![
            "Info: amend",
            "commit amend=true \"Last\\n\"",
            "refs",
            "commits 0 2000",
            "status"
        ]
    );
    assert!(!app.diff().amend());
    assert_eq!(app.diff().message(), "edited\n");
}

#[test]
fn amend_needs_a_commit() {
    let ports: TestPorts = TestPorts {
        has_commits: Some(false),
        ..TestPorts::new()
    };
    let mut app: App = app(&ports);
    run(&mut app, &ports, &[Event::ToggleAmend]);
    let message: &str = "Nothing to amend: there are no commits yet";
    assert_eq!(app.note(), &Note::Problem(Problem::NothingToAmend));
    assert_eq!(
        ports.calls(),
        vec!["has commits".to_string(), format!("Warn: {message}")]
    );
    assert!(!app.diff().amend());
}

#[test]
fn amend_failures() {
    let problem: Note = Note::Problem(Problem::Failed(Operation::ReadingLastCommit));
    for ports in [
        TestPorts {
            has_commits: None,
            ..TestPorts::new()
        },
        TestPorts {
            last_message: None,
            ..TestPorts::new()
        },
    ] {
        let mut app: App = app(&ports);
        run(&mut app, &ports, &[Event::ToggleAmend]);
        assert_eq!(app.note(), &problem);
        assert!(!app.diff().amend());
    }
}
