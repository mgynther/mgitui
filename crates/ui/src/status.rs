//! The wording of everything in the status line: notes, problems, prompts and questions.

/// What a text prompt asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptKind {
    /// A search term.
    Search,
    /// The name of a new branch.
    BranchName,
    /// The number of the branch to check out.
    CheckoutChoice(Vec<String>),
    /// The number of the branch to delete.
    DeleteBranch(Vec<String>),
    /// The number of the branch to rebase.
    RebaseBranch {
        /// The branches to choose from.
        branches: Vec<String>,
        /// Whether the rebase is interactive.
        interactive: bool,
    },
    /// The number of the branch to rebase onto.
    RebaseOnto {
        /// The branch being rebased.
        branch: String,
        /// The branches to choose from.
        onto: Vec<String>,
        /// Whether the rebase is interactive.
        interactive: bool,
    },
}

/// A question answered with y or n.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Question {
    /// Whether to quit.
    Quit,
    /// Whether to delete a branch, also when it is not merged.
    DeleteBranch {
        /// The branch name.
        name: String,
    },
    /// Whether to reset the checked out branch to a commit.
    ResetBranch {
        /// The checked out branch.
        branch: String,
        /// The full commit hash.
        hash: String,
    },
    /// Whether to reset a branch that already exists to the commit where it was to be created.
    ResetExistingBranch {
        /// The branch name.
        branch: String,
        /// The full commit hash.
        hash: String,
    },
    /// Whether to delete an untracked file.
    DeleteUntracked {
        /// The file's path.
        path: String,
    },
    /// Whether to discard the unstaged changes of a tracked file.
    DiscardChanges {
        /// The file's path.
        path: String,
    },
    /// Whether to stage a conflicted file that still has conflict markers.
    StageWithConflictMarkers {
        /// The file's path.
        path: String,
    },
}

/// Information shown in the status line.
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
    /// The checked out branch was reset to a commit.
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
    /// The interactive rebase ended without a rebase in progress.
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

/// A problem shown in the status line in red.
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
    /// Rebasing, which may have left a rebase in progress.
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

/// The label in front of the typed text.
#[must_use]
pub fn prompt_label(kind: &PromptKind) -> String {
    match kind {
        PromptKind::Search => "/".to_string(),
        PromptKind::BranchName => "New branch name: ".to_string(),
        PromptKind::CheckoutChoice(branches) => format!("Check out ({}): ", numbered(branches)),
        PromptKind::DeleteBranch(branches) => {
            format!("Delete which branch ({}): ", numbered(branches))
        }
        PromptKind::RebaseBranch {
            branches,
            interactive,
        } => format!(
            "{} which branch ({}): ",
            rebase_verb(*interactive),
            numbered(branches)
        ),
        PromptKind::RebaseOnto {
            branch,
            onto,
            interactive,
        } => format!(
            "{} {branch} onto ({}): ",
            rebase_verb(*interactive),
            numbered(onto)
        ),
    }
}

/// The question, ending in `y/n`. Commits are shown by their short hash.
#[must_use]
pub fn question_label(question: &Question) -> String {
    match question {
        Question::Quit => "Quit? y/n".to_string(),
        Question::DeleteBranch { name } => {
            format!("Delete branch {name} even if it is not merged? y/n")
        }
        Question::ResetBranch { branch, hash } => {
            format!("Reset {branch} to {}? y/n", short(hash))
        }
        Question::ResetExistingBranch { branch, hash } => {
            format!("Branch \"{branch}\" exists. Reset to {}? y/n", short(hash))
        }
        Question::DeleteUntracked { path } => format!("Delete untracked file {path}? y/n"),
        Question::DiscardChanges { path } => format!("Discard unstaged changes in {path}? y/n"),
        Question::StageWithConflictMarkers { path } => {
            format!("{path} still has conflict markers. Stage anyway? y/n")
        }
    }
}

/// The text of an information note. Commits are shown by their short hash.
#[must_use]
pub fn info_text(info: &Info) -> String {
    match info {
        Info::NoCommitSelected => "No commit selected".to_string(),
        Info::CreatedBranch { name } => format!("Created branch {name}"),
        Info::BranchAlreadyPointsTo { branch, hash } => {
            format!("Branch {branch} already points to {}", short(hash))
        }
        Info::DeletedBranch { name } => format!("Deleted branch {name}"),
        Info::NoOtherBranchToRebaseOnto => "No other branch to rebase onto".to_string(),
        Info::Rebased { branch, onto } => format!("Rebased {branch} onto {onto}"),
        Info::RebasedInteractively { branch, onto } => {
            format!("Interactively rebased {branch} onto {onto}")
        }
        Info::CheckedOut { branch } => format!("Checked out {branch}"),
        Info::UpdatedRemotes => "Updated remotes".to_string(),
        Info::FetchedAndPruned => {
            "Fetched all remotes and pruned stale remote branches".to_string()
        }
        Info::ResetBranch { branch, hash } => format!("Reset {branch} to {}", short(hash)),
        Info::CherryPicked { hash } => format!("Cherry-picked {}", short(hash)),
        Info::NoMainOrMasterBranch => "No main or master branch".to_string(),
        Info::OnlyWholeFilesCanBeStagedHere => {
            "Only whole files can be staged here: use Enter in the file list".to_string()
        }
        Info::OnlyUnstagedChangesCanBeDiscarded => {
            "Only unstaged changes can be discarded: unstage the file first".to_string()
        }
        Info::NothingToStage => "Nothing to stage".to_string(),
        Info::EditingMessageCancelled => "Editing the message was cancelled".to_string(),
        Info::InteractiveRebaseCancelled => "The interactive rebase was cancelled".to_string(),
        Info::NoChangedLinesSelected => "No changed lines selected".to_string(),
        Info::DeletedFile { path } => format!("Deleted {path}"),
        Info::DiscardedChanges { path } => format!("Discarded changes in {path}"),
        Info::NoPreviousSearch => "No previous search".to_string(),
        Info::PatternNotFound { term } => format!("Pattern not found: {term}"),
        Info::SearchWrapped => "Search wrapped".to_string(),
    }
}

/// The text of a problem.
#[must_use]
pub fn problem_text(problem: &Problem) -> String {
    match problem {
        Problem::Failed(operation) => failure_text(*operation).to_string(),
        Problem::Invalid(input) => input_problem_text(input),
        Problem::ChoiceNotListed => "The choice is not listed".to_string(),
        Problem::NoLocalBranchToCheckOut => {
            "No local branch at this commit to check out".to_string()
        }
        Problem::NoLocalBranchToDelete => "No local branch at this commit to delete".to_string(),
        Problem::CheckedOutBranchCannotBeDeleted => {
            "The checked out branch cannot be deleted".to_string()
        }
        Problem::NoLocalBranchToRebase => "No local branch at this commit to rebase".to_string(),
        Problem::NoBranchCheckedOutToReset => "No branch is checked out to reset".to_string(),
        Problem::UncommittedChangesBeforeReset => {
            "Commit or discard uncommitted changes before resetting".to_string()
        }
        Problem::UncommittedChangesBeforeRebase => {
            "Commit or discard uncommitted changes before rebasing".to_string()
        }
        Problem::RebaseStopped => {
            "The rebase stopped: finish or abort it outside mgitui".to_string()
        }
        Problem::MergeConflict { path } => {
            format!("{path} has a merge conflict: resolve it, then stage the whole file")
        }
        Problem::MergeConflictsBeforeStagingAll => {
            "Resolve merge conflicts outside mgitui before staging all".to_string()
        }
        Problem::NothingToAmend => "Nothing to amend: there are no commits yet".to_string(),
        Problem::AmendedCommitWouldBeEmpty => {
            "The amended commit would be empty: stage changes first".to_string()
        }
    }
}

/// Each failure has its own full message, so that the wording can differ where it needs to.
fn failure_text(operation: Operation) -> &'static str {
    match operation {
        Operation::RunningGc => "Running gc failed, see the Log view",
        Operation::ReloadingCommits => "Reloading commits failed, see the Log view",
        Operation::LoadingCommit => "Loading the commit failed, see the Log view",
        Operation::LoadingDiff => "Loading the diff failed, see the Log view",
        Operation::LoadingMoreCommits => "Loading more commits failed, see the Log view",
        Operation::CreatingBranch => "Creating the branch failed, see the Log view",
        Operation::DeletingBranch => "Deleting the branch failed, see the Log view",
        Operation::Rebasing => "Rebasing failed, see the Log view; resolve it outside mgitui",
        Operation::CheckingOut => "Checking out failed, see the Log view",
        Operation::UpdatingRemotes => "Updating remotes failed, see the Log view",
        Operation::FetchingAndPruning => "Fetching and pruning remotes failed, see the Log view",
        Operation::ShowingCommit => "Showing the commit failed, see the Log view",
        Operation::ReadingStatus => "Reading the status failed, see the Log view",
        Operation::ReadingBranches => "Reading the branches failed, see the Log view",
        Operation::CheckingConflictMarkers => {
            "Checking for conflict markers failed, see the Log view"
        }
        Operation::ResettingBranch => "Resetting the branch failed, see the Log view",
        Operation::CherryPicking => "Cherry-picking failed, see the Log view",
        Operation::ReloadingChanges => "Reloading changes failed, see the Log view",
        Operation::DeletingFile => "Deleting the file failed, see the Log view",
        Operation::DiscardingChanges => "Discarding the changes failed, see the Log view",
        Operation::Staging => "Staging failed, see the Log view",
        Operation::EditingMessage => "Editing the message failed, see the Log view",
        Operation::ReadingLastCommit => "Reading the last commit failed, see the Log view",
        Operation::Committing => "Committing failed, see the Log view",
    }
}

fn input_problem_text(problem: &InputProblem) -> String {
    match problem {
        InputProblem::BranchName(rule) => branch_name_text(*rule),
        InputProblem::NotANumber => "The choice is not a number".to_string(),
        InputProblem::OutOfRange { count } => {
            format!("The choice must be a number from 1 to {count}")
        }
        InputProblem::EmptySearch => "The search text is empty".to_string(),
        InputProblem::EmptyCommitMessage => "The commit message is empty".to_string(),
    }
}

fn branch_name_text(rule: BranchNameProblem) -> String {
    match rule {
        BranchNameProblem::Empty => "The branch name is empty".to_string(),
        BranchNameProblem::Reserved => "The branch name is reserved by git".to_string(),
        BranchNameProblem::StartsWithDash => "A branch name cannot start with '-'".to_string(),
        BranchNameProblem::BadSlash => {
            "A branch name cannot start or end with '/' or contain '//'".to_string()
        }
        BranchNameProblem::ComponentStartsWithDot => {
            "No part of a branch name can start with '.'".to_string()
        }
        BranchNameProblem::ComponentEndsWithLock => {
            "No part of a branch name can end with '.lock'".to_string()
        }
        BranchNameProblem::EndsWithDot => "A branch name cannot end with '.'".to_string(),
        BranchNameProblem::DoubleDot => "A branch name cannot contain '..'".to_string(),
        BranchNameProblem::AtBrace => "A branch name cannot contain '@{'".to_string(),
        BranchNameProblem::ForbiddenCharacter(c) => {
            format!("A branch name cannot contain the character {c:?}")
        }
    }
}

/// A commit's short hash.
fn short(hash: &str) -> &str {
    hash.get(..7).unwrap_or(hash)
}

fn rebase_verb(interactive: bool) -> &'static str {
    if interactive {
        "Interactively rebase"
    } else {
        "Rebase"
    }
}

/// `1 a, 2 b, ...` for a numbered prompt.
fn numbered(items: &[String]) -> String {
    let choices: Vec<String> = items
        .iter()
        .enumerate()
        .map(|(index, item): (usize, &String)| format!("{} {item}", index + 1))
        .collect();
    choices.join(", ")
}

#[cfg(test)]
mod tests {
    use super::{
        BranchNameProblem, Info, InputProblem, Operation, Problem, PromptKind, Question, info_text,
        problem_text, prompt_label, question_label,
    };

    fn s(text: &str) -> String {
        text.to_string()
    }

    #[test]
    fn info_texts() {
        let cases: [(Info, &str); 24] = [
            (Info::NoCommitSelected, "No commit selected"),
            (Info::CreatedBranch { name: s("t") }, "Created branch t"),
            (Info::DeletedBranch { name: s("old") }, "Deleted branch old"),
            (
                Info::NoOtherBranchToRebaseOnto,
                "No other branch to rebase onto",
            ),
            (
                Info::Rebased {
                    branch: s("t"),
                    onto: s("main"),
                },
                "Rebased t onto main",
            ),
            (
                Info::RebasedInteractively {
                    branch: s("t"),
                    onto: s("main"),
                },
                "Interactively rebased t onto main",
            ),
            (Info::CheckedOut { branch: s("main") }, "Checked out main"),
            (Info::UpdatedRemotes, "Updated remotes"),
            (
                Info::FetchedAndPruned,
                "Fetched all remotes and pruned stale remote branches",
            ),
            (
                Info::ResetBranch {
                    branch: s("main"),
                    hash: s("1234567890"),
                },
                "Reset main to 1234567",
            ),
            (
                Info::BranchAlreadyPointsTo {
                    branch: s("topic"),
                    hash: s("1234567890"),
                },
                "Branch topic already points to 1234567",
            ),
            (
                Info::CherryPicked {
                    hash: s("1234567890"),
                },
                "Cherry-picked 1234567",
            ),
            (Info::NoMainOrMasterBranch, "No main or master branch"),
            (
                Info::OnlyWholeFilesCanBeStagedHere,
                "Only whole files can be staged here: use Enter in the file list",
            ),
            (
                Info::OnlyUnstagedChangesCanBeDiscarded,
                "Only unstaged changes can be discarded: unstage the file first",
            ),
            (Info::NothingToStage, "Nothing to stage"),
            (
                Info::EditingMessageCancelled,
                "Editing the message was cancelled",
            ),
            (
                Info::InteractiveRebaseCancelled,
                "The interactive rebase was cancelled",
            ),
            (Info::NoChangedLinesSelected, "No changed lines selected"),
            (Info::DeletedFile { path: s("n") }, "Deleted n"),
            (
                Info::DiscardedChanges { path: s("u") },
                "Discarded changes in u",
            ),
            (Info::NoPreviousSearch, "No previous search"),
            (
                Info::PatternNotFound { term: s("z") },
                "Pattern not found: z",
            ),
            (Info::SearchWrapped, "Search wrapped"),
        ];
        for (info, text) in cases {
            assert_eq!(info_text(&info), text);
        }
    }

    #[test]
    fn problem_texts() {
        let cases: [(Problem, &str); 14] = [
            (Problem::ChoiceNotListed, "The choice is not listed"),
            (
                Problem::NoLocalBranchToCheckOut,
                "No local branch at this commit to check out",
            ),
            (
                Problem::NoLocalBranchToDelete,
                "No local branch at this commit to delete",
            ),
            (
                Problem::CheckedOutBranchCannotBeDeleted,
                "The checked out branch cannot be deleted",
            ),
            (
                Problem::NoLocalBranchToRebase,
                "No local branch at this commit to rebase",
            ),
            (
                Problem::NoBranchCheckedOutToReset,
                "No branch is checked out to reset",
            ),
            (
                Problem::UncommittedChangesBeforeReset,
                "Commit or discard uncommitted changes before resetting",
            ),
            (
                Problem::UncommittedChangesBeforeRebase,
                "Commit or discard uncommitted changes before rebasing",
            ),
            (
                Problem::RebaseStopped,
                "The rebase stopped: finish or abort it outside mgitui",
            ),
            (
                Problem::MergeConflict { path: s("c") },
                "c has a merge conflict: resolve it, then stage the whole file",
            ),
            (
                Problem::MergeConflictsBeforeStagingAll,
                "Resolve merge conflicts outside mgitui before staging all",
            ),
            (
                Problem::NothingToAmend,
                "Nothing to amend: there are no commits yet",
            ),
            (
                Problem::AmendedCommitWouldBeEmpty,
                "The amended commit would be empty: stage changes first",
            ),
            (
                Problem::Invalid(InputProblem::EmptySearch),
                "The search text is empty",
            ),
        ];
        for (problem, text) in cases {
            assert_eq!(problem_text(&problem), text);
        }
    }

    #[test]
    fn failure_texts() {
        let cases: [(Operation, &str); 24] = [
            (Operation::RunningGc, "Running gc failed, see the Log view"),
            (
                Operation::ReloadingCommits,
                "Reloading commits failed, see the Log view",
            ),
            (
                Operation::LoadingCommit,
                "Loading the commit failed, see the Log view",
            ),
            (
                Operation::LoadingDiff,
                "Loading the diff failed, see the Log view",
            ),
            (
                Operation::LoadingMoreCommits,
                "Loading more commits failed, see the Log view",
            ),
            (
                Operation::CreatingBranch,
                "Creating the branch failed, see the Log view",
            ),
            (
                Operation::DeletingBranch,
                "Deleting the branch failed, see the Log view",
            ),
            (
                Operation::Rebasing,
                "Rebasing failed, see the Log view; resolve it outside mgitui",
            ),
            (
                Operation::CheckingOut,
                "Checking out failed, see the Log view",
            ),
            (
                Operation::UpdatingRemotes,
                "Updating remotes failed, see the Log view",
            ),
            (
                Operation::FetchingAndPruning,
                "Fetching and pruning remotes failed, see the Log view",
            ),
            (
                Operation::ShowingCommit,
                "Showing the commit failed, see the Log view",
            ),
            (
                Operation::ReadingStatus,
                "Reading the status failed, see the Log view",
            ),
            (
                Operation::ReadingBranches,
                "Reading the branches failed, see the Log view",
            ),
            (
                Operation::CheckingConflictMarkers,
                "Checking for conflict markers failed, see the Log view",
            ),
            (
                Operation::ResettingBranch,
                "Resetting the branch failed, see the Log view",
            ),
            (
                Operation::CherryPicking,
                "Cherry-picking failed, see the Log view",
            ),
            (
                Operation::ReloadingChanges,
                "Reloading changes failed, see the Log view",
            ),
            (
                Operation::DeletingFile,
                "Deleting the file failed, see the Log view",
            ),
            (
                Operation::DiscardingChanges,
                "Discarding the changes failed, see the Log view",
            ),
            (Operation::Staging, "Staging failed, see the Log view"),
            (
                Operation::EditingMessage,
                "Editing the message failed, see the Log view",
            ),
            (
                Operation::ReadingLastCommit,
                "Reading the last commit failed, see the Log view",
            ),
            (Operation::Committing, "Committing failed, see the Log view"),
        ];
        for (operation, text) in cases {
            assert_eq!(problem_text(&Problem::Failed(operation)), text);
        }
    }

    #[test]
    fn input_problem_texts() {
        let cases: [(InputProblem, &str); 14] = [
            (InputProblem::NotANumber, "The choice is not a number"),
            (
                InputProblem::OutOfRange { count: 3 },
                "The choice must be a number from 1 to 3",
            ),
            (InputProblem::EmptySearch, "The search text is empty"),
            (
                InputProblem::EmptyCommitMessage,
                "The commit message is empty",
            ),
            (
                InputProblem::BranchName(BranchNameProblem::Empty),
                "The branch name is empty",
            ),
            (
                InputProblem::BranchName(BranchNameProblem::Reserved),
                "The branch name is reserved by git",
            ),
            (
                InputProblem::BranchName(BranchNameProblem::StartsWithDash),
                "A branch name cannot start with '-'",
            ),
            (
                InputProblem::BranchName(BranchNameProblem::BadSlash),
                "A branch name cannot start or end with '/' or contain '//'",
            ),
            (
                InputProblem::BranchName(BranchNameProblem::ComponentStartsWithDot),
                "No part of a branch name can start with '.'",
            ),
            (
                InputProblem::BranchName(BranchNameProblem::ComponentEndsWithLock),
                "No part of a branch name can end with '.lock'",
            ),
            (
                InputProblem::BranchName(BranchNameProblem::EndsWithDot),
                "A branch name cannot end with '.'",
            ),
            (
                InputProblem::BranchName(BranchNameProblem::DoubleDot),
                "A branch name cannot contain '..'",
            ),
            (
                InputProblem::BranchName(BranchNameProblem::AtBrace),
                "A branch name cannot contain '@{'",
            ),
            (
                InputProblem::BranchName(BranchNameProblem::ForbiddenCharacter(' ')),
                "A branch name cannot contain the character ' '",
            ),
        ];
        for (problem, text) in cases {
            assert_eq!(problem_text(&Problem::Invalid(problem)), text);
        }
    }

    #[test]
    fn prompt_labels() {
        let branches: Vec<String> = vec!["a".to_string(), "b".to_string()];
        let cases: [(PromptKind, &str); 7] = [
            (PromptKind::Search, "/"),
            (PromptKind::BranchName, "New branch name: "),
            (
                PromptKind::CheckoutChoice(branches.clone()),
                "Check out (1 a, 2 b): ",
            ),
            (
                PromptKind::DeleteBranch(branches.clone()),
                "Delete which branch (1 a, 2 b): ",
            ),
            (
                PromptKind::RebaseBranch {
                    branches: branches.clone(),
                    interactive: false,
                },
                "Rebase which branch (1 a, 2 b): ",
            ),
            (
                PromptKind::RebaseBranch {
                    branches: branches.clone(),
                    interactive: true,
                },
                "Interactively rebase which branch (1 a, 2 b): ",
            ),
            (
                PromptKind::RebaseOnto {
                    branch: "t".to_string(),
                    onto: branches,
                    interactive: false,
                },
                "Rebase t onto (1 a, 2 b): ",
            ),
        ];
        for (kind, label) in cases {
            assert_eq!(prompt_label(&kind), label);
        }
    }

    #[test]
    fn question_labels() {
        let cases: [(Question, &str); 8] = [
            (Question::Quit, "Quit? y/n"),
            (
                Question::DeleteBranch {
                    name: "old".to_string(),
                },
                "Delete branch old even if it is not merged? y/n",
            ),
            (
                Question::ResetBranch {
                    branch: "main".to_string(),
                    hash: "1234567890".to_string(),
                },
                "Reset main to 1234567? y/n",
            ),
            (
                Question::ResetBranch {
                    branch: "main".to_string(),
                    hash: "123".to_string(),
                },
                "Reset main to 123? y/n",
            ),
            (
                Question::ResetExistingBranch {
                    branch: "topic".to_string(),
                    hash: "1234567890".to_string(),
                },
                "Branch \"topic\" exists. Reset to 1234567? y/n",
            ),
            (
                Question::DeleteUntracked {
                    path: "new.txt".to_string(),
                },
                "Delete untracked file new.txt? y/n",
            ),
            (
                Question::DiscardChanges {
                    path: "a.txt".to_string(),
                },
                "Discard unstaged changes in a.txt? y/n",
            ),
            (
                Question::StageWithConflictMarkers {
                    path: "c.txt".to_string(),
                },
                "c.txt still has conflict markers. Stage anyway? y/n",
            ),
        ];
        for (question, label) in cases {
            assert_eq!(question_label(&question), label);
        }
    }
}
