use super::{App, Mode, Note, Prompt, PromptKind};
use crate::event::Event;
use crate::model::{Label, Refs};
use crate::test_ports::{TestPorts, linear_history};

fn local(name: &str) -> Label {
    Label::Local {
        name: name.to_string(),
        head: false,
    }
}

/// Three commits; `c2` has the given labels.
fn labelled_ports(labels: Vec<Label>) -> TestPorts {
    let ports: TestPorts = TestPorts::new();
    *ports.history.borrow_mut() = Some(linear_history(3));
    *ports.refs.borrow_mut() = Some(Refs {
        labels: labels.into_iter().map(|l| ("c2".to_string(), l)).collect(),
        head: Some("c2".to_string()),
    });
    ports
}

fn loaded_app(ports: &TestPorts) -> App {
    let mut app: App = App::new();
    app.handle(Event::Reload, ports);
    ports.clear();
    app
}

fn run(app: &mut App, ports: &TestPorts, events: &[Event]) {
    for event in events {
        app.handle(*event, ports);
    }
}

fn typed(app: &mut App, ports: &TestPorts, text: &str) {
    for c in text.chars() {
        app.handle(Event::Input(c), ports);
    }
}

fn actions(ports: &TestPorts) -> Vec<String> {
    ports
        .calls()
        .into_iter()
        .filter(|c: &String| {
            c.starts_with("create")
                || c.starts_with("checkout")
                || c.starts_with("cherry-pick")
                || c.starts_with("rebase")
                || c.contains(": ")
        })
        .collect()
}

#[test]
fn create_branch_at_selected_commit() {
    let ports: TestPorts = labelled_ports(vec![]);
    let mut app: App = loaded_app(&ports);
    run(&mut app, &ports, &[Event::MoveDown, Event::CreateBranch]);
    assert_eq!(
        app.mode(),
        &Mode::Prompt(Prompt {
            kind: PromptKind::BranchName {
                at: "c1".to_string()
            },
            text: String::new()
        })
    );
    typed(&mut app, &ports, "topic");
    run(&mut app, &ports, &[Event::InputSubmit]);
    assert_eq!(
        actions(&ports),
        vec!["Info: create branch topic at c1", "create topic at c1"]
    );
    assert_eq!(app.note(), &Note::Info("Created branch topic".to_string()));
    assert!(ports.calls().contains(&"refs".to_string()));
}

#[test]
fn create_branch_problems() {
    let ports: TestPorts = labelled_ports(vec![]);
    let mut app: App = loaded_app(&ports);
    run(&mut app, &ports, &[Event::CreateBranch]);
    typed(&mut app, &ports, "bad name");
    run(&mut app, &ports, &[Event::InputSubmit]);
    assert_eq!(
        app.note(),
        &Note::Problem("invalid branch name".to_string())
    );
    assert_eq!(app.mode(), &Mode::Normal);
    let failing: TestPorts = TestPorts {
        branch_fails: true,
        ..labelled_ports(vec![])
    };
    run(
        &mut app,
        &failing,
        &[Event::CreateBranch, Event::Input('x'), Event::InputSubmit],
    );
    assert_eq!(
        app.note(),
        &Note::Problem("Creating the branch failed, see the Log view".to_string())
    );
    let empty: TestPorts = TestPorts::new();
    let mut empty_app: App = loaded_app(&empty);
    run(&mut empty_app, &empty, &[Event::CreateBranch]);
    assert_eq!(
        empty_app.note(),
        &Note::Info("No commit selected".to_string())
    );
    assert_eq!(empty_app.mode(), &Mode::Normal);
}

#[test]
fn checkout_the_only_local_branch() {
    let ports: TestPorts = labelled_ports(vec![
        local("main"),
        Label::Remote {
            remote: "origin".to_string(),
            branch: "main".to_string(),
        },
        Label::Tag {
            name: "v1".to_string(),
        },
    ]);
    let mut app: App = loaded_app(&ports);
    run(&mut app, &ports, &[Event::Checkout]);
    assert_eq!(
        actions(&ports),
        vec!["Info: check out main", "checkout main"]
    );
    assert_eq!(app.note(), &Note::Info("Checked out main".to_string()));
}

#[test]
fn checkout_without_local_branch_is_an_error() {
    let ports: TestPorts = labelled_ports(vec![Label::Tag {
        name: "v1".to_string(),
    }]);
    let mut app: App = loaded_app(&ports);
    run(&mut app, &ports, &[Event::Checkout]);
    let message: &str = "No local branch at this commit to check out";
    assert_eq!(actions(&ports), vec![format!("Error: {message}")]);
    assert_eq!(app.note(), &Note::Problem(message.to_string()));
    let empty: TestPorts = TestPorts::new();
    let mut empty_app: App = loaded_app(&empty);
    run(&mut empty_app, &empty, &[Event::Checkout]);
    assert_eq!(empty_app.note(), &Note::Problem(message.to_string()));
}

#[test]
fn choose_between_local_branches() {
    let ports: TestPorts = labelled_ports(vec![local("main"), local("dev")]);
    let mut app: App = loaded_app(&ports);
    run(&mut app, &ports, &[Event::Checkout]);
    let prompt: Prompt = Prompt {
        kind: PromptKind::CheckoutChoice {
            branches: vec!["main".to_string(), "dev".to_string()],
        },
        text: "3".to_string(),
    };
    typed(&mut app, &ports, "3");
    run(&mut app, &ports, &[Event::InputSubmit]);
    assert_eq!(
        app.note(),
        &Note::Problem("choice must be a number from 1 to 2".to_string())
    );
    assert_eq!(app.mode(), &Mode::Prompt(prompt));
    run(
        &mut app,
        &ports,
        &[Event::InputBackspace, Event::Input('2'), Event::InputSubmit],
    );
    assert_eq!(actions(&ports), vec!["Info: check out dev", "checkout dev"]);
    assert_eq!(app.mode(), &Mode::Normal);
}

#[test]
fn checkout_failure_and_cancel() {
    let failing: TestPorts = TestPorts {
        branch_fails: true,
        ..labelled_ports(vec![local("main"), local("dev")])
    };
    let mut app: App = loaded_app(&failing);
    run(&mut app, &failing, &[Event::Checkout, Event::InputCancel]);
    assert_eq!(app.mode(), &Mode::Normal);
    run(&mut app, &failing, &[Event::Checkout]);
    typed(&mut app, &failing, "99");
    run(&mut app, &failing, &[Event::InputSubmit]);
    assert_eq!(
        app.note(),
        &Note::Problem("Choice is not listed".to_string())
    );
    run(
        &mut app,
        &failing,
        &[Event::Checkout, Event::Input('1'), Event::InputSubmit],
    );
    assert_eq!(
        app.note(),
        &Note::Problem("Checkout failed, see the Log view".to_string())
    );
}

#[test]
fn cherry_pick_selected_commit() {
    let ports: TestPorts = labelled_ports(vec![]);
    let mut app: App = loaded_app(&ports);
    run(&mut app, &ports, &[Event::MoveDown, Event::CherryPick]);
    assert_eq!(
        actions(&ports),
        vec!["Info: cherry-pick c1", "cherry-pick c1"]
    );
    assert_eq!(app.note(), &Note::Info("Cherry-picked c1".to_string()));
    let failing: TestPorts = TestPorts {
        branch_fails: true,
        ..labelled_ports(vec![])
    };
    run(&mut app, &failing, &[Event::CherryPick]);
    assert_eq!(
        app.note(),
        &Note::Problem("Cherry-pick failed, see the Log view".to_string())
    );
    let empty: TestPorts = TestPorts::new();
    let mut empty_app: App = loaded_app(&empty);
    run(&mut empty_app, &empty, &[Event::CherryPick]);
    assert_eq!(
        empty_app.note(),
        &Note::Info("No commit selected".to_string())
    );
}

/// Labels: c2 has `topic`, c1 has `main`, `dev` and `aaa`, c0 has `master`.
fn rebase_ports(topic_labels: Vec<Label>) -> TestPorts {
    let ports: TestPorts = labelled_ports(topic_labels);
    if let Some(refs) = ports.refs.borrow_mut().as_mut() {
        for name in ["dev", "main", "aaa"] {
            refs.labels.push(("c1".to_string(), local(name)));
        }
        refs.labels.push(("c0".to_string(), local("master")));
    }
    ports
}

#[test]
fn rebase_asks_for_target_with_main_and_master_first() {
    let ports: TestPorts = rebase_ports(vec![local("topic")]);
    let mut app: App = loaded_app(&ports);
    run(&mut app, &ports, &[Event::Rebase]);
    assert_eq!(
        app.mode(),
        &Mode::Prompt(Prompt {
            kind: PromptKind::RebaseOnto {
                branch: "topic".to_string(),
                onto: vec![
                    "main".to_string(),
                    "master".to_string(),
                    "aaa".to_string(),
                    "dev".to_string()
                ],
            },
            text: String::new()
        })
    );
    typed(&mut app, &ports, "9");
    run(&mut app, &ports, &[Event::InputSubmit]);
    assert!(matches!(app.mode(), Mode::Prompt(_)));
    run(
        &mut app,
        &ports,
        &[Event::InputBackspace, Event::Input('1'), Event::InputSubmit],
    );
    assert_eq!(
        actions(&ports),
        vec!["Info: rebase topic onto main", "rebase topic onto main"]
    );
    assert_eq!(
        app.note(),
        &Note::Info("Rebased topic onto main".to_string())
    );
}

#[test]
fn rebase_asks_which_branch_first() {
    let ports: TestPorts = rebase_ports(vec![local("topic"), local("other")]);
    let mut app: App = loaded_app(&ports);
    run(&mut app, &ports, &[Event::Rebase]);
    assert_eq!(
        app.mode(),
        &Mode::Prompt(Prompt {
            kind: PromptKind::RebaseBranch {
                branches: vec!["topic".to_string(), "other".to_string()],
            },
            text: String::new()
        })
    );
    typed(&mut app, &ports, "2");
    run(&mut app, &ports, &[Event::InputSubmit]);
    let Mode::Prompt(prompt) = app.mode().clone() else {
        panic!("expected the onto prompt");
    };
    assert_eq!(
        prompt.kind,
        PromptKind::RebaseOnto {
            branch: "other".to_string(),
            onto: vec![
                "main".to_string(),
                "master".to_string(),
                "aaa".to_string(),
                "dev".to_string(),
                "topic".to_string()
            ],
        }
    );
    typed(&mut app, &ports, "5");
    run(&mut app, &ports, &[Event::InputSubmit]);
    assert_eq!(
        actions(&ports),
        vec!["Info: rebase other onto topic", "rebase other onto topic"]
    );
    run(
        &mut app,
        &ports,
        &[Event::Rebase, Event::Input('9'), Event::InputSubmit],
    );
    assert!(matches!(app.mode(), Mode::Prompt(_)));
}

#[test]
fn rebase_problems() {
    let ports: TestPorts = labelled_ports(vec![]);
    let mut app: App = loaded_app(&ports);
    run(&mut app, &ports, &[Event::Rebase]);
    let message: &str = "No local branch at this commit to rebase";
    assert_eq!(actions(&ports), vec![format!("Error: {message}")]);
    assert_eq!(app.note(), &Note::Problem(message.to_string()));
    let alone: TestPorts = labelled_ports(vec![local("only")]);
    let mut alone_app: App = loaded_app(&alone);
    run(&mut alone_app, &alone, &[Event::Rebase]);
    assert_eq!(
        alone_app.note(),
        &Note::Info("No other local branch to rebase onto".to_string())
    );
    let failing: TestPorts = TestPorts {
        branch_fails: true,
        ..rebase_ports(vec![local("topic")])
    };
    let mut failing_app: App = loaded_app(&failing);
    run(
        &mut failing_app,
        &failing,
        &[Event::Rebase, Event::Input('1'), Event::InputSubmit],
    );
    assert_eq!(
        failing_app.note(),
        &Note::Problem("Rebase failed, see the Log view; resolve it outside mgitui".to_string())
    );
    let ports: TestPorts = rebase_ports(vec![local("topic")]);
    let mut app: App = loaded_app(&ports);
    run(&mut app, &ports, &[Event::Rebase]);
    typed(&mut app, &ports, "99");
    run(&mut app, &ports, &[Event::InputSubmit]);
    assert_eq!(
        app.note(),
        &Note::Problem("Choice is not listed".to_string())
    );
}
