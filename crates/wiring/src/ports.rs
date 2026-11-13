//! The functions wiring provides to the logic layer.

use logic::model::{ChangeKind, ChangedFile, FileDiff, Side, WorkStatus};
use logic::ports::Failed;

use crate::log::LogBuffer;
use crate::model;

/// Implements the logic layer's ports with the git and validation layers.
pub struct SessionPorts<'a> {
    /// Collects log messages from every layer.
    pub log: &'a LogBuffer,
    /// Runs git in the repository.
    pub git: &'a git::Git,
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
}

#[cfg(test)]
mod tests;
