use super::{App, Confirmation, Mode, Note, Prompt, PromptKind};
use crate::event::Event;
use crate::model::{ChangeKind, ChangedFile, Label, Refs, WorkStatus};
use crate::test_ports::{TestPorts, linear_history, modified};

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

/// The recorded branch actions and log messages, without the reloads.
fn actions(ports: &TestPorts) -> Vec<String> {
    ports
        .calls()
        .into_iter()
        .filter(|c: &String| {
            c.starts_with("create")
                || c.starts_with("checkout")
                || c.starts_with("cherry-pick")
                || c.starts_with("delete")
                || c.starts_with("remote")
                || c.starts_with("fetch")
                || c.starts_with("reset")
                || c.starts_with("show-commit")
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
                interactive: false,
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
                interactive: false,
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
            interactive: false,
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
        &Note::Info("No other branch to rebase onto".to_string())
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
    let unlisted: TestPorts = rebase_ports(vec![local("topic")]);
    let mut unlisted_app: App = loaded_app(&unlisted);
    run(&mut unlisted_app, &unlisted, &[Event::Rebase]);
    typed(&mut unlisted_app, &unlisted, "99");
    run(&mut unlisted_app, &unlisted, &[Event::InputSubmit]);
    assert_eq!(
        unlisted_app.note(),
        &Note::Problem("Choice is not listed".to_string())
    );
}

#[test]
fn interactive_rebase_uses_the_same_prompts() {
    let ports: TestPorts = rebase_ports(vec![local("topic"), local("other")]);
    let mut app: App = loaded_app(&ports);
    run(&mut app, &ports, &[Event::InteractiveRebase]);
    assert!(matches!(
        app.mode(),
        Mode::Prompt(Prompt {
            kind: PromptKind::RebaseBranch {
                interactive: true,
                ..
            },
            ..
        })
    ));
    run(&mut app, &ports, &[Event::Input('1'), Event::InputSubmit]);
    assert!(matches!(
        app.mode(),
        Mode::Prompt(Prompt {
            kind: PromptKind::RebaseOnto {
                interactive: true,
                ..
            },
            ..
        })
    ));
    run(&mut app, &ports, &[Event::Input('1'), Event::InputSubmit]);
    assert_eq!(
        actions(&ports),
        vec![
            "Info: interactive rebase topic onto main",
            "rebase -i topic onto main"
        ]
    );
    assert_eq!(
        app.note(),
        &Note::Info("Interactively rebased topic onto main".to_string())
    );
}

fn remote(remote: &str, branch: &str) -> Label {
    Label::Remote {
        remote: remote.to_string(),
        branch: branch.to_string(),
    }
}

/// `topic` at c2 with local `main` and `dev`, and remote branches at c1.
fn remote_ports() -> TestPorts {
    let ports: TestPorts = labelled_ports(vec![local("topic")]);
    if let Some(refs) = ports.refs.borrow_mut().as_mut() {
        for label in [
            local("dev"),
            local("main"),
            remote("upstream", "main"),
            remote("origin", "old-feature"),
            remote("origin", "master"),
            remote("origin", "main"),
            Label::Tag {
                name: "v1".to_string(),
            },
        ] {
            refs.labels.push(("c1".to_string(), label));
        }
    }
    ports
}

fn onto_choices(app: &App) -> Vec<String> {
    match app.mode() {
        Mode::Prompt(Prompt {
            kind: PromptKind::RebaseOnto { onto, .. },
            ..
        }) => onto.clone(),
        other => vec![format!("{other:?}")],
    }
}

#[test]
fn rebase_offers_remote_main_and_master_only() {
    let ports: TestPorts = remote_ports();
    let mut app: App = loaded_app(&ports);
    run(&mut app, &ports, &[Event::Rebase]);
    assert_eq!(
        onto_choices(&app),
        vec![
            "main",
            "origin/main",
            "origin/master",
            "upstream/main",
            "dev"
        ]
    );
    run(&mut app, &ports, &[Event::Input('2'), Event::InputSubmit]);
    assert_eq!(
        actions(&ports),
        vec![
            "Info: rebase topic onto origin/main",
            "rebase topic onto origin/main"
        ]
    );
}

#[test]
fn interactive_rebase_offers_no_remote_branches() {
    let ports: TestPorts = remote_ports();
    let mut app: App = loaded_app(&ports);
    run(&mut app, &ports, &[Event::InteractiveRebase]);
    assert_eq!(onto_choices(&app), vec!["main", "dev"]);
    run(&mut app, &ports, &[Event::InputCancel]);
    assert_eq!(onto_choices(&app), vec!["Normal"]);
}

#[test]
fn delete_the_only_branch_that_is_not_checked_out() {
    let ports: TestPorts = labelled_ports(vec![
        Label::Local {
            name: "main".to_string(),
            head: true,
        },
        local("old"),
    ]);
    let mut app: App = loaded_app(&ports);
    run(&mut app, &ports, &[Event::DeleteBranch]);
    assert_eq!(
        app.mode(),
        &Mode::Confirm(Confirmation::DeleteBranch {
            name: "old".to_string()
        })
    );
    assert_eq!(actions(&ports), Vec::<String>::new());
    run(&mut app, &ports, &[Event::No]);
    assert_eq!(app.mode(), &Mode::Normal);
    assert_eq!(actions(&ports), Vec::<String>::new());
    run(&mut app, &ports, &[Event::DeleteBranch, Event::Yes]);
    assert_eq!(
        actions(&ports),
        vec!["Info: delete branch old", "delete old"]
    );
    assert_eq!(app.note(), &Note::Info("Deleted branch old".to_string()));
}

#[test]
fn delete_asks_which_branch() {
    let ports: TestPorts = labelled_ports(vec![local("a"), local("b")]);
    let mut app: App = loaded_app(&ports);
    run(&mut app, &ports, &[Event::DeleteBranch]);
    assert_eq!(
        app.mode(),
        &Mode::Prompt(Prompt {
            kind: PromptKind::DeleteBranch {
                branches: vec!["a".to_string(), "b".to_string()],
            },
            text: String::new()
        })
    );
    run(
        &mut app,
        &ports,
        &[Event::Input('2'), Event::InputSubmit, Event::Yes],
    );
    assert_eq!(actions(&ports), vec!["Info: delete branch b", "delete b"]);
    run(
        &mut app,
        &ports,
        &[Event::DeleteBranch, Event::Input('9'), Event::InputSubmit],
    );
    assert!(matches!(app.mode(), Mode::Prompt(_)));
}

#[test]
fn delete_problems() {
    let head_only: TestPorts = labelled_ports(vec![Label::Local {
        name: "main".to_string(),
        head: true,
    }]);
    let mut app: App = loaded_app(&head_only);
    run(&mut app, &head_only, &[Event::DeleteBranch]);
    let checked_out: &str = "The checked out branch cannot be deleted";
    assert_eq!(actions(&head_only), vec![format!("Error: {checked_out}")]);
    assert_eq!(app.note(), &Note::Problem(checked_out.to_string()));
    let none: TestPorts = labelled_ports(vec![]);
    let mut none_app: App = loaded_app(&none);
    run(&mut none_app, &none, &[Event::DeleteBranch]);
    assert_eq!(
        none_app.note(),
        &Note::Problem("No local branch at this commit to delete".to_string())
    );
    let failing: TestPorts = TestPorts {
        branch_fails: true,
        ..labelled_ports(vec![local("old")])
    };
    let mut failing_app: App = loaded_app(&failing);
    run(
        &mut failing_app,
        &failing,
        &[Event::DeleteBranch, Event::Yes],
    );
    assert_eq!(
        failing_app.note(),
        &Note::Problem("Deleting the branch failed, see the Log view".to_string())
    );
}

#[test]
fn remote_update_fetches_and_reloads() {
    let ports: TestPorts = labelled_ports(vec![]);
    let mut app: App = loaded_app(&ports);
    run(&mut app, &ports, &[Event::RemoteUpdate]);
    assert_eq!(
        actions(&ports),
        vec!["Info: remote update", "remote update"]
    );
    assert!(ports.calls().contains(&"refs".to_string()));
    assert_eq!(app.note(), &Note::Info("Updated remotes".to_string()));
    let failing: TestPorts = TestPorts {
        branch_fails: true,
        ..labelled_ports(vec![])
    };
    run(&mut app, &failing, &[Event::RemoteUpdate]);
    assert_eq!(
        app.note(),
        &Note::Problem("Remote update failed, see the Log view".to_string())
    );
}

#[test]
fn fetch_all_prune_fetches_and_reloads() {
    let ports: TestPorts = labelled_ports(vec![]);
    let mut app: App = loaded_app(&ports);
    run(&mut app, &ports, &[Event::FetchAllPrune]);
    assert_eq!(
        actions(&ports),
        vec!["Info: fetch --all --prune", "fetch --all --prune"]
    );
    assert!(ports.calls().contains(&"refs".to_string()));
    assert_eq!(
        app.note(),
        &Note::Info("Fetched all remotes pruning stale remote branches".to_string())
    );
    let failing: TestPorts = TestPorts {
        branch_fails: true,
        ..labelled_ports(vec![])
    };
    run(&mut app, &failing, &[Event::FetchAllPrune]);
    assert_eq!(
        app.note(),
        &Note::Problem("Fetch all prune failed, see the Log view".to_string())
    );
}

fn head(name: &str) -> Label {
    Label::Local {
        name: name.to_string(),
        head: true,
    }
}

fn changed(path: &str, kind: ChangeKind) -> ChangedFile {
    ChangedFile {
        path: path.to_string(),
        original_path: None,
        kind,
        mode_changed: false,
    }
}

#[test]
fn reset_branch_to_selected_commit_after_confirming() {
    let ports: TestPorts = labelled_ports(vec![head("main")]);
    *ports.status.borrow_mut() = Some(WorkStatus {
        staged: vec![],
        unstaged: vec![changed("new.txt", ChangeKind::Untracked)],
    });
    let mut app: App = loaded_app(&ports);
    run(&mut app, &ports, &[Event::MoveDown, Event::ResetBranch]);
    assert_eq!(
        app.mode(),
        &Mode::Confirm(Confirmation::ResetBranch {
            branch: "main".to_string(),
            hash: "c1".to_string(),
        })
    );
    run(&mut app, &ports, &[Event::No]);
    assert_eq!(app.mode(), &Mode::Normal);
    assert_eq!(actions(&ports), Vec::<String>::new());
    run(&mut app, &ports, &[Event::ResetBranch, Event::Yes]);
    assert_eq!(
        actions(&ports),
        vec!["Info: reset main to c1", "reset --keep c1"]
    );
    assert!(ports.calls().contains(&"refs".to_string()));
    assert_eq!(app.note(), &Note::Info("Reset main to c1".to_string()));
}

#[test]
fn reset_branch_problems() {
    let detached: TestPorts = labelled_ports(vec![local("main")]);
    let mut app: App = loaded_app(&detached);
    run(&mut app, &detached, &[Event::ResetBranch]);
    let no_branch: &str = "No branch is checked out to reset";
    assert_eq!(actions(&detached), vec![format!("Error: {no_branch}")]);
    assert_eq!(app.note(), &Note::Problem(no_branch.to_string()));
    assert_eq!(app.mode(), &Mode::Normal);

    for status in [
        WorkStatus {
            staged: vec![modified("a.txt")],
            unstaged: vec![],
        },
        WorkStatus {
            staged: vec![],
            unstaged: vec![modified("a.txt")],
        },
    ] {
        let changes: TestPorts = labelled_ports(vec![head("main")]);
        *changes.status.borrow_mut() = Some(status);
        let mut changes_app: App = loaded_app(&changes);
        run(&mut changes_app, &changes, &[Event::ResetBranch]);
        let uncommitted: &str = "Commit or discard uncommitted changes before resetting";
        assert_eq!(actions(&changes), vec![format!("Error: {uncommitted}")]);
        assert_eq!(changes_app.note(), &Note::Problem(uncommitted.to_string()));
        assert_eq!(changes_app.mode(), &Mode::Normal);
    }

    let no_status: TestPorts = labelled_ports(vec![head("main")]);
    let mut no_status_app: App = loaded_app(&no_status);
    *no_status.status.borrow_mut() = None;
    run(&mut no_status_app, &no_status, &[Event::ResetBranch]);
    assert_eq!(
        no_status_app.note(),
        &Note::Problem("Reading the status failed, see the Log view".to_string())
    );
    assert_eq!(no_status_app.mode(), &Mode::Normal);

    let failing: TestPorts = TestPorts {
        branch_fails: true,
        ..labelled_ports(vec![head("main")])
    };
    let mut failing_app: App = loaded_app(&failing);
    run(
        &mut failing_app,
        &failing,
        &[Event::ResetBranch, Event::Yes],
    );
    assert_eq!(
        failing_app.note(),
        &Note::Problem("Reset failed, see the Log view".to_string())
    );

    let empty: TestPorts = TestPorts::new();
    let mut empty_app: App = loaded_app(&empty);
    run(&mut empty_app, &empty, &[Event::ResetBranch]);
    assert_eq!(
        empty_app.note(),
        &Note::Info("No commit selected".to_string())
    );
}

#[test]
fn show_selected_commit() {
    let ports: TestPorts = labelled_ports(vec![]);
    let mut app: App = loaded_app(&ports);
    run(&mut app, &ports, &[Event::MoveDown, Event::ShowCommit]);
    assert_eq!(actions(&ports), vec!["Info: show c1", "show-commit c1"]);
    assert!(!ports.calls().contains(&"refs".to_string()));
    assert_eq!(app.note(), &Note::None);
    let failing: TestPorts = TestPorts {
        details_fail: true,
        ..labelled_ports(vec![])
    };
    run(&mut app, &failing, &[Event::ShowCommit]);
    assert_eq!(
        app.note(),
        &Note::Problem("Showing the commit failed, see the Log view".to_string())
    );
    let empty: TestPorts = TestPorts::new();
    let mut empty_app: App = loaded_app(&empty);
    run(&mut empty_app, &empty, &[Event::ShowCommit]);
    assert_eq!(
        empty_app.note(),
        &Note::Info("No commit selected".to_string())
    );
}
