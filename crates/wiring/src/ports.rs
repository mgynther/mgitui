//! The functions wiring provides to the logic layer.

use logic::log::LogLevel;
use logic::model::{AmendBase, ChangedFile, Commit, FileDiff, Refs, Side, WorkStatus};
use logic::note::InputProblem;
use logic::ports::{BranchName, CommitMessage, Failed};

use crate::editor::{self, Editor, Handoff};
use crate::log::LogBuffer;
use crate::model;

/// Implements the logic layer's ports with the git and validation layers.
pub struct SessionPorts<'a> {
    /// Collects log messages from every layer.
    pub log: &'a LogBuffer,
    /// Runs git in the repository.
    pub git: &'a git::Git,
    /// Runs Vim on a file.
    pub editor: &'a Editor,
    /// Hands the terminal to git when git runs Vim itself.
    pub handoff: &'a Handoff<'a>,
}

impl SessionPorts<'_> {
    fn log_validation(&self) -> impl Fn(validation::log::LogLevel, &str) + '_ {
        |level: validation::log::LogLevel, message: &str| self.log.validation(level, message)
    }

    /// Runs `run` with the terminal handed over, for git commands that may use the terminal.
    fn with_terminal(&self, run: &mut dyn FnMut() -> Result<(), Failed>) -> Result<(), Failed> {
        let mut result: Result<(), Failed> = Err(Failed);
        let handed: std::io::Result<()> = (self.handoff)(&mut || result = run());
        if let Err(error) = handed {
            self.log.wiring(
                LogLevel::Error,
                &format!("handing the terminal to git failed: {error}"),
            );
            return Err(Failed);
        }
        result
    }

    fn log_git(&self) -> impl Fn(git::log::LogLevel, &str) + '_ {
        |level: git::log::LogLevel, message: &str| self.log.git(level, message)
    }
}

impl logic::ports::Ports for SessionPorts<'_> {
    fn log(&self, level: logic::log::LogLevel, message: &str) {
        self.log.logic(level, message);
    }

    fn validate_search(&self, input: &str) -> Result<logic::ports::SearchText, InputProblem> {
        validation::search_text(input, &self.log_validation())
            .map(|text: validation::SearchText| logic::ports::SearchText(text.as_str().to_string()))
            .map_err(|error: validation::ValidationError| model::input_problem(&error))
    }

    fn status(&self) -> Result<WorkStatus, Failed> {
        self.git
            .status(&self.log_git())
            .map(model::work_status)
            .map_err(|_| Failed)
    }

    fn file_diff(&self, paths: &[String], side: Side) -> Result<FileDiff, Failed> {
        self.git
            .diff(paths, side == Side::Staged, &self.log_git())
            .map(model::file_diff)
            .map_err(|_| Failed)
    }

    fn untracked_file_diff(&self, path: &str) -> Result<FileDiff, Failed> {
        self.git
            .diff_untracked(path, &self.log_git())
            .map(model::file_diff)
            .map_err(|_| Failed)
    }

    fn stage_paths(&self, paths: &[String]) -> Result<(), Failed> {
        self.git.add(paths, &self.log_git()).map_err(|_| Failed)
    }

    fn stage_all(&self) -> Result<(), Failed> {
        self.git.add_all(&self.log_git()).map_err(|_| Failed)
    }

    fn unstage_paths(&self, paths: &[String]) -> Result<(), Failed> {
        self.git
            .restore_staged(paths, &self.log_git())
            .map_err(|_| Failed)
    }

    fn remove_from_index(&self, paths: &[String]) -> Result<(), Failed> {
        self.git
            .remove_cached(paths, &self.log_git())
            .map_err(|_| Failed)
    }

    fn restore_paths(&self, paths: &[String]) -> Result<(), Failed> {
        self.git.restore(paths, &self.log_git()).map_err(|_| Failed)
    }

    fn delete_untracked(&self, paths: &[String]) -> Result<(), Failed> {
        self.git.clean(paths, &self.log_git()).map_err(|_| Failed)
    }

    fn apply_to_index(&self, patch: &str, reverse: bool) -> Result<(), Failed> {
        self.git
            .apply_cached(patch, reverse, &self.log_git())
            .map_err(|_| Failed)
    }

    fn has_commits(&self) -> Result<bool, Failed> {
        self.git.has_head(&self.log_git()).map_err(|_| Failed)
    }

    fn last_commit_has_parent(&self) -> Result<bool, Failed> {
        self.git
            .head_has_parent(&self.log_git())
            .map_err(|_| Failed)
    }

    fn amended_files(&self, base: AmendBase) -> Result<Vec<ChangedFile>, Failed> {
        self.git
            .staged_since(model::amend_base(base), &self.log_git())
            .map(model::changed_files)
            .map_err(|_| Failed)
    }

    fn amended_file_diff(&self, paths: &[String], base: AmendBase) -> Result<FileDiff, Failed> {
        self.git
            .diff_since(paths, model::amend_base(base), &self.log_git())
            .map(model::file_diff)
            .map_err(|_| Failed)
    }

    fn unstage_from_amend(&self, paths: &[String]) -> Result<(), Failed> {
        self.git
            .reset_to_parent(paths, &self.log_git())
            .map_err(|_| Failed)
    }

    fn last_commit_message(&self) -> Result<String, Failed> {
        self.git
            .last_commit_message(&self.log_git())
            .map(|message: String| format!("{}\n", message.trim_end()))
            .map_err(|_| Failed)
    }

    fn edit_message(&self, text: &str) -> Result<String, Failed> {
        editor::edit_text(text, self.git.temp_dir(), self.editor).map_err(
            |error: std::io::Error| {
                self.log.wiring(
                    LogLevel::Error,
                    &format!("editing the commit message failed: {error}"),
                );
                Failed
            },
        )
    }

    fn validate_commit_message(&self, text: &str) -> Result<CommitMessage, InputProblem> {
        validation::commit_message(text, &self.log_validation())
            .map(|message: validation::CommitMessage| CommitMessage(message.as_str().to_string()))
            .map_err(|error: validation::ValidationError| model::input_problem(&error))
    }

    fn commit(&self, message: &CommitMessage, amend: bool) -> Result<(), Failed> {
        self.git
            .commit(&message.0, amend, &self.log_git())
            .map_err(|_| Failed)
    }

    fn commits(&self, skip: usize, count: usize) -> Result<Vec<Commit>, Failed> {
        self.git
            .log(skip, count, &self.log_git())
            .map(|commits: Vec<git::Commit>| commits.into_iter().map(model::commit).collect())
            .map_err(|_| Failed)
    }

    fn refs(&self) -> Result<Refs, Failed> {
        self.git
            .refs(&self.log_git())
            .and_then(|refs: Vec<git::Ref>| {
                self.git
                    .head(&self.log_git())
                    .map(|head: Option<String>| model::refs(refs, head))
            })
            .map_err(|_| Failed)
    }

    fn validate_branch_name(&self, input: &str) -> Result<BranchName, InputProblem> {
        validation::branch_name(input, &self.log_validation())
            .map(|name: validation::BranchName| BranchName(name.as_str().to_string()))
            .map_err(|error: validation::ValidationError| model::input_problem(&error))
    }

    fn validate_checkout_choice(&self, input: &str, count: usize) -> Result<usize, InputProblem> {
        validation::checkout_choice(input, count, &self.log_validation())
            .map(validation::CheckoutChoice::index)
            .map_err(|error: validation::ValidationError| model::input_problem(&error))
    }

    fn create_branch(&self, name: &BranchName, at: &str) -> Result<(), Failed> {
        self.git
            .create_branch(&name.0, at, &self.log_git())
            .map_err(|_| Failed)
    }

    fn delete_branch(&self, name: &str) -> Result<(), Failed> {
        self.git
            .delete_branch(name, &self.log_git())
            .map_err(|_| Failed)
    }

    fn remote_update(&self) -> Result<(), Failed> {
        self.with_terminal(&mut || self.git.remote_update(&self.log_git()).map_err(|_| Failed))
    }

    fn fetch_all_prune(&self) -> Result<(), Failed> {
        self.with_terminal(&mut || {
            self.git
                .fetch_all_prune(&self.log_git())
                .map_err(|_| Failed)
        })
    }

    fn show_commit(&self, hash: &str) -> Result<(), Failed> {
        self.with_terminal(&mut || {
            self.git
                .show_commit(hash, &self.log_git())
                .map_err(|_| Failed)
        })
    }

    fn reset_keep(&self, hash: &str) -> Result<(), Failed> {
        self.git
            .reset_keep(hash, &self.log_git())
            .map_err(|_| Failed)
    }

    fn cherry_pick(&self, hash: &str) -> Result<(), Failed> {
        self.git
            .cherry_pick(hash, &self.log_git())
            .map_err(|_| Failed)
    }

    fn rebase(&self, branch: &str, onto: &str, interactive: bool) -> Result<(), Failed> {
        if !interactive {
            return self
                .git
                .rebase(branch, onto, &self.log_git())
                .map_err(|_| Failed);
        }
        self.with_terminal(&mut || {
            self.git
                .rebase_interactive(branch, onto, &self.log_git())
                .map_err(|_| Failed)
        })
    }

    fn checkout(&self, branch: &str) -> Result<(), Failed> {
        self.git.switch(branch, &self.log_git()).map_err(|_| Failed)
    }

    fn commit_message(&self, hash: &str) -> Result<String, Failed> {
        self.git
            .commit_message(hash, &self.log_git())
            .map_err(|_| Failed)
    }

    fn commit_files(&self, hash: &str) -> Result<Vec<ChangedFile>, Failed> {
        self.git
            .commit_files(hash, &self.log_git())
            .map(model::changed_files)
            .map_err(|_| Failed)
    }

    fn commit_file_diff(&self, hash: &str, paths: &[String]) -> Result<FileDiff, Failed> {
        self.git
            .commit_file_diff(hash, paths, &self.log_git())
            .map(model::file_diff)
            .map_err(|_| Failed)
    }
}

#[cfg(test)]
mod tests;
