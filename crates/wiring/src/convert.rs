//! Conversions between the types of the logic and ui layers.

use logic::app::{App, Mode};

/// The logic event for a ui event.
#[must_use]
pub fn event(event: ui::event::Event) -> logic::event::Event {
    use logic::event::Event as L;
    use ui::event::Event as U;
    match event {
        U::SwitchView(view) => L::SwitchView(logic_view(view)),
        U::Quit => L::Quit,
        U::ConfirmQuit => L::ConfirmQuit,
        U::CancelQuit => L::CancelQuit,
        U::Reload => L::Reload,
        U::MoveDown => L::MoveDown,
        U::MoveUp => L::MoveUp,
        U::Top => L::Top,
        U::Bottom => L::Bottom,
        U::NextPane => L::NextPane,
        U::StartSearch => L::StartSearch,
        U::SearchNext => L::SearchNext,
        U::SearchPrevious => L::SearchPrevious,
        U::Cancel => L::Cancel,
        U::Activate => L::Activate,
        U::CreateBranch => L::CreateBranch,
        U::Checkout => L::Checkout,
        U::SwitchList => L::SwitchList,
        U::StartSelection => L::StartSelection,
        U::EditMessage => L::EditMessage,
        U::Commit => L::Commit,
        U::ToggleAmend => L::ToggleAmend,
        U::Input(c) => L::Input(c),
        U::InputBackspace => L::InputBackspace,
        U::InputSubmit => L::InputSubmit,
        U::InputCancel => L::InputCancel,
    }
}

/// The logic view for a ui view.
#[must_use]
pub fn logic_view(view: ui::event::ViewId) -> logic::event::ViewId {
    match view {
        ui::event::ViewId::Branches => logic::event::ViewId::Branches,
        ui::event::ViewId::Diff => logic::event::ViewId::Diff,
        ui::event::ViewId::Log => logic::event::ViewId::Log,
    }
}

/// The ui view for a logic view.
#[must_use]
pub fn ui_view(view: logic::event::ViewId) -> ui::event::ViewId {
    match view {
        logic::event::ViewId::Branches => ui::event::ViewId::Branches,
        logic::event::ViewId::Diff => ui::event::ViewId::Diff,
        logic::event::ViewId::Log => ui::event::ViewId::Log,
    }
}

/// The ui input mode for a logic mode.
#[must_use]
pub fn input_mode(mode: &Mode) -> ui::event::InputMode {
    match mode {
        Mode::Normal => ui::event::InputMode::Normal,
        Mode::QuitConfirm => ui::event::InputMode::QuitConfirm,
    }
}

/// The screen model for the application state.
#[must_use]
pub fn screen(app: &App) -> ui::screen::Screen {
    ui::screen::Screen {
        tabs: ui::screen::Tabs {
            active: ui_view(app.view()),
            diff_has_changes: false,
            log_has_problems: app.log().has_unseen_problems(),
        },
        status: status(app),
        body: match app.view() {
            logic::event::ViewId::Branches => ui::screen::Body::Branches,
            logic::event::ViewId::Diff => ui::screen::Body::Diff,
            logic::event::ViewId::Log => ui::screen::Body::Log,
        },
    }
}

fn status(app: &App) -> ui::screen::Status {
    match app.mode() {
        Mode::QuitConfirm => ui::screen::Status::Prompt {
            label: "Quit? y/n".to_string(),
            text: String::new(),
        },
        Mode::Normal => ui::screen::Status::Empty,
    }
}

#[cfg(test)]
mod tests {
    use super::{event, input_mode, logic_view, screen, ui_view};
    use logic::app::{App, Mode};
    use logic::event::Event as L;
    use logic::event::ViewId as LV;
    use ui::event::Event as U;
    use ui::event::ViewId as UV;

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
        assert_eq!(input_mode(&Mode::Normal), ui::event::InputMode::Normal);
        assert_eq!(
            input_mode(&Mode::QuitConfirm),
            ui::event::InputMode::QuitConfirm
        );
    }

    #[test]
    fn startup_screen() {
        let s: ui::screen::Screen = screen(&App::new());
        assert_eq!(s.tabs.active, UV::Branches);
        assert_eq!(s.status, ui::screen::Status::Empty);
        assert_eq!(s.body, ui::screen::Body::Branches);
    }
}
