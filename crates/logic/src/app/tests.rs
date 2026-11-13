use super::{App, Confirmation, Mode, Note, Prompt, PromptKind, Viewports};
use crate::event::{Event, ViewId};
use crate::log::{Layer, LogEntry, LogLevel};
use crate::ports::Ports;
use crate::test_ports::TestPorts;

fn entry(level: LogLevel, message: &str) -> LogEntry {
    LogEntry {
        time: 0,
        layer: Layer::Git,
        level,
        message: message.to_string(),
    }
}

fn run(app: &mut App, events: &[Event]) {
    let ports: TestPorts = TestPorts::new();
    for event in events {
        app.handle(*event, &ports);
    }
}

fn typed(text: &str) -> Vec<Event> {
    text.chars().map(Event::Input).collect()
}

fn log_app(messages: &[&str]) -> App {
    let mut app: App = App::new();
    app.set_viewports(Viewports {
        log: 3,
        ..Viewports::default()
    });
    for message in messages {
        app.push_log(&entry(LogLevel::Info, message));
    }
    run(&mut app, &[Event::SwitchView(ViewId::Log)]);
    app
}

#[test]
fn starts_in_branches_view() {
    let app: App = App::default();
    assert_eq!(app.view(), ViewId::Branches);
    assert_eq!(app.mode(), &Mode::Normal);
    assert_eq!(app.note(), &Note::None);
    assert!(!app.should_quit());
}

#[test]
fn quit_needs_confirmation() {
    let mut app: App = App::new();
    run(&mut app, &[Event::Quit]);
    assert_eq!(app.mode(), &Mode::Confirm(Confirmation::Quit));
    run(&mut app, &[Event::No]);
    assert_eq!(app.mode(), &Mode::Normal);
    assert!(!app.should_quit());
    run(&mut app, &[Event::Quit, Event::Yes]);
    assert!(app.should_quit());
}

#[test]
fn switching_views_and_unseen_problems() {
    let mut app: App = App::new();
    app.push_log(&entry(LogLevel::Error, "bad"));
    assert!(app.log().has_unseen_problems());
    run(&mut app, &[Event::SwitchView(ViewId::Log)]);
    assert_eq!(app.view(), ViewId::Log);
    assert!(app.log().has_unseen_problems());
    app.push_log(&entry(LogLevel::Error, "bad"));
    run(&mut app, &[Event::SwitchView(ViewId::Log)]);
    assert!(app.log().has_unseen_problems());
    run(&mut app, &[Event::SwitchView(ViewId::Diff)]);
    assert!(!app.log().has_unseen_problems());
    run(&mut app, &[Event::SwitchView(ViewId::Branches)]);
    assert_eq!(app.view(), ViewId::Branches);
}

#[test]
fn log_selection_follows_new_lines_at_the_end() {
    let mut app: App = log_app(&["a", "b", "c", "d"]);
    assert_eq!(app.log_cursor().selected(), 3);
    assert_eq!(app.log_cursor().offset(), 1);
    run(&mut app, &[Event::MoveUp]);
    app.push_log(&entry(LogLevel::Info, "e"));
    assert_eq!(app.log_cursor().selected(), 2);
}

#[test]
fn log_navigation() {
    let mut app: App = log_app(&["a", "b", "c", "d"]);
    run(&mut app, &[Event::Top]);
    assert_eq!(app.log_cursor().selected(), 0);
    run(&mut app, &[Event::MoveDown, Event::MoveDown]);
    assert_eq!(app.log_cursor().selected(), 2);
    run(&mut app, &[Event::Bottom]);
    assert_eq!(app.log_cursor().selected(), 3);
    run(&mut app, &[Event::Cancel]);
    assert_eq!(app.log_cursor().selected(), 3);
    app.set_viewports(Viewports {
        log: 1,
        ..Viewports::default()
    });
    assert_eq!(app.log_cursor().offset(), 3);
    assert_eq!(app.viewports().log, 1);
}

#[test]
fn prompt_editing_and_cancel() {
    let mut app: App = log_app(&[]);
    run(
        &mut app,
        &[Event::StartSearch, Event::Input('a'), Event::Input('b')],
    );
    run(&mut app, &[Event::InputBackspace, Event::MoveDown]);
    assert_eq!(
        app.mode(),
        &Mode::Prompt(Prompt {
            kind: PromptKind::Search,
            text: "a".to_string()
        })
    );
    run(&mut app, &[Event::InputCancel]);
    assert_eq!(app.mode(), &Mode::Normal);
}

#[test]
fn log_search_wraps_and_repeats() {
    let mut app: App = log_app(&["git status", "other", "git diff", "x"]);
    run(&mut app, &[Event::Top, Event::StartSearch]);
    run(&mut app, &typed("GIT"));
    run(&mut app, &[Event::InputSubmit]);
    assert_eq!(app.log_cursor().selected(), 2);
    assert_eq!(app.note(), &Note::None);
    run(&mut app, &[Event::SearchNext]);
    assert_eq!(app.log_cursor().selected(), 0);
    assert_eq!(app.note(), &Note::Info("search wrapped".to_string()));
    run(&mut app, &[Event::SearchPrevious]);
    assert_eq!(app.log_cursor().selected(), 2);
}

#[test]
fn search_problems() {
    let mut app: App = log_app(&["a"]);
    run(&mut app, &[Event::SearchNext]);
    assert_eq!(app.note(), &Note::Info("No previous search".to_string()));
    run(&mut app, &[Event::StartSearch, Event::InputSubmit]);
    assert_eq!(
        app.note(),
        &Note::Problem("search text is empty".to_string())
    );
    run(
        &mut app,
        &[Event::StartSearch, Event::Input('z'), Event::InputSubmit],
    );
    assert_eq!(app.note(), &Note::Info("Pattern not found: z".to_string()));
}

#[test]
fn search_in_views_without_lists_finds_nothing() {
    let mut app: App = App::new();
    run(
        &mut app,
        &[Event::StartSearch, Event::Input('a'), Event::InputSubmit],
    );
    assert_eq!(app.note(), &Note::Info("Pattern not found: a".to_string()));
    run(&mut app, &[Event::SearchPrevious]);
    assert_eq!(app.note(), &Note::Info("Pattern not found: a".to_string()));
    run(
        &mut app,
        &[
            Event::SwitchView(ViewId::Diff),
            Event::MoveDown,
            Event::SearchNext,
        ],
    );
    assert_eq!(app.note(), &Note::Info("No previous search".to_string()));
    run(
        &mut app,
        &[Event::StartSearch, Event::Input('b'), Event::InputSubmit],
    );
    assert_eq!(app.note(), &Note::Info("Pattern not found: b".to_string()));
}

#[test]
fn branches_view_ignores_list_keys_for_now() {
    let mut app: App = App::new();
    run(&mut app, &[Event::MoveDown]);
    assert_eq!(app.view(), ViewId::Branches);
    assert_eq!(app.mode(), &Mode::Normal);
}

#[test]
fn test_ports_log() {
    let ports: TestPorts = TestPorts::new();
    ports.log(LogLevel::Info, "x");
    assert_eq!(ports.calls(), vec!["Info: x".to_string()]);
}
