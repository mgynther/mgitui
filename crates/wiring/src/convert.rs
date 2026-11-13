//! Conversions between the types of the logic and ui layers.

use logic::app::{App, Mode, Note, Prompt, PromptKind, Viewports};
use logic::log::{Layer, LogEntry, LogLevel};

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
        Mode::Prompt(_) => ui::event::InputMode::Text,
    }
}

/// How many rows each list has for a terminal of `size`.
#[must_use]
pub fn viewports(size: ui::layout::Size) -> Viewports {
    Viewports {
        log: ui::layout::log_layout(size).list.content().height,
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
            logic::event::ViewId::Log => ui::screen::Body::Log(log_view(app)),
        },
    }
}

fn status(app: &App) -> ui::screen::Status {
    match (app.mode(), app.note()) {
        (Mode::QuitConfirm, _) => ui::screen::Status::Prompt {
            label: "Quit? y/n".to_string(),
            text: String::new(),
        },
        (Mode::Prompt(prompt), _) => ui::screen::Status::Prompt {
            label: prompt_label(prompt),
            text: prompt.text.clone(),
        },
        (Mode::Normal, Note::None) => ui::screen::Status::Empty,
        (Mode::Normal, Note::Info(text)) => ui::screen::Status::Note(text.clone()),
        (Mode::Normal, Note::Problem(text)) => ui::screen::Status::Problem(text.clone()),
    }
}

fn prompt_label(prompt: &Prompt) -> String {
    match prompt.kind {
        PromptKind::Search => "/".to_string(),
    }
}

fn log_view(app: &App) -> ui::views::log::LogView {
    let cursor: logic::cursor::Cursor = app.log_cursor();
    let rows: Vec<ui::views::log::LogRow> = app
        .log()
        .entries()
        .iter()
        .enumerate()
        .skip(cursor.offset())
        .take(app.viewports().log)
        .map(
            |(index, entry): (usize, &LogEntry)| ui::views::log::LogRow {
                time: entry.time,
                layer: layer_name(entry.layer).to_string(),
                level: ui_level(entry.level),
                message: entry.message.clone(),
                selected: index == cursor.selected(),
            },
        )
        .collect();
    ui::views::log::LogView { rows }
}

fn layer_name(layer: Layer) -> &'static str {
    match layer {
        Layer::Git => "git",
        Layer::Logic => "logic",
        Layer::Ui => "ui",
        Layer::Validation => "validation",
        Layer::Wiring => "wiring",
    }
}

fn ui_level(level: LogLevel) -> ui::log::LogLevel {
    match level {
        LogLevel::Error => ui::log::LogLevel::Error,
        LogLevel::Warn => ui::log::LogLevel::Warn,
        LogLevel::Info => ui::log::LogLevel::Info,
        LogLevel::Debug => ui::log::LogLevel::Debug,
    }
}

#[cfg(test)]
mod tests;
