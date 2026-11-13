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

    /// Deletes local branch `name`. Git refuses branches that are not merged.
    ///
    /// # Errors
    ///
    /// Fails when git fails, for example when the branch is not merged or is checked out.
    pub fn delete_branch(&self, name: &str, log: &Log<'_>) -> Result<(), GitError> {
        let command: Command = Command::DeleteBranch {
            name: name.to_string(),
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

impl Git {
    /// Applies commit `hash` onto the checked out branch. A conflict leaves the cherry-pick in
    /// progress for the user to resolve.
    ///
    /// # Errors
    ///
    /// Fails when git fails, for example on a conflict.
    pub fn cherry_pick(&self, hash: &str, log: &Log<'_>) -> Result<(), GitError> {
        let command: Command = Command::CherryPick {
            hash: hash.to_string(),
        };
        self.run(&command, None, log)?;
        Ok(())
    }

    /// Checks out `branch` and rebases it onto `onto` interactively. Git runs Vim on the
    /// terminal for the todo list and messages, so the caller must hand the terminal over. A
    /// conflict leaves the rebase in progress for the user to resolve.
    ///
    /// # Errors
    ///
    /// Fails when git fails, for example on a conflict or when the todo list is emptied.
    pub fn rebase_interactive(
        &self,
        branch: &str,
        onto: &str,
        log: &Log<'_>,
    ) -> Result<(), GitError> {
        let command: Command = Command::RebaseInteractive {
            onto: onto.to_string(),
            branch: branch.to_string(),
        };
        self.run_interactive(&command, log)
    }

    /// Checks out `branch` and rebases it onto `onto`. A conflict leaves the rebase in
    /// progress for the user to resolve.
    ///
    /// # Errors
    ///
    /// Fails when git fails, for example on a conflict.
    pub fn rebase(&self, branch: &str, onto: &str, log: &Log<'_>) -> Result<(), GitError> {
        let command: Command = Command::Rebase {
            onto: onto.to_string(),
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

    #[test]
    fn cherry_pick_and_conflict() {
        let repo: TestRepo = TestRepo::new();
        repo.write("a.txt", "a\n");
        repo.git_ok(&["add", "."]);
        repo.git_ok(&["commit", "-q", "-m", "base"]);
        repo.git_ok(&["switch", "-q", "-c", "side"]);
        repo.write("b.txt", "b\n");
        repo.git_ok(&["add", "."]);
        repo.git_ok(&["commit", "-q", "-m", "add b"]);
        repo.write("a.txt", "side\n");
        repo.git_ok(&["commit", "-q", "-am", "change a"]);
        let add_b: String = repo.git_ok(&["rev-parse", "HEAD~1"]).trim().to_string();
        let change_a: String = repo.git_ok(&["rev-parse", "HEAD"]).trim().to_string();
        repo.git_ok(&["switch", "-q", "main"]);
        repo.write("a.txt", "main\n");
        repo.git_ok(&["commit", "-q", "-am", "main change"]);
        let logged: Logged = Logged::new();
        repo.git().cherry_pick(&add_b, &logged.log()).unwrap();
        assert_eq!(repo.git_ok(&["log", "-1", "--format=%s"]), "add b\n");
        assert!(matches!(
            repo.git().cherry_pick(&change_a, &logged.log()),
            Err(GitError::Failed { .. })
        ));
        assert!(repo.path().join(".git/CHERRY_PICK_HEAD").exists());
    }

    #[test]
    fn rebase_and_conflict() {
        let repo: TestRepo = TestRepo::new();
        repo.write("a.txt", "a\n");
        repo.git_ok(&["add", "."]);
        repo.git_ok(&["commit", "-q", "-m", "base"]);
        repo.git_ok(&["switch", "-q", "-c", "topic"]);
        repo.write("t.txt", "t\n");
        repo.git_ok(&["add", "."]);
        repo.git_ok(&["commit", "-q", "-m", "topic work"]);
        repo.git_ok(&["switch", "-q", "main"]);
        repo.write("m.txt", "m\n");
        repo.git_ok(&["add", "."]);
        repo.git_ok(&["commit", "-q", "-m", "main work"]);
        let logged: Logged = Logged::new();
        repo.git().rebase("topic", "main", &logged.log()).unwrap();
        assert_eq!(
            repo.git_ok(&["log", "--format=%s", "topic"]),
            "topic work\nmain work\nbase\n"
        );
        assert_eq!(repo.git_ok(&["branch", "--show-current"]), "topic\n");
        repo.git_ok(&["switch", "-q", "-c", "clash", "main~1"]);
        repo.write("m.txt", "other\n");
        repo.git_ok(&["add", "."]);
        repo.git_ok(&["commit", "-q", "-m", "clash"]);
        assert!(matches!(
            repo.git().rebase("clash", "main", &logged.log()),
            Err(GitError::Failed { .. })
        ));
        assert!(repo.path().join(".git/rebase-merge").exists());
    }

    #[test]
    fn interactive_rebase_with_scripted_editors() {
        let repo: TestRepo = TestRepo::new();
        repo.git_ok(&["commit", "-q", "--allow-empty", "-m", "base"]);
        repo.git_ok(&["switch", "-q", "-c", "topic"]);
        repo.git_ok(&["commit", "-q", "--allow-empty", "-m", "one"]);
        repo.git_ok(&["commit", "-q", "--allow-empty", "-m", "two"]);
        repo.git_ok(&["switch", "-q", "main"]);
        repo.git_ok(&["commit", "-q", "--allow-empty", "-m", "main work"]);
        let logged: Logged = Logged::new();
        let squash: crate::runner::Git = repo
            .git()
            .with_env("GIT_SEQUENCE_EDITOR", "sed -i 2s/^pick/fixup/")
            .with_env("GIT_EDITOR", "true");
        squash
            .rebase_interactive("topic", "main", &logged.log())
            .unwrap();
        assert_eq!(
            repo.git_ok(&["log", "--format=%s", "topic"]),
            "one\nmain work\nbase\n"
        );
        assert_eq!(
            logged.lines(),
            vec!["Info: git rebase -i main topic (exit 0)"]
        );
        let emptied: crate::runner::Git = repo.git().with_env("GIT_SEQUENCE_EDITOR", "sed -i d");
        assert!(matches!(
            emptied.rebase_interactive("main", "topic", &logged.log()),
            Err(GitError::Failed { .. })
        ));
        let missing: crate::runner::Git = repo.git().with_program("/nonexistent/git");
        assert!(matches!(
            missing.rebase_interactive("topic", "main", &logged.log()),
            Err(GitError::Spawn(_))
        ));
    }

    #[test]
    fn delete_merged_branch_only() {
        let repo: TestRepo = TestRepo::new();
        repo.git_ok(&["commit", "-q", "--allow-empty", "-m", "base"]);
        repo.git_ok(&["branch", "merged"]);
        repo.git_ok(&["switch", "-q", "-c", "unmerged"]);
        repo.git_ok(&["commit", "-q", "--allow-empty", "-m", "work"]);
        repo.git_ok(&["switch", "-q", "main"]);
        let logged: Logged = Logged::new();
        repo.git().delete_branch("merged", &logged.log()).unwrap();
        assert!(matches!(
            repo.git().delete_branch("unmerged", &logged.log()),
            Err(GitError::Failed { .. })
        ));
        assert_eq!(
            repo.git_ok(&["branch", "--format=%(refname:short)"]),
            "main\nunmerged\n"
        );
    }
}
