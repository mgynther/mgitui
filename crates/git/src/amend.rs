//! What amending the last commit would contain: the index compared with the commit before
//! it, and taking changes out of it.

use crate::command::Command;
use crate::diff::{FileDiff, parse_diff};
use crate::history::parse_name_status;
use crate::log::Log;
use crate::runner::{Git, GitError, Output};
use crate::status::FileChange;

/// What to compare the index with when amending.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Base {
    /// The parent of the last commit, `HEAD^`.
    ParentOfHead,
    /// Nothing, for amending the first commit: the empty tree.
    Empty,
}

impl Git {
    /// Whether the last commit has a parent.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    pub fn head_has_parent(&self, log: &Log<'_>) -> Result<bool, GitError> {
        let output: Output = self.run(&Command::HasParent, None, log)?;
        Ok(output.code == 0)
    }

    /// The files whose index content differs from `base`, as `git diff --cached` shows them.
    ///
    /// # Errors
    ///
    /// Fails when git fails or its output cannot be parsed.
    pub fn staged_since(&self, base: Base, log: &Log<'_>) -> Result<Vec<FileChange>, GitError> {
        let command: Command = Command::StagedSince {
            base: self.base_name(base, log)?,
        };
        let output: Output = self.run(&command, None, log)?;
        parse_name_status(&output.stdout)
    }

    /// The diff of `paths` in the index against `base`.
    ///
    /// # Errors
    ///
    /// Fails when git fails.
    pub fn diff_since(
        &self,
        paths: &[String],
        base: Base,
        log: &Log<'_>,
    ) -> Result<FileDiff, GitError> {
        let command: Command = Command::DiffSince {
            base: self.base_name(base, log)?,
            paths: paths.to_vec(),
        };
        let output: Output = self.run(&command, None, log)?;
        Ok(parse_diff(&output.stdout))
    }

    /// Sets the index entries of `paths` to the parent of the last commit, which takes their
    /// changes out of the amended commit. Paths that the parent does not have are removed from
    /// the index.
    ///
    /// # Errors
    ///
    /// Fails when git fails, for example when the last commit has no parent.
    pub fn reset_to_parent(&self, paths: &[String], log: &Log<'_>) -> Result<(), GitError> {
        let command: Command = Command::ResetToParent {
            paths: paths.to_vec(),
        };
        self.run(&command, None, log)?;
        Ok(())
    }

    /// The name git understands for `base`.
    fn base_name(&self, base: Base, log: &Log<'_>) -> Result<String, GitError> {
        match base {
            Base::ParentOfHead => Ok("HEAD^".to_string()),
            Base::Empty => {
                let output: Output = self.run(&Command::EmptyTree, None, log)?;
                Ok(output.stdout.trim().to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Base;
    use crate::runner::GitError;
    use crate::status::{ChangeKind, FileChange};
    use crate::test_repo::{Logged, TestRepo};

    fn paths(names: &[&str]) -> Vec<String> {
        names.iter().map(|n: &&str| (*n).to_string()).collect()
    }

    fn kinds(changes: &[FileChange]) -> Vec<(String, ChangeKind)> {
        changes
            .iter()
            .map(|c: &FileChange| (c.path.clone(), c.kind))
            .collect()
    }

    /// A repository whose last commit modifies `keep.txt`, adds `added.txt` and deletes
    /// `gone.txt`.
    fn amendable() -> TestRepo {
        let repo: TestRepo = TestRepo::new();
        repo.write("keep.txt", "1\n2\n");
        repo.write("gone.txt", "g\n");
        repo.git_ok(&["add", "."]);
        repo.git_ok(&["commit", "-q", "-m", "base"]);
        repo.write("keep.txt", "1\nTWO\n");
        repo.write("added.txt", "a\n");
        repo.git_ok(&["rm", "-q", "gone.txt"]);
        repo.git_ok(&["add", "."]);
        repo.git_ok(&["commit", "-q", "-m", "last"]);
        repo
    }

    #[test]
    fn the_last_commit_against_its_parent() {
        let repo: TestRepo = amendable();
        let logged: Logged = Logged::new();
        assert_eq!(repo.git().head_has_parent(&logged.log()), Ok(true));
        let staged: Vec<FileChange> = repo
            .git()
            .staged_since(Base::ParentOfHead, &logged.log())
            .unwrap();
        assert_eq!(
            kinds(&staged),
            vec![
                ("added.txt".to_string(), ChangeKind::Added),
                ("gone.txt".to_string(), ChangeKind::Deleted),
                ("keep.txt".to_string(), ChangeKind::Modified),
            ]
        );
        let diff: crate::diff::FileDiff = repo
            .git()
            .diff_since(&paths(&["keep.txt"]), Base::ParentOfHead, &logged.log())
            .unwrap();
        assert_eq!(diff.hunks[0].header, "@@ -1,2 +1,2 @@");
    }

    #[test]
    fn reset_to_parent_takes_changes_out_of_the_amended_commit() {
        let repo: TestRepo = amendable();
        let logged: Logged = Logged::new();
        repo.git()
            .reset_to_parent(&paths(&["added.txt", "gone.txt"]), &logged.log())
            .unwrap();
        let staged: Vec<FileChange> = repo
            .git()
            .staged_since(Base::ParentOfHead, &logged.log())
            .unwrap();
        assert_eq!(
            kinds(&staged),
            vec![("keep.txt".to_string(), ChangeKind::Modified)]
        );
        assert_eq!(
            repo.git_ok(&["status", "--porcelain"]),
            "D  added.txt\nAD gone.txt\n?? added.txt\n"
        );
    }

    #[test]
    fn the_first_commit_against_nothing() {
        let repo: TestRepo = TestRepo::new();
        repo.write("a.txt", "a\n");
        repo.git_ok(&["add", "."]);
        repo.git_ok(&["commit", "-q", "-m", "first"]);
        let logged: Logged = Logged::new();
        assert_eq!(repo.git().head_has_parent(&logged.log()), Ok(false));
        let staged: Vec<FileChange> = repo.git().staged_since(Base::Empty, &logged.log()).unwrap();
        assert_eq!(
            kinds(&staged),
            vec![("a.txt".to_string(), ChangeKind::Added)]
        );
        let diff: crate::diff::FileDiff = repo
            .git()
            .diff_since(&paths(&["a.txt"]), Base::Empty, &logged.log())
            .unwrap();
        assert_eq!(diff.hunks[0].lines[0].text, "a");
        assert!(matches!(
            repo.git()
                .reset_to_parent(&paths(&["a.txt"]), &logged.log()),
            Err(GitError::Failed { .. })
        ));
    }

    #[test]
    fn failures() {
        let repo: TestRepo = amendable();
        let logged: Logged = Logged::new();
        let missing: crate::runner::Git = repo.git().with_program("/nonexistent/git");
        assert!(missing.head_has_parent(&logged.log()).is_err());
        assert!(missing.staged_since(Base::Empty, &logged.log()).is_err());
        assert!(
            missing
                .diff_since(&paths(&["a"]), Base::Empty, &logged.log())
                .is_err()
        );
        assert!(
            missing
                .staged_since(Base::ParentOfHead, &logged.log())
                .is_err()
        );
        assert!(
            missing
                .diff_since(&paths(&["a"]), Base::ParentOfHead, &logged.log())
                .is_err()
        );
    }
}
