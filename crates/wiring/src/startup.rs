//! Checks done before the user interface starts.

use std::path::PathBuf;

use ui::exit::ExitProblem;

use crate::log::LogBuffer;

/// Finds the repository that contains the directory of `git` and returns a git runner for its
/// root directory.
///
/// # Errors
///
/// Returns [`ExitProblem::NotInRepository`] when the directory is not inside a git repository
/// or git cannot be run.
pub fn open_repository(git: &git::Git, log: &LogBuffer) -> Result<git::Git, ExitProblem> {
    let log_git = |level: git::log::LogLevel, message: &str| log.git(level, message);
    git.toplevel(&log_git)
        .map(|root: PathBuf| git.in_dir(&root))
        .map_err(|error: git::GitError| ExitProblem::NotInRepository {
            dir: git.dir().display().to_string(),
            detail: error.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{ExitProblem, open_repository};
    use crate::log::LogBuffer;
    use crate::test_support::{TempDir, fixed_clock, init_repo, test_git};

    #[test]
    fn opens_repository_from_subdirectory_at_its_root() {
        let dir: TempDir = TempDir::new();
        init_repo(dir.path());
        let sub: PathBuf = dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        let log: LogBuffer = LogBuffer::new(fixed_clock);
        let git: git::Git = open_repository(&test_git(&sub), &log).unwrap();
        assert_eq!(git.dir(), dir.path().canonicalize().unwrap());
        assert_eq!(log.drain().len(), 1);
    }

    #[test]
    fn outside_repository_is_an_error() {
        let dir: TempDir = TempDir::new();
        let log: LogBuffer = LogBuffer::new(fixed_clock);
        let git: git::Git = test_git(dir.path()).with_env("GIT_CEILING_DIRECTORIES", "/");
        let problem: ExitProblem = open_repository(&git, &log).unwrap_err();
        let text: String = ui::exit::exit_text(&problem);
        assert!(
            text.starts_with(&format!(
                "mgitui: {} is not inside a git repository (git failed",
                dir.path().display()
            )),
            "{text}"
        );
    }
}
