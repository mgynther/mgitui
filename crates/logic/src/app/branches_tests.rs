use super::{App, CHUNK, Note, Viewports};
use crate::event::Event;
use crate::model::{Label, Refs};
use crate::test_ports::{TestPorts, linear_history};

fn ports(count: usize) -> TestPorts {
    let ports: TestPorts = TestPorts::new();
    *ports.history.borrow_mut() = Some(linear_history(count));
    ports
}

fn app(ports: &TestPorts) -> App {
    let mut app: App = App::new();
    app.set_viewports(Viewports {
        branches_graph: 10,
        ..Viewports::default()
    });
    app.handle(Event::Reload, ports);
    ports.clear();
    app
}

fn run(app: &mut App, ports: &TestPorts, events: &[Event]) {
    for event in events {
        app.handle(*event, ports);
    }
}

fn selected_hash(app: &App) -> Option<String> {
    app.branches().selected().map(|c| c.hash.clone())
}

#[test]
fn reload_loads_refs_and_first_chunk() {
    let ports: TestPorts = ports(5);
    *ports.refs.borrow_mut() = Some(Refs {
        labels: vec![
            (
                "c4".to_string(),
                Label::Local {
                    name: "main".to_string(),
                    head: true,
                },
            ),
            (
                "c4".to_string(),
                Label::Tag {
                    name: "v1".to_string(),
                },
            ),
        ],
        head: Some("c4".to_string()),
    });
    let app: App = app(&ports);
    assert_eq!(app.branches().commits().len(), 5);
    assert!(app.branches().all_loaded());
    assert_eq!(app.branches().labels("c4").len(), 2);
    assert_eq!(app.branches().labels("c0"), &[]);
    assert_eq!(app.branches().head(), Some("c4"));
    assert_eq!(selected_hash(&app), Some("c4".to_string()));
}

#[test]
fn more_commits_load_near_the_bottom() {
    let ports: TestPorts = ports(CHUNK + 50);
    let mut app: App = app(&ports);
    assert_eq!(app.branches().commits().len(), CHUNK);
    assert!(!app.branches().all_loaded());
    run(&mut app, &ports, &[Event::MoveDown]);
    assert_eq!(ports.calls(), Vec::<String>::new());
    run(&mut app, &ports, &[Event::Bottom]);
    assert_eq!(ports.calls(), vec![format!("commits {CHUNK} {CHUNK}")]);
    assert_eq!(app.branches().commits().len(), CHUNK + 50);
    assert_eq!(app.branches().rows().len(), CHUNK + 50);
    assert!(app.branches().all_loaded());
    assert_eq!(app.branches().cursor().selected(), CHUNK - 1);
    run(
        &mut app,
        &ports,
        &[Event::Bottom, Event::MoveUp, Event::Top],
    );
    assert_eq!(selected_hash(&app), Some(format!("c{}", CHUNK + 49)));
}

#[test]
fn reload_keeps_selected_commit_and_loaded_count() {
    let ports: TestPorts = ports(CHUNK + 50);
    let mut app: App = app(&ports);
    run(&mut app, &ports, &[Event::Bottom, Event::MoveDown]);
    let selected: Option<String> = selected_hash(&app);
    *ports.history.borrow_mut() = Some(linear_history(CHUNK + 51));
    ports.clear();
    run(&mut app, &ports, &[Event::Reload]);
    assert!(ports.calls().contains(&format!("commits 0 {}", CHUNK + 50)));
    assert_eq!(selected_hash(&app), selected);
    *ports.history.borrow_mut() = Some(linear_history(3));
    run(&mut app, &ports, &[Event::Reload]);
    assert_eq!(app.branches().cursor().selected(), 0);
}

#[test]
fn failures_are_shown() {
    let ports: TestPorts = ports(CHUNK + 1);
    let mut app: App = app(&ports);
    *ports.history.borrow_mut() = None;
    run(&mut app, &ports, &[Event::Bottom]);
    assert_eq!(
        app.note(),
        &Note::Problem("Loading more commits failed, see the Log view".to_string())
    );
    run(&mut app, &ports, &[Event::Reload]);
    assert_eq!(
        app.note(),
        &Note::Problem("Reloading commits failed, see the Log view".to_string())
    );
    *ports.refs.borrow_mut() = None;
    *ports.history.borrow_mut() = Some(linear_history(1));
    run(&mut app, &ports, &[Event::Reload]);
    assert_eq!(
        app.note(),
        &Note::Problem("Reloading commits failed, see the Log view".to_string())
    );
    assert_eq!(app.branches().commits().len(), CHUNK);
}

#[test]
fn search_by_subject_hash_and_author() {
    let ports: TestPorts = ports(10);
    let mut app: App = app(&ports);
    let search = |app: &mut App, text: &str| {
        app.handle(Event::StartSearch, &ports);
        for c in text.chars() {
            app.handle(Event::Input(c), &ports);
        }
        app.handle(Event::InputSubmit, &ports);
    };
    search(&mut app, "commit 3");
    assert_eq!(selected_hash(&app), Some("c3".to_string()));
    search(&mut app, "c7");
    assert_eq!(selected_hash(&app), Some("c7".to_string()));
    search(&mut app, "bo");
    assert_eq!(selected_hash(&app), Some("c5".to_string()));
    run(&mut app, &ports, &[Event::SearchPrevious]);
    assert_eq!(selected_hash(&app), Some("c7".to_string()));
    search(&mut app, "nothing");
    assert_eq!(
        app.note(),
        &Note::Info("Pattern not found: nothing".to_string())
    );
}

#[test]
fn resize_clamps_the_commit_selection() {
    let ports: TestPorts = ports(30);
    let mut app: App = app(&ports);
    run(&mut app, &ports, &[Event::Bottom]);
    app.set_viewports(Viewports {
        branches_graph: 5,
        ..Viewports::default()
    });
    assert_eq!(app.branches().cursor().offset(), 25);
}
