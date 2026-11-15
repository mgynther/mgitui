//! The functions logic uses to get things done, provided by wiring.

use crate::log::LogLevel;
use crate::model::{ChangedFile, Commit, FileDiff, Refs, Side, WorkStatus};

/// Search text that passed validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchText(pub String);

/// A branch name that passed validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchName(pub String);

/// A commit message that passed validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitMessage(pub String);

/// A failure that was already logged by the layer where it happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Failed;

/// Everything outside the application state that logic needs. Failures are logged by the
/// layer where they happen; the error values only carry what the status line needs.
pub trait Ports {
    /// Logs a message from the logic layer.
    fn log(&self, level: LogLevel, message: &str);

    /// Validates search text typed by the user.
    ///
    /// # Errors
    ///
    /// Returns a short description when the text is not valid.
    fn validate_search(&self, input: &str) -> Result<SearchText, String>;

    /// The staged and unstaged changes.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    fn status(&self) -> Result<WorkStatus, Failed>;

    /// The diff of a changed file in the staged or unstaged list.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    fn file_diff(&self, file: &ChangedFile, side: Side) -> Result<FileDiff, Failed>;

    /// Stages all changes of a file in the unstaged list.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    fn stage_file(&self, file: &ChangedFile) -> Result<(), Failed>;

    /// Unstages all changes of a file in the staged list.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    fn unstage_file(&self, file: &ChangedFile) -> Result<(), Failed>;

    /// Discards the unstaged changes of a file in the unstaged list: an untracked file is
    /// deleted, a tracked file is restored from the index.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    fn discard_file(&self, file: &ChangedFile) -> Result<(), Failed>;

    /// Applies a patch to the index, in reverse when `reverse` is true.
    ///
    /// # Errors
    ///
    /// Fails when git fails, for example when the patch does not apply.
    fn apply_to_index(&self, patch: &str, reverse: bool) -> Result<(), Failed>;

    /// Whether the repository has at least one commit.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    fn has_commits(&self) -> Result<bool, Failed>;

    /// The message of the last commit.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    fn last_commit_message(&self) -> Result<String, Failed>;

    /// Lets the user edit `text` in Vim and returns the edited text.
    ///
    /// # Errors
    ///
    /// Fails when the editor cannot be run.
    fn edit_message(&self, text: &str) -> Result<String, Failed>;

    /// Validates a commit message.
    ///
    /// # Errors
    ///
    /// Returns a short description when the message is not valid.
    fn validate_commit_message(&self, text: &str) -> Result<CommitMessage, String>;

    /// Commits the staged changes, or amends the last commit.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    fn commit(&self, message: &CommitMessage, amend: bool) -> Result<(), Failed>;

    /// Up to `count` commits of all refs in topological order, after skipping `skip`.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    fn commits(&self, skip: usize, count: usize) -> Result<Vec<Commit>, Failed>;

    /// All branch and tag labels and the commit `HEAD` points to.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    fn refs(&self) -> Result<Refs, Failed>;

    /// The full message of a commit.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    fn commit_message(&self, hash: &str) -> Result<String, Failed>;

    /// The files a commit changed, compared with its first parent.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    fn commit_files(&self, hash: &str) -> Result<Vec<ChangedFile>, Failed>;

    /// The diff of one file of a commit, compared with its first parent.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    fn commit_file_diff(&self, hash: &str, file: &ChangedFile) -> Result<FileDiff, Failed>;

    /// Validates a branch name typed by the user.
    ///
    /// # Errors
    ///
    /// Returns a short description when the name is not valid.
    fn validate_branch_name(&self, input: &str) -> Result<BranchName, String>;

    /// Validates a number typed to choose one of `count` listed items. Returns the zero-based
    /// index of the item.
    ///
    /// # Errors
    ///
    /// Returns a short description when the input is not one of the listed numbers.
    fn validate_checkout_choice(&self, input: &str, count: usize) -> Result<usize, String>;

    /// Creates a branch at a commit without checking it out.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    fn create_branch(&self, name: &BranchName, at: &str) -> Result<(), Failed>;

    /// Deletes a local branch that is not checked out.
    ///
    /// # Errors
    ///
    /// Fails when git fails, for example when the branch is not merged.
    fn delete_branch(&self, name: &str) -> Result<(), Failed>;

    /// Fetches all remotes with `git remote update`.
    ///
    /// # Errors
    ///
    /// Fails when git fails, for example when a remote cannot be reached.
    fn remote_update(&self) -> Result<(), Failed>;

    /// Fetches all remotes with `git fetch --all --prune`.
    ///
    /// # Errors
    ///
    /// Fails when git fails, for example when a remote cannot be reached.
    fn fetch_all_prune(&self) -> Result<(), Failed>;

    /// Shows a commit with `git show` on the terminal, in the user's pager.
    ///
    /// # Errors
    ///
    /// Fails when git fails or the terminal cannot be handed over.
    fn show_commit(&self, hash: &str) -> Result<(), Failed>;

    /// Resets the checked out branch to a commit with `git reset --keep`, which updates the
    /// working tree but refuses to overwrite local changes and untracked files.
    ///
    /// # Errors
    ///
    /// Fails when git fails, for example when an untracked file is in the way.
    fn reset_keep(&self, hash: &str) -> Result<(), Failed>;

    /// Cherry-picks a commit onto the checked out branch.
    ///
    /// # Errors
    ///
    /// Fails when git fails, for example on a conflict.
    fn cherry_pick(&self, hash: &str) -> Result<(), Failed>;

    /// Checks out `branch` and rebases it onto `onto`. An interactive rebase lets the user edit
    /// the todo list in Vim.
    ///
    /// # Errors
    ///
    /// Fails when git fails, for example on a conflict.
    fn rebase(&self, branch: &str, onto: &str, interactive: bool) -> Result<(), Failed>;

    /// Checks out a local branch.
    ///
    /// # Errors
    ///
    /// Fails when git fails, for example with local changes in the way.
    fn checkout(&self, branch: &str) -> Result<(), Failed>;
}
