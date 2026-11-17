//! Committing.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::command::Command;
use crate::log::Log;
use crate::runner::{Git, GitError, Output};

static NEXT_FILE: AtomicUsize = AtomicUsize::new(0);

/// A file in the system temp directory, removed when dropped.
struct TempFile(PathBuf);

impl TempFile {
    fn create(dir: &Path, content: &str) -> std::io::Result<TempFile> {
        let id: usize = NEXT_FILE.fetch_add(1, Ordering::Relaxed);
        let path: PathBuf = dir.join(format!("mgitui-commit-{}-{id}.txt", std::process::id()));
        std::fs::write(&path, content)?;
        Ok(TempFile(path))
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

impl Git {
    /// The message of the last commit.
    ///
    /// # Errors
    ///
    /// Fails when git fails, for example when there are no commits.
    pub fn last_commit_message(&self, log: &Log<'_>) -> Result<String, GitError> {
        let output: Output = self.run(&Command::LastMessage, None, log)?;
        Ok(output.stdout)
    }

    /// Commits the staged changes with `message`, or amends the last commit. The message is
    /// passed in a temporary file so that git never opens an editor.
    ///
    /// # Errors
    ///
    /// Fails when the temporary file cannot be written or git fails, for example when nothing
    /// is staged.
    pub fn commit(&self, message: &str, amend: bool, log: &Log<'_>) -> Result<(), GitError> {
        let file: TempFile = TempFile::create(self.temp_dir(), message)
            .map_err(|error: std::io::Error| GitError::Spawn(error.to_string()))?;
        let command: Command = Command::Commit {
            message_file: file.0.to_string_lossy().into_owned(),
            amend,
        };
        self.run(&command, None, log)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::runner::{Git, GitError};
    use crate::test_repo::{Logged, TempDir, TestRepo};

    #[test]
    fn commit_and_amend() {
        let repo: TestRepo = TestRepo::new();
        repo.write("a.txt", "a\n");
        repo.git_ok(&["add", "."]);
        // A temp directory of its own: tests running in parallel commit with message files in
        // the system temp directory, which would count as leftovers here.
        let temp: TempDir = TempDir::new();
        let git: Git = repo.git().with_temp_dir(temp.path());
        let logged: Logged = Logged::new();
        git.commit("First\n\nBody\n", false, &logged.log()).unwrap();
        assert_eq!(
            git.last_commit_message(&logged.log()).unwrap(),
            "First\n\nBody\n\n"
        );
        git.commit("Changed\n", true, &logged.log()).unwrap();
        assert_eq!(repo.git_ok(&["rev-list", "--count", "HEAD"]), "1\n");
        assert_eq!(repo.git_ok(&["log", "-1", "--format=%s"]), "Changed\n");
        let leftovers: usize = std::fs::read_dir(temp.path()).unwrap().count();
        assert_eq!(leftovers, 0);
    }

    #[test]
    fn failures() {
        let repo: TestRepo = TestRepo::new();
        let logged: Logged = Logged::new();
        assert!(matches!(
            repo.git().last_commit_message(&logged.log()),
            Err(GitError::Failed { .. })
        ));
        assert!(matches!(
            repo.git().commit("Nothing staged\n", false, &logged.log()),
            Err(GitError::Failed { .. })
        ));
        let missing: std::path::PathBuf = repo.path().join("missing");
        assert!(matches!(
            repo.git()
                .with_temp_dir(&missing)
                .commit("x\n", false, &logged.log()),
            Err(GitError::Spawn(_))
        ));
    }
}
