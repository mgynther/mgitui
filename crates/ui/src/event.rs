//! Turning key presses into events, depending on the active view and input mode.

use crate::key::Key;

/// The views of the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewId {
    /// The branch graph.
    Branches,
    /// Staging and committing.
    Diff,
    /// The application log.
    Log,
}

/// What key presses are used for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    /// Keys are commands.
    Normal,
    /// Keys are typed into a prompt.
    Text,
    /// A y/n confirmation is shown.
    Confirm,
}

/// Something the user asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// Show another view.
    SwitchView(ViewId),
    /// Ask whether to quit.
    Quit,
    /// Answer yes to a y/n confirmation.
    Yes,
    /// Answer no to a y/n confirmation.
    No,
    /// Reload everything from git.
    Reload,
    /// Move the selection down.
    MoveDown,
    /// Move the selection up.
    MoveUp,
    /// Jump to the top.
    Top,
    /// Jump to the bottom.
    Bottom,
    /// Focus the next pane.
    NextPane,
    /// Start typing a search.
    StartSearch,
    /// Jump to the next search match.
    SearchNext,
    /// Jump to the previous search match.
    SearchPrevious,
    /// Clear the current selection.
    Cancel,
    /// Enter: stage, unstage or confirm.
    Activate,
    /// Create a branch at the selected commit.
    CreateBranch,
    /// Check out a branch of the selected commit.
    Checkout,
    /// Cherry-pick the selected commit onto the checked out branch.
    CherryPick,
    /// Fetch all remotes with `git remote update`.
    RemoteUpdate,
    /// Fetch all remotes pruning stale remote branches.
    FetchAllPrune,
    /// Select the next commit with a local or remote `main` or `master` branch.
    NextMainBranch,
    /// Show the selected commit with `git show` in the user's pager.
    ShowCommit,
    /// Reset the checked out branch to the selected commit.
    ResetBranch,
    /// Delete a local branch of the selected commit that is not checked out.
    DeleteBranch,
    /// Rebase a branch of the selected commit onto another branch.
    Rebase,
    /// Interactively rebase a branch of the selected commit onto another branch.
    InteractiveRebase,
    /// Switch between the unstaged and staged lists.
    SwitchList,
    /// Discard the unstaged changes of the selected file, deleting it when untracked.
    DiscardFile,
    /// Start selecting lines.
    StartSelection,
    /// Edit the commit message.
    EditMessage,
    /// Commit.
    Commit,
    /// Toggle amend mode.
    ToggleAmend,
    /// A character typed into a prompt.
    Input(char),
    /// Delete the last character of a prompt.
    InputBackspace,
    /// Submit a prompt.
    InputSubmit,
    /// Close a prompt without submitting.
    InputCancel,
}

/// The event for `key` in `view` and `mode`, if the key means anything there.
#[must_use]
pub fn event_for(key: Key, view: ViewId, mode: InputMode) -> Option<Event> {
    match mode {
        InputMode::Text => text_event(key),
        InputMode::Confirm => Some(if key == Key::Char('y') {
            Event::Yes
        } else {
            Event::No
        }),
        InputMode::Normal => common_event(key).or_else(|| view_event(key, view)),
    }
}

fn text_event(key: Key) -> Option<Event> {
    match key {
        Key::Char(c) => Some(Event::Input(c)),
        Key::Backspace => Some(Event::InputBackspace),
        Key::Enter => Some(Event::InputSubmit),
        Key::Esc => Some(Event::InputCancel),
        Key::F(_) | Key::Ctrl(_) => None,
    }
}

fn common_event(key: Key) -> Option<Event> {
    match key {
        Key::F(1) => Some(Event::SwitchView(ViewId::Branches)),
        Key::F(2) => Some(Event::SwitchView(ViewId::Diff)),
        Key::F(3) => Some(Event::SwitchView(ViewId::Log)),
        Key::Char('q') => Some(Event::Quit),
        Key::Char('r') => Some(Event::Reload),
        Key::Char('j') => Some(Event::MoveDown),
        Key::Char('k') => Some(Event::MoveUp),
        Key::Char('g') => Some(Event::Top),
        Key::Char('G') => Some(Event::Bottom),
        Key::Char('/') => Some(Event::StartSearch),
        Key::Char('n') => Some(Event::SearchNext),
        Key::Char('m') => Some(Event::SearchPrevious),
        Key::Esc => Some(Event::Cancel),
        _ => None,
    }
}

fn view_event(key: Key, view: ViewId) -> Option<Event> {
    match (view, key) {
        (ViewId::Branches | ViewId::Diff, Key::Char('w')) => Some(Event::NextPane),
        (ViewId::Branches | ViewId::Diff, Key::Enter) => Some(Event::Activate),
        (ViewId::Branches, Key::Char('c')) => Some(Event::CreateBranch),
        (ViewId::Branches, Key::Char('h')) => Some(Event::Checkout),
        (ViewId::Branches, Key::Char('p')) => Some(Event::CherryPick),
        (ViewId::Branches, Key::Char('u')) => Some(Event::RemoteUpdate),
        (ViewId::Branches, Key::Char('U')) => Some(Event::FetchAllPrune),
        (ViewId::Branches, Key::Char('M')) => Some(Event::NextMainBranch),
        (ViewId::Branches, Key::Char('s')) => Some(Event::ShowCommit),
        (ViewId::Branches, Key::Char('R')) => Some(Event::ResetBranch),
        (ViewId::Branches, Key::Char('d')) => Some(Event::DeleteBranch),
        (ViewId::Branches, Key::Char('b')) => Some(Event::Rebase),
        (ViewId::Branches, Key::Char('i')) => Some(Event::InteractiveRebase),
        (ViewId::Diff, Key::Char('s')) => Some(Event::SwitchList),
        (ViewId::Diff, Key::Char('d')) => Some(Event::DiscardFile),
        (ViewId::Diff, Key::Char(' ')) => Some(Event::StartSelection),
        (ViewId::Diff, Key::Char('e')) => Some(Event::EditMessage),
        (ViewId::Diff, Key::Char('c')) => Some(Event::Commit),
        (ViewId::Diff, Key::Char('a')) => Some(Event::ToggleAmend),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{Event, InputMode, ViewId, event_for};
    use crate::key::Key;

    const VIEWS: [ViewId; 3] = [ViewId::Branches, ViewId::Diff, ViewId::Log];

    #[test]
    fn common_keys_work_in_every_view() {
        let cases: [(Key, Event); 13] = [
            (Key::F(1), Event::SwitchView(ViewId::Branches)),
            (Key::F(2), Event::SwitchView(ViewId::Diff)),
            (Key::F(3), Event::SwitchView(ViewId::Log)),
            (Key::Char('q'), Event::Quit),
            (Key::Char('r'), Event::Reload),
            (Key::Char('j'), Event::MoveDown),
            (Key::Char('k'), Event::MoveUp),
            (Key::Char('g'), Event::Top),
            (Key::Char('G'), Event::Bottom),
            (Key::Char('/'), Event::StartSearch),
            (Key::Char('n'), Event::SearchNext),
            (Key::Char('m'), Event::SearchPrevious),
            (Key::Esc, Event::Cancel),
        ];
        for view in VIEWS {
            for (key, event) in cases {
                assert_eq!(event_for(key, view, InputMode::Normal), Some(event));
            }
        }
    }

    #[test]
    fn view_specific_keys() {
        let cases: [(ViewId, Key, Option<Event>); 27] = [
            (ViewId::Branches, Key::Char('w'), Some(Event::NextPane)),
            (ViewId::Diff, Key::Char('w'), Some(Event::NextPane)),
            (ViewId::Log, Key::Char('w'), None),
            (ViewId::Branches, Key::Enter, Some(Event::Activate)),
            (ViewId::Diff, Key::Enter, Some(Event::Activate)),
            (ViewId::Branches, Key::Char('c'), Some(Event::CreateBranch)),
            (ViewId::Branches, Key::Char('h'), Some(Event::Checkout)),
            (ViewId::Branches, Key::Char('p'), Some(Event::CherryPick)),
            (ViewId::Branches, Key::Char('u'), Some(Event::RemoteUpdate)),
            (ViewId::Branches, Key::Char('U'), Some(Event::FetchAllPrune)),
            (
                ViewId::Branches,
                Key::Char('M'),
                Some(Event::NextMainBranch),
            ),
            (ViewId::Diff, Key::Char('M'), None),
            (ViewId::Branches, Key::Char('s'), Some(Event::ShowCommit)),
            (ViewId::Branches, Key::Char('R'), Some(Event::ResetBranch)),
            (ViewId::Branches, Key::Char('d'), Some(Event::DeleteBranch)),
            (ViewId::Branches, Key::Char('b'), Some(Event::Rebase)),
            (
                ViewId::Branches,
                Key::Char('i'),
                Some(Event::InteractiveRebase),
            ),
            (ViewId::Diff, Key::Char('p'), None),
            (ViewId::Diff, Key::Char('s'), Some(Event::SwitchList)),
            (ViewId::Diff, Key::Char('d'), Some(Event::DiscardFile)),
            (ViewId::Log, Key::Char('d'), None),
            (ViewId::Diff, Key::Char(' '), Some(Event::StartSelection)),
            (ViewId::Diff, Key::Char('e'), Some(Event::EditMessage)),
            (ViewId::Diff, Key::Char('c'), Some(Event::Commit)),
            (ViewId::Diff, Key::Char('a'), Some(Event::ToggleAmend)),
            (ViewId::Log, Key::Char('c'), None),
            (ViewId::Diff, Key::F(4), None),
        ];
        for (view, key, event) in cases {
            assert_eq!(
                event_for(key, view, InputMode::Normal),
                event,
                "{view:?} {key:?}"
            );
        }
    }

    #[test]
    fn text_input_keys() {
        let cases: [(Key, Option<Event>); 7] = [
            (Key::Char('q'), Some(Event::Input('q'))),
            (Key::Char(' '), Some(Event::Input(' '))),
            (Key::Backspace, Some(Event::InputBackspace)),
            (Key::Enter, Some(Event::InputSubmit)),
            (Key::Esc, Some(Event::InputCancel)),
            (Key::F(1), None),
            (Key::Ctrl('c'), None),
        ];
        for (key, event) in cases {
            assert_eq!(event_for(key, ViewId::Branches, InputMode::Text), event);
        }
    }

    #[test]
    fn quit_prompt_keys() {
        assert_eq!(
            event_for(Key::Char('y'), ViewId::Log, InputMode::Confirm),
            Some(Event::Yes)
        );
        assert_eq!(
            event_for(Key::Char('n'), ViewId::Log, InputMode::Confirm),
            Some(Event::No)
        );
        assert_eq!(
            event_for(Key::Esc, ViewId::Log, InputMode::Confirm),
            Some(Event::No)
        );
    }
}
