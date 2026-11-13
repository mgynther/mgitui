//! The functions wiring provides to the logic layer.

use logic::log::LogLevel;
use logic::model::{ChangeKind, ChangedFile, Commit, FileDiff, Refs, Side, WorkStatus};
use logic::ports::{CommitMessage, Failed};

use crate::editor::{self, Editor};
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
}

impl SessionPorts<'_> {
    fn log_validation(&self) -> impl Fn(validation::log::LogLevel, &str) + '_ {
        |level: validation::log::LogLevel, message: &str| self.log.validation(level, message)
    }

    fn log_git(&self) -> impl Fn(git::log::LogLevel, &str) + '_ {
        |level: git::log::LogLevel, message: &str| self.log.git(level, message)
    }
}

/// The paths git needs for the diff of `file`: renames in the index need both paths.
fn diff_paths(file: &ChangedFile, side: Side) -> Vec<String> {
    match (&file.original_path, side) {
        (Some(original), Side::Staged) => vec![original.clone(), file.path.clone()],
        _ => vec![file.path.clone()],
    }
}

impl logic::ports::Ports for SessionPorts<'_> {
    fn log(&self, level: logic::log::LogLevel, message: &str) {
        self.log.logic(level, message);
    }

    fn validate_search(&self, input: &str) -> Result<logic::ports::SearchText, String> {
        validation::search_text(input, &self.log_validation())
            .map(|text: validation::SearchText| logic::ports::SearchText(text.as_str().to_string()))
            .map_err(|error: validation::ValidationError| error.to_string())
    }

    fn status(&self) -> Result<WorkStatus, Failed> {
        self.git
            .status(&self.log_git())
            .map(model::work_status)
            .map_err(|_| Failed)
    }

    fn file_diff(&self, file: &ChangedFile, side: Side) -> Result<FileDiff, Failed> {
        let diff: Result<git::FileDiff, git::GitError> = if file.kind == ChangeKind::Untracked {
            self.git.diff_untracked(&file.path, &self.log_git())
        } else {
            self.git.diff(
                &diff_paths(file, side),
                side == Side::Staged,
                &self.log_git(),
            )
        };
        diff.map(model::file_diff).map_err(|_| Failed)
    }

    fn stage_file(&self, file: &ChangedFile) -> Result<(), Failed> {
        self.git
            .add(&diff_paths(file, Side::Unstaged), &self.log_git())
            .map_err(|_| Failed)
    }

    fn unstage_file(&self, file: &ChangedFile) -> Result<(), Failed> {
        self.git
            .unstage(&diff_paths(file, Side::Staged), &self.log_git())
            .map_err(|_| Failed)
    }

    fn apply_to_index(&self, patch: &str, reverse: bool) -> Result<(), Failed> {
        self.git
            .apply_cached(patch, reverse, &self.log_git())
            .map_err(|_| Failed)
    }

    fn has_commits(&self) -> Result<bool, Failed> {
        self.git.has_head(&self.log_git()).map_err(|_| Failed)
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

    fn validate_commit_message(&self, text: &str) -> Result<CommitMessage, String> {
        validation::commit_message(text, &self.log_validation())
            .map(|message: validation::CommitMessage| CommitMessage(message.as_str().to_string()))
            .map_err(|error: validation::ValidationError| error.to_string())
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
}

#[cfg(test)]
mod tests;
