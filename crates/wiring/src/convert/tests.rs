use super::{
    event, input_mode, layer_name, log_view, logic_view, screen, ui_level, ui_view, viewports,
};
use logic::app::{App, Mode, Viewports};
use logic::event::Event as L;
use logic::event::ViewId as LV;
use logic::log::{Layer, LogEntry, LogLevel};
use ui::event::Event as U;
use ui::event::ViewId as UV;

struct NoPorts;

impl logic::ports::Ports for NoPorts {
    fn log(&self, _level: LogLevel, _message: &str) {}

    fn validate_search(&self, input: &str) -> Result<logic::ports::SearchText, String> {
        Ok(logic::ports::SearchText(input.to_string()))
    }
}

#[test]
fn every_event_converts() {
    let cases: [(U, L); 26] = [
        (U::SwitchView(UV::Log), L::SwitchView(LV::Log)),
        (U::Quit, L::Quit),
        (U::ConfirmQuit, L::ConfirmQuit),
        (U::CancelQuit, L::CancelQuit),
        (U::Reload, L::Reload),
        (U::MoveDown, L::MoveDown),
        (U::MoveUp, L::MoveUp),
        (U::Top, L::Top),
        (U::Bottom, L::Bottom),
        (U::NextPane, L::NextPane),
        (U::StartSearch, L::StartSearch),
        (U::SearchNext, L::SearchNext),
        (U::SearchPrevious, L::SearchPrevious),
        (U::Cancel, L::Cancel),
        (U::Activate, L::Activate),
        (U::CreateBranch, L::CreateBranch),
        (U::Checkout, L::Checkout),
        (U::SwitchList, L::SwitchList),
        (U::StartSelection, L::StartSelection),
        (U::EditMessage, L::EditMessage),
        (U::Commit, L::Commit),
        (U::ToggleAmend, L::ToggleAmend),
        (U::Input('x'), L::Input('x')),
        (U::InputBackspace, L::InputBackspace),
        (U::InputSubmit, L::InputSubmit),
        (U::InputCancel, L::InputCancel),
    ];
    for (from, to) in cases {
        assert_eq!(event(from), to);
    }
}

#[test]
fn views_convert_both_ways() {
    for (u, l) in [
        (UV::Branches, LV::Branches),
        (UV::Diff, LV::Diff),
        (UV::Log, LV::Log),
    ] {
        assert_eq!(logic_view(u), l);
        assert_eq!(ui_view(l), u);
    }
}

#[test]
fn modes_convert() {
    let mut app: App = App::new();
    assert_eq!(input_mode(app.mode()), ui::event::InputMode::Normal);
    app.handle(L::StartSearch, &NoPorts);
    assert_eq!(input_mode(app.mode()), ui::event::InputMode::Text);
    assert_eq!(
        input_mode(&Mode::QuitConfirm),
        ui::event::InputMode::QuitConfirm
    );
}

#[test]
fn layers_and_levels() {
    let layers: [(Layer, &str); 5] = [
        (Layer::Git, "git"),
        (Layer::Logic, "logic"),
        (Layer::Ui, "ui"),
        (Layer::Validation, "validation"),
        (Layer::Wiring, "wiring"),
    ];
    for (layer, name) in layers {
        assert_eq!(layer_name(layer), name);
    }
    let levels: [(LogLevel, ui::log::LogLevel); 4] = [
        (LogLevel::Error, ui::log::LogLevel::Error),
        (LogLevel::Warn, ui::log::LogLevel::Warn),
        (LogLevel::Info, ui::log::LogLevel::Info),
        (LogLevel::Debug, ui::log::LogLevel::Debug),
    ];
    for (level, ui) in levels {
        assert_eq!(ui_level(level), ui);
    }
}

#[test]
fn viewports_follow_layout() {
    let size: ui::layout::Size = ui::layout::Size {
        width: 80,
        height: 24,
    };
    assert_eq!(viewports(size), Viewports { log: 21 });
}

#[test]
fn startup_screen() {
    let s: ui::screen::Screen = screen(&App::new());
    assert_eq!(s.tabs.active, UV::Branches);
    assert_eq!(s.status, ui::screen::Status::Empty);
    assert_eq!(s.body, ui::screen::Body::Branches);
}

#[test]
fn log_screen_shows_visible_rows_and_status() {
    let mut app: App = App::new();
    app.set_viewports(Viewports { log: 2 });
    for message in ["a", "b", "c"] {
        app.push_log(&LogEntry {
            time: 5,
            layer: Layer::Git,
            level: LogLevel::Info,
            message: message.to_string(),
        });
    }
    app.handle(L::SwitchView(LV::Log), &NoPorts);
    app.handle(L::StartSearch, &NoPorts);
    app.handle(L::Input('a'), &NoPorts);
    let prompt: ui::screen::Screen = screen(&app);
    assert_eq!(
        prompt.status,
        ui::screen::Status::Prompt {
            label: "/".to_string(),
            text: "a".to_string()
        }
    );
    assert!(matches!(prompt.body, ui::screen::Body::Log(_)));
    let shown: Vec<(String, bool)> = log_view(&app)
        .rows
        .iter()
        .map(|r: &ui::views::log::LogRow| (r.message.clone(), r.selected))
        .collect();
    assert_eq!(
        shown,
        vec![("b".to_string(), false), ("c".to_string(), true)]
    );
    app.handle(L::InputSubmit, &NoPorts);
    assert_eq!(
        screen(&app).status,
        ui::screen::Status::Note("search wrapped".to_string())
    );
    app.handle(L::StartSearch, &NoPorts);
    app.handle(L::Input('z'), &NoPorts);
    app.handle(L::InputSubmit, &NoPorts);
    assert_eq!(
        screen(&app).status,
        ui::screen::Status::Note("Pattern not found: z".to_string())
    );
    app.handle(L::SwitchView(LV::Diff), &NoPorts);
    assert_eq!(screen(&app).body, ui::screen::Body::Diff);
}
