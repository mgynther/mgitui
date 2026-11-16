//! The functions logic uses to get things done, provided by wiring.

use crate::log::LogLevel;
use crate::model::{AmendBase, ChangedFile, Commit, FileDiff, Refs, Side, WorkStatus};
use crate::note::InputProblem;

/// Search text that passed validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchText(pub String);

/// A branch name that passed validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchName(pub String);

/// A commit message that passed validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitMessage(pub String);

/// The message after the editor exited.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditedMessage {
    /// The edited text, also when the editor exited with an error.
    pub text: String,
    /// Whether the editor exited successfully. Vim exits with an error on `:cq`.
    pub editor_succeeded: bool,
}

/// How a rebase ended when git ran it to its end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebaseEnd {
    /// The branch was rebased.
    Finished,
    /// The rebase did not finish: it stopped with the rebase in progress, it did not start,
    /// or the user cancelled an interactive rebase.
    NotFinished,
}

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
    /// Returns the problem when the text is not valid.
    fn validate_search(&self, input: &str) -> Result<SearchText, InputProblem>;

    /// The staged and unstaged changes.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    fn status(&self) -> Result<WorkStatus, Failed>;

    /// The diff of tracked `paths` in the staged or unstaged list.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    fn file_diff(&self, paths: &[String], side: Side) -> Result<FileDiff, Failed>;

    /// The diff of an untracked file, showing all of it as added.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    fn untracked_file_diff(&self, path: &str) -> Result<FileDiff, Failed>;

    /// Stages all changes of `paths`, including deletions.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    fn stage_paths(&self, paths: &[String]) -> Result<(), Failed>;

    /// Stages all changes in the working tree, including deletions and untracked files.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    fn stage_all(&self) -> Result<(), Failed>;

    /// Unstages all changes of `paths`, restoring them in the index from the last commit.
    ///
    /// # Errors
    ///
    /// Fails when git fails, for example when there are no commits.
    fn unstage_paths(&self, paths: &[String]) -> Result<(), Failed>;

    /// Removes `paths` from the index, keeping the files: unstaging when there are no commits.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    fn remove_from_index(&self, paths: &[String]) -> Result<(), Failed>;

    /// Discards the unstaged changes of tracked `paths`, restoring them from the index.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    fn restore_paths(&self, paths: &[String]) -> Result<(), Failed>;

    /// Deletes untracked `paths`.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    fn delete_untracked(&self, paths: &[String]) -> Result<(), Failed>;

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

    /// Whether the last commit has a parent, that is, whether it is not the first commit.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    fn last_commit_has_parent(&self) -> Result<bool, Failed>;

    /// The files the amended commit will change: the index compared with `base`.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    fn amended_files(&self, base: AmendBase) -> Result<Vec<ChangedFile>, Failed>;

    /// The diff of `paths` in the amended commit: the index compared with `base`.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    fn amended_file_diff(&self, paths: &[String], base: AmendBase) -> Result<FileDiff, Failed>;

    /// Takes all changes of `paths` out of the amended commit, setting them in the index to
    /// the commit before the last one.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    fn unstage_from_amend(&self, paths: &[String]) -> Result<(), Failed>;

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
    /// Fails when the editor cannot be run or is ended by a signal.
    fn edit_message(&self, text: &str) -> Result<EditedMessage, Failed>;

    /// Validates a commit message.
    ///
    /// # Errors
    ///
    /// Returns the problem when the message is not valid.
    fn validate_commit_message(&self, text: &str) -> Result<CommitMessage, InputProblem>;

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

    /// The diff of one file of a commit, given by its `paths`, compared with its first parent.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    fn commit_file_diff(&self, hash: &str, paths: &[String]) -> Result<FileDiff, Failed>;

    /// Validates a branch name typed by the user.
    ///
    /// # Errors
    ///
    /// Returns the problem when the name is not valid.
    fn validate_branch_name(&self, input: &str) -> Result<BranchName, InputProblem>;

    /// Validates a number typed to choose one of `count` listed items. Returns the zero-based
    /// index of the item.
    ///
    /// # Errors
    ///
    /// Returns the problem when the input is not one of the listed numbers.
    fn validate_checkout_choice(&self, input: &str, count: usize) -> Result<usize, InputProblem>;

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

    /// Moves local branch `name`, which is not checked out, to commit `hash`. The working tree
    /// is not touched.
    ///
    /// # Errors
    ///
    /// Fails when git fails, for example when the branch is checked out in another worktree.
    fn move_branch(&self, name: &str, hash: &str) -> Result<(), Failed>;

    /// Cherry-picks a commit onto the checked out branch.
    ///
    /// # Errors
    ///
    /// Fails when git fails, for example on a conflict.
    fn cherry_pick(&self, hash: &str) -> Result<(), Failed>;

    /// Checks out `branch` and rebases it onto `onto`.
    ///
    /// # Errors
    ///
    /// Fails when git cannot be run or fails in an unexpected way.
    fn rebase(&self, branch: &str, onto: &str) -> Result<RebaseEnd, Failed>;

    /// Checks out `branch` and rebases it onto `onto` interactively, with the user editing the
    /// todo list in Vim on the terminal.
    ///
    /// # Errors
    ///
    /// Fails when git cannot be run or fails in an unexpected way, or the terminal cannot be
    /// handed over.
    fn rebase_interactive(&self, branch: &str, onto: &str) -> Result<RebaseEnd, Failed>;

    /// Whether a rebase is in progress, stopped for the user to continue or abort it.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    fn rebase_in_progress(&self) -> Result<bool, Failed>;

    /// Checks out a local branch.
    ///
    /// # Errors
    ///
    /// Fails when git fails, for example with local changes in the way.
    fn checkout(&self, branch: &str) -> Result<(), Failed>;
}
