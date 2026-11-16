use super::{App, BranchesPane, CHUNK, Viewports};
use crate::event::Event;
use crate::model::{Label, Refs};
use crate::note::{Info, Note, Operation, Problem};
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
        branches_files: 0,
        branches_diff: 0,
        log: 0,
        diff_unstaged: 0,
        diff_staged: 0,
        diff_file: 0,
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

/// Types `text` into a search prompt and submits it.
fn search(app: &mut App, ports: &TestPorts, text: &str) {
    app.handle(Event::StartSearch, ports);
    for c in text.chars() {
        app.handle(Event::Input(c), ports);
    }
    app.handle(Event::InputSubmit, ports);
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
    assert!(
        !ports
            .calls()
            .iter()
            .any(|c: &String| c.starts_with("commits"))
    );
    ports.clear();
    run(&mut app, &ports, &[Event::Bottom]);
    assert_eq!(ports.calls()[0], format!("commits {CHUNK} {CHUNK}"));
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
        &Note::Problem(Problem::Failed(Operation::LoadingMoreCommits))
    );
    run(&mut app, &ports, &[Event::Reload]);
    assert_eq!(
        app.note(),
        &Note::Problem(Problem::Failed(Operation::ReloadingCommits))
    );
    *ports.refs.borrow_mut() = None;
    *ports.history.borrow_mut() = Some(linear_history(1));
    run(&mut app, &ports, &[Event::Reload]);
    assert_eq!(
        app.note(),
        &Note::Problem(Problem::Failed(Operation::ReloadingCommits))
    );
    assert_eq!(app.branches().commits().len(), CHUNK);
}

#[test]
fn search_by_subject_hash_and_author() {
    let ports: TestPorts = ports(10);
    let mut app: App = app(&ports);
    search(&mut app, &ports, "commit 3");
    assert_eq!(selected_hash(&app), Some("c3".to_string()));
    search(&mut app, &ports, "c7");
    assert_eq!(selected_hash(&app), Some("c7".to_string()));
    search(&mut app, &ports, "bo");
    assert_eq!(selected_hash(&app), Some("c5".to_string()));
    run(&mut app, &ports, &[Event::SearchPrevious]);
    assert_eq!(selected_hash(&app), Some("c7".to_string()));
    search(&mut app, &ports, "nothing");
    assert_eq!(
        app.note(),
        &Note::Info(Info::PatternNotFound {
            term: "nothing".to_string()
        })
    );
}

#[test]
fn search_by_branch_and_tag_names() {
    let ports: TestPorts = ports(10);
    let label = |hash: &str, label: Label| (hash.to_string(), label);
    *ports.refs.borrow_mut() = Some(Refs {
        labels: vec![
            label(
                "c2",
                Label::Local {
                    name: "Feature".to_string(),
                    head: false,
                },
            ),
            label(
                "c4",
                Label::Remote {
                    remote: "origin".to_string(),
                    branch: "topic".to_string(),
                },
            ),
            label(
                "c6",
                Label::Tag {
                    name: "v1.2".to_string(),
                },
            ),
            label("c3", Label::RebaseHead),
        ],
        head: None,
    });
    let mut app: App = app(&ports);
    search(&mut app, &ports, "feat");
    assert_eq!(selected_hash(&app), Some("c2".to_string()));
    search(&mut app, &ports, "v1.2");
    assert_eq!(selected_hash(&app), Some("c6".to_string()));
    search(&mut app, &ports, "origin/top");
    assert_eq!(selected_hash(&app), Some("c4".to_string()));
    search(&mut app, &ports, "n/t");
    assert_eq!(selected_hash(&app), Some("c4".to_string()));
    assert_eq!(app.note(), &Note::Info(Info::SearchWrapped));
}

#[test]
fn next_main_branch_cycles_through_exact_main_and_master_names() {
    let ports: TestPorts = ports(10);
    let local = |hash: &str, name: &str| {
        (
            hash.to_string(),
            Label::Local {
                name: name.to_string(),
                head: false,
            },
        )
    };
    let remote = |hash: &str, remote: &str, branch: &str| {
        (
            hash.to_string(),
            Label::Remote {
                remote: remote.to_string(),
                branch: branch.to_string(),
            },
        )
    };
    *ports.refs.borrow_mut() = Some(Refs {
        labels: vec![
            local("c8", "main-feature"),
            (
                "c7".to_string(),
                Label::Tag {
                    name: "main".to_string(),
                },
            ),
            local("c6", "Main"),
            ("c6".to_string(), Label::RebaseHead),
            remote("c5", "origin", "master"),
            remote("c4", "origin", "feature/main"),
            remote("c3", "upstream", "main"),
            local("c2", "main"),
            remote("c2", "origin", "main"),
        ],
        head: None,
    });
    let mut app: App = app(&ports);
    let mut visited: Vec<String> = Vec::new();
    for _ in 0..4 {
        run(&mut app, &ports, &[Event::NextMainBranch]);
        visited.extend(selected_hash(&app));
    }
    assert_eq!(visited, vec!["c5", "c3", "c2", "c5"]);
    assert_eq!(app.note(), &Note::None);
}

#[test]
fn next_main_branch_without_main_or_master() {
    let ports: TestPorts = ports(3);
    let mut app: App = app(&ports);
    run(&mut app, &ports, &[Event::NextMainBranch]);
    assert_eq!(selected_hash(&app), Some("c2".to_string()));
    assert_eq!(app.note(), &Note::Info(Info::NoMainOrMasterBranch));
}

#[test]
fn resize_clamps_the_commit_selection() {
    let ports: TestPorts = ports(30);
    let mut app: App = app(&ports);
    run(&mut app, &ports, &[Event::Bottom]);
    app.set_viewports(Viewports {
        branches_graph: 5,
        branches_files: 0,
        branches_diff: 0,
        log: 0,
        diff_unstaged: 0,
        diff_staged: 0,
        diff_file: 0,
    });
    assert_eq!(app.branches().cursor().offset(), 25);
}

#[test]
fn details_of_the_selected_commit() {
    let ports: TestPorts = ports(3);
    let mut app: App = app(&ports);
    assert_eq!(app.branches().selected_lane(), Some(0));
    assert_eq!(app.branches().message(), "Message of c2\n\nBody\n");
    assert_eq!(app.branches().files().len(), 2);
    assert_eq!(app.branches().diff_rows()[1].text, "c2");
    run(&mut app, &ports, &[Event::MoveDown]);
    assert_eq!(
        ports.calls(),
        vec!["message c1", "files c1", "show c1 c1.txt"]
    );
    ports.clear();
    run(&mut app, &ports, &[Event::MoveDown, Event::MoveDown]);
    assert_eq!(
        ports.calls(),
        vec!["message c0", "files c0", "show c0 c0.txt"]
    );
}

#[test]
fn panes_cycle_and_move_separately() {
    let ports: TestPorts = ports(3);
    let mut app: App = app(&ports);
    assert_eq!(app.branches().pane(), BranchesPane::Graph);
    run(&mut app, &ports, &[Event::NextPane]);
    assert_eq!(app.branches().pane(), BranchesPane::Files);
    ports.clear();
    run(&mut app, &ports, &[Event::MoveDown, Event::MoveDown]);
    assert_eq!(ports.calls(), vec!["show c2 other.txt"]);
    assert_eq!(
        app.branches().selected_file().map(|f| f.path.clone()),
        Some("other.txt".to_string())
    );
    run(&mut app, &ports, &[Event::NextPane, Event::Bottom]);
    assert_eq!(app.branches().pane(), BranchesPane::FileDiff);
    assert_eq!(app.branches().diff_cursor().selected(), 4);
    assert_eq!(app.branches().file_cursor().selected(), 1);
    run(&mut app, &ports, &[Event::NextPane]);
    assert_eq!(app.branches().pane(), BranchesPane::Graph);
    run(
        &mut app,
        &ports,
        &[
            Event::NextPane,
            Event::StartSearch,
            Event::Input('1'),
            Event::InputSubmit,
        ],
    );
    assert_eq!(app.branches().pane(), BranchesPane::Graph);
    assert_eq!(app.branches().file_cursor().selected(), 0);
    app.set_viewports(Viewports {
        branches_graph: 1,
        branches_files: 1,
        branches_diff: 1,
        log: 0,
        diff_unstaged: 0,
        diff_staged: 0,
        diff_file: 0,
    });
    assert_eq!(app.branches().cursor().offset(), 1);
}

#[test]
fn detail_failures_and_empty_history() {
    let failing: TestPorts = TestPorts {
        details_fail: true,
        ..ports(2)
    };
    let mut app: App = app(&failing);
    assert_eq!(app.branches().files(), &[]);
    run(&mut app, &failing, &[Event::MoveDown]);
    assert_eq!(
        app.note(),
        &Note::Problem(Problem::Failed(Operation::LoadingCommit))
    );
    let ports: TestPorts = ports(2);
    run(&mut app, &ports, &[Event::MoveUp]);
    assert_eq!(app.branches().files().len(), 2);
    run(&mut app, &failing, &[Event::NextPane, Event::MoveDown]);
    assert_eq!(
        app.note(),
        &Note::Problem(Problem::Failed(Operation::LoadingDiff))
    );
    *ports.history.borrow_mut() = Some(Vec::new());
    run(&mut app, &ports, &[Event::Reload]);
    assert_eq!(app.branches().message(), "");
    assert_eq!(app.branches().diff_rows(), &[]);
    assert_eq!(app.branches().selected_lane(), None);
}
