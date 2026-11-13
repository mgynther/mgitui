//! Creating and checking out branches.

use crate::command::Command;
use crate::log::Log;
use crate::runner::{Git, GitError};

impl Git {
    /// Creates branch `name` at commit `at` without checking it out.
    ///
    /// # Errors
    ///
    /// Fails when git fails, for example when the branch already exists.
    pub fn create_branch(&self, name: &str, at: &str, log: &Log<'_>) -> Result<(), GitError> {
        let command: Command = Command::CreateBranch {
            name: name.to_string(),
            at: at.to_string(),
        };
        self.run(&command, None, log)?;
        Ok(())
    }

    /// Checks out local branch `branch`.
    ///
    /// # Errors
    ///
    /// Fails when git fails, for example when local changes would be overwritten.
    pub fn switch(&self, branch: &str, log: &Log<'_>) -> Result<(), GitError> {
        let command: Command = Command::Switch {
            branch: branch.to_string(),
        };
        self.run(&command, None, log)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::runner::GitError;
    use crate::test_repo::{Logged, TestRepo};

    #[test]
    fn create_and_switch() {
        let repo: TestRepo = TestRepo::new();
        repo.write("a.txt", "a\n");
        repo.git_ok(&["add", "."]);
        repo.git_ok(&["commit", "-q", "-m", "one"]);
        repo.git_ok(&["commit", "-q", "--allow-empty", "-m", "two"]);
        let first: String = repo.git_ok(&["rev-parse", "HEAD~1"]).trim().to_string();
        let logged: Logged = Logged::new();
        repo.git()
            .create_branch("old", &first, &logged.log())
            .unwrap();
        assert_eq!(repo.git_ok(&["rev-parse", "old"]).trim(), first);
        assert_eq!(repo.git_ok(&["branch", "--show-current"]), "main\n");
        repo.git().switch("old", &logged.log()).unwrap();
        assert_eq!(repo.git_ok(&["branch", "--show-current"]), "old\n");
        assert!(matches!(
            repo.git().create_branch("old", &first, &logged.log()),
            Err(GitError::Failed { .. })
        ));
        repo.write("a.txt", "changed\n");
        repo.git()
            .create_branch("other", &first, &logged.log())
            .unwrap();
        repo.git_ok(&["switch", "-q", "main"]);
        repo.git_ok(&["stash", "-q"]);
        repo.write("a.txt", "dirty\n");
        repo.git_ok(&["commit", "-q", "-am", "three"]);
        repo.write("a.txt", "dirty again\n");
        assert!(matches!(
            repo.git().switch("old", &logged.log()),
            Err(GitError::Failed { .. })
        ));
    }
}
