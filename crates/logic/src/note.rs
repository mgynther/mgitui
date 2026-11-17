//! What the status line tells about the last event. Logic decides what happened and whether it
//! is a problem; the ui words it. Variant names follow the wording, so that a note and its
//! text are easy to connect.

/// A short message for the status line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Note {
    /// Nothing to show.
    None,
    /// Information.
    Info(Info),
    /// A problem.
    Problem(Problem),
}

/// Information for the status line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Info {
    /// There is no commit to act on.
    NoCommitSelected,
    /// A branch was created.
    CreatedBranch {
        /// The branch name.
        name: String,
    },
    /// The branch to create already exists at the commit it was to be created at.
    BranchAlreadyPointsTo {
        /// The branch name.
        branch: String,
        /// The full commit hash.
        hash: String,
    },
    /// A branch was deleted.
    DeletedBranch {
        /// The branch name.
        name: String,
    },
    /// There is no other branch to rebase onto.
    NoOtherBranchToRebaseOnto,
    /// A branch was rebased.
    Rebased {
        /// The rebased branch.
        branch: String,
        /// The branch it was rebased onto.
        onto: String,
    },
    /// A branch was rebased interactively.
    RebasedInteractively {
        /// The rebased branch.
        branch: String,
        /// The branch it was rebased onto.
        onto: String,
    },
    /// A branch was checked out.
    CheckedOut {
        /// The branch name.
        branch: String,
    },
    /// All remotes were fetched.
    UpdatedRemotes,
    /// All remotes were fetched and stale remote branches pruned.
    FetchedAndPruned,
    /// A branch was reset to a commit.
    ResetBranch {
        /// The branch name.
        branch: String,
        /// The full commit hash.
        hash: String,
    },
    /// A commit was cherry-picked.
    CherryPicked {
        /// The full commit hash.
        hash: String,
    },
    /// No loaded commit has a `main` or `master` branch.
    NoMainOrMasterBranch,
    /// Lines of this file cannot be staged one by one.
    OnlyWholeFilesCanBeStagedHere,
    /// Discarding works only on unstaged changes.
    OnlyUnstagedChangesCanBeDiscarded,
    /// There are no unstaged changes.
    NothingToStage,
    /// The editor exited with an error, so the message was kept as it was.
    EditingMessageCancelled,
    /// The interactive rebase ended without a rebase in progress, as when the todo list is
    /// emptied or Vim quits with `:cq`.
    InteractiveRebaseCancelled,
    /// The selection has no added or removed lines.
    NoChangedLinesSelected,
    /// An untracked file was deleted.
    DeletedFile {
        /// The file's path.
        path: String,
    },
    /// The unstaged changes of a file were discarded.
    DiscardedChanges {
        /// The file's path.
        path: String,
    },
    /// n or m was pressed before any search.
    NoPreviousSearch,
    /// The search found nothing.
    PatternNotFound {
        /// The search term.
        term: String,
    },
    /// The search went past the end or the start of the list.
    SearchWrapped,
}

/// A problem for the status line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// An operation failed; the Log view has the details.
    Failed(Operation),
    /// Typed input is not valid.
    Invalid(InputProblem),
    /// The validated choice is not one of the listed items.
    ChoiceNotListed,
    /// The selected commit has no local branch to check out.
    NoLocalBranchToCheckOut,
    /// The selected commit has no local branch to delete.
    NoLocalBranchToDelete,
    /// The only local branch of the selected commit is checked out.
    CheckedOutBranchCannotBeDeleted,
    /// The selected commit has no local branch to rebase.
    NoLocalBranchToRebase,
    /// `HEAD` is detached.
    NoBranchCheckedOutToReset,
    /// Files git tracks have changes.
    UncommittedChangesBeforeReset,
    /// Files git tracks have changes.
    UncommittedChangesBeforeRebase,
    /// The rebase stopped, for example on a conflict, and is still in progress.
    RebaseStopped,
    /// The selected file has a merge conflict.
    MergeConflict {
        /// The file's path.
        path: String,
    },
    /// Some file has a merge conflict.
    MergeConflictsBeforeStagingAll,
    /// There are no commits yet.
    NothingToAmend,
    /// The amended commit would have no changes.
    AmendedCommitWouldBeEmpty,
}

/// What failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    /// Running `git gc` at startup.
    RunningGc,
    /// Reloading the commits and refs.
    ReloadingCommits,
    /// Loading the message and files of the selected commit.
    LoadingCommit,
    /// Loading the diff of the selected file.
    LoadingDiff,
    /// Loading the next chunk of commits.
    LoadingMoreCommits,
    /// Creating a branch.
    CreatingBranch,
    /// Deleting a branch.
    DeletingBranch,
    /// Rebasing.
    Rebasing,
    /// Checking out a branch.
    CheckingOut,
    /// Fetching all remotes.
    UpdatingRemotes,
    /// Fetching all remotes and pruning stale remote branches.
    FetchingAndPruning,
    /// Showing a commit in the pager.
    ShowingCommit,
    /// Reading the staged and unstaged changes.
    ReadingStatus,
    /// Reading the branches, to check whether a branch exists.
    ReadingBranches,
    /// Checking a conflicted file for conflict markers before staging it.
    CheckingConflictMarkers,
    /// Resetting a branch to a commit.
    ResettingBranch,
    /// Cherry-picking a commit.
    CherryPicking,
    /// Reloading the staged and unstaged changes.
    ReloadingChanges,
    /// Deleting an untracked file.
    DeletingFile,
    /// Discarding the unstaged changes of a file.
    DiscardingChanges,
    /// Staging or unstaging.
    Staging,
    /// Editing the commit message.
    EditingMessage,
    /// Reading the last commit for amending.
    ReadingLastCommit,
    /// Committing or amending.
    Committing,
}

/// Why typed input is not valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputProblem {
    /// The branch name breaks one of git's naming rules.
    BranchName(BranchNameProblem),
    /// The choice is not a number.
    NotANumber,
    /// The choice is not one of the listed numbers `1..=count`.
    OutOfRange {
        /// How many choices were listed.
        count: usize,
    },
    /// The search text is empty.
    EmptySearch,
    /// The commit message is empty.
    EmptyCommitMessage,
}

/// The git naming rule a branch name breaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchNameProblem {
    /// The name is empty.
    Empty,
    /// The name is `@` or `HEAD`.
    Reserved,
    /// The name starts with `-`.
    StartsWithDash,
    /// The name starts or ends with `/`, or contains `//`.
    BadSlash,
    /// A part between slashes starts with `.`.
    ComponentStartsWithDot,
    /// A part between slashes ends with `.lock`.
    ComponentEndsWithLock,
    /// The name ends with `.`.
    EndsWithDot,
    /// The name contains `..`.
    DoubleDot,
    /// The name contains `@{`.
    AtBrace,
    /// The name contains a space, a control character or one of `~ ^ : ? * [ \`.
    ForbiddenCharacter(char),
}
