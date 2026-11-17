//! Keeping the repository in good shape.

use crate::command::Command;
use crate::log::Log;
use crate::runner::{Git, GitError};

impl Git {
    /// Cleans up and packs the repository with `git gc`, using git's default settings.
    ///
    /// # Errors
    ///
    /// Fails when git fails, for example when another gc is already running.
    pub fn gc(&self, log: &Log<'_>) -> Result<(), GitError> {
        self.run(&Command::Gc, None, log)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::runner::{Git, GitError};
    use crate::test_repo::{Logged, TempDir, TestRepo};

    /// The `count:` line of `git count-objects -v`: the number of loose objects.
    fn loose_objects(repo: &TestRepo) -> String {
        repo.git_ok(&["count-objects", "-v"])
            .lines()
            .find(|line: &&str| line.starts_with("count: "))
            .unwrap()
            .to_string()
    }

    #[test]
    fn gc_packs_loose_objects() {
        let repo: TestRepo = TestRepo::new();
        repo.write("a.txt", "a\n");
        repo.git_ok(&["add", "a.txt"]);
        repo.git_ok(&["commit", "-q", "-m", "first"]);
        assert_eq!(loose_objects(&repo), "count: 3");
        let logged: Logged = Logged::new();
        repo.git().gc(&logged.log()).unwrap();
        assert_eq!(loose_objects(&repo), "count: 0");
        assert_eq!(logged.lines()[0], "Info: git gc (exit 0)");
    }

    #[test]
    fn gc_outside_a_repository_fails() {
        let dir: TempDir = TempDir::new();
        let git: Git =
            TestRepo::isolated(Git::new(dir.path())).with_env("GIT_CEILING_DIRECTORIES", "/");
        let logged: Logged = Logged::new();
        assert!(matches!(
            git.gc(&logged.log()),
            Err(GitError::Failed { .. })
        ));
    }
}
