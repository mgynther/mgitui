//! Creating and checking out branches.

use crate::command::Command;
use crate::log::Log;
use crate::runner::{Git, GitError, Output};

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

    /// Moves existing local branch `name` to commit `to`, leaving the working tree as it is.
    ///
    /// # Errors
    ///
    /// Fails when git fails, for example when the branch is checked out.
    pub fn move_branch(&self, name: &str, to: &str, log: &Log<'_>) -> Result<(), GitError> {
        let command: Command = Command::MoveBranch {
            name: name.to_string(),
            to: to.to_string(),
        };
        self.run(&command, None, log)?;
        Ok(())
    }

    /// Deletes local branch `name`, also when it is not merged, as after a squash merge.
    ///
    /// # Errors
    ///
    /// Fails when git fails, for example when the branch is checked out.
    pub fn delete_branch(&self, name: &str, log: &Log<'_>) -> Result<(), GitError> {
        let command: Command = Command::DeleteBranch {
            name: name.to_string(),
        };
        self.run(&command, None, log)?;
        Ok(())
    }

    /// Fetches all remotes. Git and ssh ask for passwords on the terminal, so the caller must
    /// hand the terminal over; the output is still captured and logged.
    ///
    /// # Errors
    ///
    /// Fails when git fails, for example when a remote cannot be reached.
    pub fn remote_update(&self, log: &Log<'_>) -> Result<(), GitError> {
        self.run(&Command::RemoteUpdate, None, log)?;
        Ok(())
    }

    /// Fetches all remotes pruning stale remote branches. Git and ssh ask for passwords on the
    /// terminal, so the caller must hand the terminal over; the output is still captured and
    /// logged.
    ///
    /// # Errors
    ///
    /// Fails when git fails, for example when a remote cannot be reached.
    pub fn fetch_all_prune(&self, log: &Log<'_>) -> Result<(), GitError> {
        self.run(&Command::FetchAllPrune, None, log)?;
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

    /// Moves the checked out branch to commit `hash` and updates the working tree to match.
    /// Git refuses, changing nothing, when this would overwrite local changes or untracked
    /// files.
    ///
    /// # Errors
    ///
    /// Fails when git fails, for example when an untracked file is in the way.
    pub fn reset_keep(&self, hash: &str, log: &Log<'_>) -> Result<(), GitError> {
        let command: Command = Command::ResetKeep {
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
    /// Fails when git cannot be run or exits with a code other than 0 or 1.
    pub fn rebase_interactive(
        &self,
        branch: &str,
        onto: &str,
        log: &Log<'_>,
    ) -> Result<RebaseEnd, GitError> {
        let command: Command = Command::RebaseInteractive {
            onto: onto.to_string(),
            branch: branch.to_string(),
        };
        let output: Output = self.run_interactive(&command, log)?;
        Ok(rebase_end(output.code))
    }

    /// Checks out `branch` and rebases it onto `onto`. A conflict leaves the rebase in
    /// progress for the user to resolve.
    ///
    /// # Errors
    ///
    /// Fails when git cannot be run or exits with a code other than 0 or 1.
    pub fn rebase(&self, branch: &str, onto: &str, log: &Log<'_>) -> Result<RebaseEnd, GitError> {
        let command: Command = Command::Rebase {
            onto: onto.to_string(),
            branch: branch.to_string(),
        };
        let output: Output = self.run(&command, None, log)?;
        Ok(rebase_end(output.code))
    }

    /// Whether a rebase is in progress, stopped for the user to continue or abort it.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    pub fn rebase_in_progress(&self, log: &Log<'_>) -> Result<bool, GitError> {
        let output: Output = self.run(&Command::RebaseStatePaths, None, log)?;
        Ok(output
            .stdout
            .lines()
            .any(|path: &str| self.dir().join(path).is_dir()))
    }
}

/// How a rebase that git ran to its end ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebaseEnd {
    /// The branch was rebased.
    Finished,
    /// Git exited with code 1: it stopped with the rebase in progress, refused to start, or
    /// the user cancelled an interactive rebase.
    NotFinished,
}

fn rebase_end(code: i32) -> RebaseEnd {
    if code == 0 {
        RebaseEnd::Finished
    } else {
        RebaseEnd::NotFinished
    }
}

#[cfg(test)]
mod tests {
    use super::RebaseEnd;
    use crate::runner::GitError;
    use crate::test_repo::{Logged, TestRepo};

    #[test]
    fn move_branch_moves_only_a_branch_that_is_not_checked_out() {
        let repo: TestRepo = TestRepo::new();
        repo.git_ok(&["commit", "-q", "--allow-empty", "-m", "one"]);
        let first: String = repo.git_ok(&["rev-parse", "HEAD"]).trim().to_string();
        repo.git_ok(&["commit", "-q", "--allow-empty", "-m", "two"]);
        let second: String = repo.git_ok(&["rev-parse", "HEAD"]).trim().to_string();
        repo.git_ok(&["branch", "topic", &first]);
        let logged: Logged = Logged::new();
        repo.git()
            .move_branch("topic", &second, &logged.log())
            .unwrap();
        assert_eq!(repo.git_ok(&["rev-parse", "topic"]).trim(), second);
        assert!(matches!(
            repo.git().move_branch("main", &first, &logged.log()),
            Err(GitError::Failed { .. })
        ));
        assert_eq!(repo.git_ok(&["rev-parse", "main"]).trim(), second);
    }

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
        assert_eq!(repo.git().rebase_in_progress(&logged.log()), Ok(false));
        assert_eq!(
            repo.git().rebase("topic", "main", &logged.log()),
            Ok(RebaseEnd::Finished)
        );
        assert_eq!(
            repo.git_ok(&["log", "--format=%s", "topic"]),
            "topic work\nmain work\nbase\n"
        );
        assert_eq!(repo.git_ok(&["branch", "--show-current"]), "topic\n");
        repo.git_ok(&["switch", "-q", "-c", "clash", "main~1"]);
        repo.write("m.txt", "other\n");
        repo.git_ok(&["add", "."]);
        repo.git_ok(&["commit", "-q", "-m", "clash"]);
        assert_eq!(
            repo.git().rebase("clash", "main", &logged.log()),
            Ok(RebaseEnd::NotFinished)
        );
        assert_eq!(repo.git().rebase_in_progress(&logged.log()), Ok(true));
        // From a subdirectory too: git gives the paths relative to where it runs.
        let sub: std::path::PathBuf = repo.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        assert_eq!(
            crate::runner::Git::new(&sub)
                .with_env("GIT_CONFIG_GLOBAL", "/dev/null")
                .rebase_in_progress(&logged.log()),
            Ok(true)
        );
        repo.git_ok(&["rebase", "--abort"]);
        repo.write("m.txt", "unstaged\n");
        assert_eq!(
            repo.git().rebase("clash", "main", &logged.log()),
            Ok(RebaseEnd::NotFinished)
        );
        assert_eq!(repo.git().rebase_in_progress(&logged.log()), Ok(false));
        let missing: crate::runner::Git = repo.git().with_program("/nonexistent/git");
        assert!(missing.rebase_in_progress(&logged.log()).is_err());
        assert!(missing.rebase("clash", "main", &logged.log()).is_err());
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
        assert_eq!(
            squash.rebase_interactive("topic", "main", &logged.log()),
            Ok(RebaseEnd::Finished)
        );
        assert_eq!(
            repo.git_ok(&["log", "--format=%s", "topic"]),
            "one\nmain work\nbase\n"
        );
        let lines: Vec<String> = logged.lines();
        assert_eq!(lines[0], "Info: git rebase -i main topic (exit 0)");
        assert!(lines[1].contains("Successfully rebased and updated refs/heads/topic."));
        let emptied: crate::runner::Git = repo.git().with_env("GIT_SEQUENCE_EDITOR", "sed -i d");
        assert_eq!(
            emptied.rebase_interactive("main", "topic", &logged.log()),
            Ok(RebaseEnd::NotFinished)
        );
        assert!(
            logged
                .lines()
                .iter()
                .any(|line: &String| line == "Info: error: nothing to do\n")
        );
        let missing: crate::runner::Git = repo.git().with_program("/nonexistent/git");
        assert!(matches!(
            missing.rebase_interactive("topic", "main", &logged.log()),
            Err(GitError::Spawn(_))
        ));
    }

    #[test]
    fn delete_merged_and_unmerged_branches() {
        let repo: TestRepo = TestRepo::new();
        repo.git_ok(&["commit", "-q", "--allow-empty", "-m", "base"]);
        repo.git_ok(&["branch", "merged"]);
        repo.git_ok(&["switch", "-q", "-c", "unmerged"]);
        repo.git_ok(&["commit", "-q", "--allow-empty", "-m", "work"]);
        repo.git_ok(&["switch", "-q", "main"]);
        let logged: Logged = Logged::new();
        repo.git().delete_branch("merged", &logged.log()).unwrap();
        repo.git().delete_branch("unmerged", &logged.log()).unwrap();
        assert!(matches!(
            repo.git().delete_branch("main", &logged.log()),
            Err(GitError::Failed { .. })
        ));
        assert_eq!(
            repo.git_ok(&["branch", "--format=%(refname:short)"]),
            "main\n"
        );
    }

    #[test]
    fn remote_update_fetches_all_remotes() {
        let upstream: TestRepo = TestRepo::new();
        upstream.git_ok(&["commit", "-q", "--allow-empty", "-m", "upstream work"]);
        let repo: TestRepo = TestRepo::new();
        let url: String = upstream.path().to_string_lossy().into_owned();
        repo.git_ok(&["remote", "add", "origin", &url]);
        let logged: Logged = Logged::new();
        repo.git().remote_update(&logged.log()).unwrap();
        assert_eq!(
            repo.git_ok(&["log", "--format=%s", "origin/main"]),
            "upstream work\n"
        );
        assert_eq!(logged.lines()[0], "Info: git remote update (exit 0)");
        repo.git_ok(&["remote", "set-url", "origin", "/nonexistent/repo"]);
        assert!(matches!(
            repo.git().remote_update(&logged.log()),
            Err(GitError::Failed { .. })
        ));
    }

    #[test]
    fn fetch_all_prune_fetches_all_remotes() {
        let upstream: TestRepo = TestRepo::new();
        upstream.git_ok(&["commit", "-q", "--allow-empty", "-m", "upstream work"]);
        upstream.git_ok(&["branch", "-c", "another-branch"]);

        let repo: TestRepo = TestRepo::new();
        let url: String = upstream.path().to_string_lossy().into_owned();
        repo.git_ok(&["remote", "add", "origin", &url]);
        let logged: Logged = Logged::new();
        repo.git().fetch_all_prune(&logged.log()).unwrap();
        assert_eq!(
            repo.git_ok(&["branch", "-r"]),
            "  origin/another-branch\n  origin/main\n"
        );
        assert_eq!(logged.lines()[0], "Info: git fetch --all --prune (exit 0)");

        upstream.git_ok(&["branch", "-D", "another-branch"]);
        let logged_before: usize = logged.lines().len();
        repo.git().fetch_all_prune(&logged.log()).unwrap();
        assert_eq!(repo.git_ok(&["branch", "-r"]), "  origin/main\n");
        assert_eq!(
            logged.lines()[logged_before],
            "Info: git fetch --all --prune (exit 0)"
        );

        repo.git_ok(&["remote", "set-url", "origin", "/nonexistent/repo"]);
        assert!(matches!(
            repo.git().fetch_all_prune(&logged.log()),
            Err(GitError::Failed { .. })
        ));
    }

    #[test]
    fn reset_keep_moves_the_branch_and_keeps_untracked_files() {
        let repo: TestRepo = TestRepo::new();
        repo.git_ok(&["commit", "-q", "--allow-empty", "-m", "one"]);
        let first: String = repo.git_ok(&["rev-parse", "HEAD"]).trim().to_string();
        repo.write("a.txt", "a\n");
        repo.git_ok(&["add", "."]);
        repo.git_ok(&["commit", "-q", "-m", "two"]);
        let second: String = repo.git_ok(&["rev-parse", "HEAD"]).trim().to_string();
        let logged: Logged = Logged::new();
        repo.git().reset_keep(&first, &logged.log()).unwrap();
        assert_eq!(repo.git_ok(&["rev-parse", "main"]).trim(), first);
        assert!(!repo.path().join("a.txt").exists());
        assert_eq!(
            logged.lines()[0],
            format!("Info: git reset --keep {first} (exit 0)")
        );

        repo.write("a.txt", "untracked\n");
        assert!(matches!(
            repo.git().reset_keep(&second, &logged.log()),
            Err(GitError::Failed { .. })
        ));
        assert_eq!(repo.git_ok(&["rev-parse", "main"]).trim(), first);
        assert_eq!(
            std::fs::read_to_string(repo.path().join("a.txt")).unwrap(),
            "untracked\n"
        );
    }
}
