//! Staging and unstaging.

use crate::command::Command;
use crate::log::Log;
use crate::runner::{Git, GitError, Output};

impl Git {
    /// Stages all changes of `paths`, including untracked and deleted files.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    pub fn add(&self, paths: &[String], log: &Log<'_>) -> Result<(), GitError> {
        self.run(
            &Command::Add {
                paths: paths.to_vec(),
            },
            None,
            log,
        )?;
        Ok(())
    }

    /// Stages all changes in the working tree, including deletions and untracked files.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    pub fn add_all(&self, log: &Log<'_>) -> Result<(), GitError> {
        self.run(&Command::AddAll, None, log)?;
        Ok(())
    }

    /// Whether the repository has at least one commit.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    pub fn has_head(&self, log: &Log<'_>) -> Result<bool, GitError> {
        let output: Output = self.run(&Command::HasHead, None, log)?;
        Ok(output.code == 0)
    }

    /// Unstages all changes of `paths`. In a repository without commits the paths are removed
    /// from the index instead.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    pub fn unstage(&self, paths: &[String], log: &Log<'_>) -> Result<(), GitError> {
        let paths: Vec<String> = paths.to_vec();
        let command: Command = if self.has_head(log)? {
            Command::RestoreStaged { paths }
        } else {
            Command::RemoveCached { paths }
        };
        self.run(&command, None, log)?;
        Ok(())
    }

    /// Discards the unstaged changes of tracked `paths`, restoring them from the index. Staged
    /// changes are kept.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    pub fn restore(&self, paths: &[String], log: &Log<'_>) -> Result<(), GitError> {
        self.run(
            &Command::Restore {
                paths: paths.to_vec(),
            },
            None,
            log,
        )?;
        Ok(())
    }

    /// Deletes untracked `paths`.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    pub fn clean(&self, paths: &[String], log: &Log<'_>) -> Result<(), GitError> {
        self.run(
            &Command::Clean {
                paths: paths.to_vec(),
            },
            None,
            log,
        )?;
        Ok(())
    }

    /// Applies `patch` to the index, in reverse when `reverse` is true.
    ///
    /// # Errors
    ///
    /// Fails when git fails, for example when the patch does not apply.
    pub fn apply_cached(&self, patch: &str, reverse: bool, log: &Log<'_>) -> Result<(), GitError> {
        self.run(&Command::ApplyCached { reverse }, Some(patch), log)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::runner::GitError;
    use crate::status::{ChangeKind, Status};
    use crate::test_repo::{Logged, TestRepo};

    fn status(repo: &TestRepo) -> Status {
        repo.git().status(&Logged::new().log()).unwrap()
    }

    fn paths(names: &[&str]) -> Vec<String> {
        names.iter().map(|n: &&str| (*n).to_string()).collect()
    }

    #[test]
    fn add_and_unstage_without_commits() {
        let repo: TestRepo = TestRepo::new();
        repo.write("a.txt", "a\n");
        let logged: Logged = Logged::new();
        assert!(!repo.git().has_head(&logged.log()).unwrap());
        repo.git().add(&paths(&["a.txt"]), &logged.log()).unwrap();
        assert_eq!(status(&repo).staged[0].kind, ChangeKind::Added);
        repo.git()
            .unstage(&paths(&["a.txt"]), &logged.log())
            .unwrap();
        assert_eq!(status(&repo).unstaged[0].kind, ChangeKind::Untracked);
        assert!(
            logged
                .lines()
                .iter()
                .any(|l: &String| l.contains("git rm --cached"))
        );
    }

    #[test]
    fn stage_deletion_and_unstage_with_commits() {
        let repo: TestRepo = TestRepo::new();
        repo.write("a.txt", "a\n");
        repo.git_ok(&["add", "."]);
        repo.git_ok(&["commit", "-q", "-m", "init"]);
        std::fs::remove_file(repo.path().join("a.txt")).unwrap();
        let logged: Logged = Logged::new();
        repo.git().add(&paths(&["a.txt"]), &logged.log()).unwrap();
        assert_eq!(status(&repo).staged[0].kind, ChangeKind::Deleted);
        repo.git()
            .unstage(&paths(&["a.txt"]), &logged.log())
            .unwrap();
        assert_eq!(status(&repo).unstaged[0].kind, ChangeKind::Deleted);
        assert!(
            logged
                .lines()
                .iter()
                .any(|l: &String| l.contains("git restore --staged"))
        );
    }

    #[test]
    fn restore_keeps_staged_changes_and_clean_deletes_untracked() {
        let repo: TestRepo = TestRepo::new();
        repo.write("f.txt", "a\n");
        repo.git_ok(&["add", "."]);
        repo.git_ok(&["commit", "-q", "-m", "init"]);
        repo.write("f.txt", "staged\n");
        repo.git_ok(&["add", "f.txt"]);
        repo.write("f.txt", "unstaged\n");
        repo.write("new.txt", "n\n");
        let logged: Logged = Logged::new();
        repo.git()
            .restore(&paths(&["f.txt"]), &logged.log())
            .unwrap();
        repo.git()
            .clean(&paths(&["new.txt"]), &logged.log())
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(repo.path().join("f.txt")).unwrap(),
            "staged\n"
        );
        assert!(!repo.path().join("new.txt").exists());
        let after: Status = status(&repo);
        assert_eq!(after.unstaged, Vec::new());
        assert_eq!(after.staged[0].kind, ChangeKind::Modified);
    }

    #[test]
    fn apply_patch_to_index_and_back() {
        let repo: TestRepo = TestRepo::new();
        repo.write("f.txt", "a\nb\n");
        repo.git_ok(&["add", "."]);
        repo.git_ok(&["commit", "-q", "-m", "init"]);
        let patch: &str =
            "diff --git a/f.txt b/f.txt\n--- a/f.txt\n+++ b/f.txt\n@@ -1,2 +1,2 @@\n a\n-b\n+c\n";
        let logged: Logged = Logged::new();
        repo.git()
            .apply_cached(patch, false, &logged.log())
            .unwrap();
        assert_eq!(repo.git_ok(&["show", ":f.txt"]), "a\nc\n");
        repo.git().apply_cached(patch, true, &logged.log()).unwrap();
        assert_eq!(repo.git_ok(&["show", ":f.txt"]), "a\nb\n");
        assert!(matches!(
            repo.git().apply_cached(patch, true, &logged.log()),
            Err(GitError::Failed { .. })
        ));
    }

    #[test]
    fn failures() {
        let repo: TestRepo = TestRepo::new();
        let logged: Logged = Logged::new();
        let git: crate::runner::Git = repo.git().with_program("/nonexistent/git");
        assert!(git.add(&paths(&["x"]), &logged.log()).is_err());
        assert!(git.has_head(&logged.log()).is_err());
        assert!(git.unstage(&paths(&["x"]), &logged.log()).is_err());
        assert!(git.apply_cached("", false, &logged.log()).is_err());
        assert!(git.restore(&paths(&["x"]), &logged.log()).is_err());
        assert!(git.clean(&paths(&["x"]), &logged.log()).is_err());
        assert!(
            repo.git()
                .unstage(&paths(&["missing"]), &logged.log())
                .is_err()
        );
    }

    #[test]
    fn add_all_stages_changes_deletions_and_untracked_files() {
        let repo: TestRepo = TestRepo::new();
        repo.write("changed.txt", "a\n");
        repo.write("deleted.txt", "d\n");
        repo.git_ok(&["add", "."]);
        repo.git_ok(&["commit", "-q", "-m", "init"]);
        repo.write("changed.txt", "b\n");
        std::fs::remove_file(repo.path().join("deleted.txt")).unwrap();
        repo.write("dir/new.txt", "n\n");
        let logged: Logged = Logged::new();
        repo.git().add_all(&logged.log()).unwrap();
        assert_eq!(
            repo.git_ok(&["status", "--porcelain"]),
            "M  changed.txt\nD  deleted.txt\nA  dir/new.txt\n"
        );
        assert_eq!(logged.lines(), vec!["Info: git add -A (exit 0)"]);
        let missing: crate::runner::Git = repo.git().with_program("/nonexistent/git");
        assert!(missing.add_all(&logged.log()).is_err());
    }
}
