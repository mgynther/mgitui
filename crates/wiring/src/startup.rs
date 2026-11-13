//! Checks done before the user interface starts.

use std::path::Path;

use crate::log::LogBuffer;

/// Finds the repository that contains `dir` and returns a git runner for its root directory.
///
/// # Errors
///
/// Returns a message for the user when `dir` is not inside a git repository or git cannot be
/// run.
pub fn open_repository(git: &git::Git, log: &LogBuffer) -> Result<git::Git, String> {
    let log_git = |level: git::log::LogLevel, message: &str| log.git(level, message);
    match git.toplevel(&log_git) {
        Ok(root) => Ok(git.in_dir(&root)),
        Err(error) => Err(format!(
            "mgitui: {} is not inside a git repository ({error})",
            display(git.dir())
        )),
    }
}

fn display(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::open_repository;
    use crate::log::LogBuffer;
    use crate::test_support::{TempDir, fixed_clock, init_repo};

    #[test]
    fn opens_repository_from_subdirectory_at_its_root() {
        let dir: TempDir = TempDir::new();
        init_repo(dir.path());
        let sub: PathBuf = dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        let log: LogBuffer = LogBuffer::new(fixed_clock);
        let git: git::Git = open_repository(&git::Git::new(&sub), &log).unwrap();
        assert_eq!(git.dir(), dir.path().canonicalize().unwrap());
        assert_eq!(log.drain().len(), 1);
    }

    #[test]
    fn outside_repository_is_an_error() {
        let dir: TempDir = TempDir::new();
        let log: LogBuffer = LogBuffer::new(fixed_clock);
        let git: git::Git = git::Git::new(dir.path()).with_env("GIT_CEILING_DIRECTORIES", "/");
        let message: String = open_repository(&git, &log).unwrap_err();
        assert!(message.starts_with(&format!(
            "mgitui: {} is not inside a git repository (git failed",
            dir.path().display()
        )));
    }
}
