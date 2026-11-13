//! Events the logic layer handles.

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
    /// Delete a local branch of the selected commit that is not checked out.
    DeleteBranch,
    /// Rebase a branch of the selected commit onto another branch.
    Rebase,
    /// Interactively rebase a branch of the selected commit onto another branch.
    InteractiveRebase,
    /// Switch between the unstaged and staged lists.
    SwitchList,
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
