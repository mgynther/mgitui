//! Staging and unstaging.

use crate::command::Command;
use crate::log::{Log, LogLevel};
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

    /// Unstages all changes of `paths` with `git restore --staged`, restoring them in the index
    /// from `HEAD`.
    ///
    /// # Errors
    ///
    /// Fails when git fails, for example when there is no commit yet.
    pub fn restore_staged(&self, paths: &[String], log: &Log<'_>) -> Result<(), GitError> {
        let command: Command = Command::RestoreStaged {
            paths: paths.to_vec(),
        };
        self.run(&command, None, log)?;
        Ok(())
    }

    /// Removes `paths` from the index with `git rm --cached`, keeping the files.
    ///
    /// # Errors
    ///
    /// Fails when git fails, for example when a path is not in the index.
    pub fn remove_cached(&self, paths: &[String], log: &Log<'_>) -> Result<(), GitError> {
        let command: Command = Command::RemoveCached {
            paths: paths.to_vec(),
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

impl Git {
    /// Whether file `path`, relative to the repository root, still has the conflict markers
    /// git writes into a conflicted file: a line starting with `<<<<<<<` or `>>>>>>>`. The
    /// `=======` between them is not looked for, because it also underlines headings in
    /// Markdown. A missing file has none, as when one side of the conflict deleted it.
    ///
    /// # Errors
    ///
    /// Fails when the file exists but cannot be read.
    pub fn has_conflict_markers(&self, path: &str, log: &Log<'_>) -> Result<bool, GitError> {
        match std::fs::read(self.dir().join(path)) {
            Ok(content) => Ok(content.split(|byte: &u8| *byte == b'\n').any(is_marker)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => {
                log(LogLevel::Error, &format!("reading {path}: {error}"));
                Err(GitError::Read(format!("{path}: {error}")))
            }
        }
    }
}

/// Whether `line` starts a conflict's first or last marker: seven `<` or `>`, then a space,
/// a carriage return or the end of the line.
fn is_marker(line: &[u8]) -> bool {
    [b"<<<<<<<", b">>>>>>>"].iter().any(|marker: &&[u8; 7]| {
        line.strip_prefix(&marker[..])
            .is_some_and(|rest: &[u8]| matches!(rest.first(), None | Some(b' ' | b'\r')))
    })
}

#[cfg(test)]
mod tests {
    use super::is_marker;
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
        assert!(
            repo.git()
                .restore_staged(&paths(&["a.txt"]), &logged.log())
                .is_err()
        );
        repo.git()
            .remove_cached(&paths(&["a.txt"]), &logged.log())
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
            .restore_staged(&paths(&["a.txt"]), &logged.log())
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
        assert!(git.restore_staged(&paths(&["x"]), &logged.log()).is_err());
        assert!(git.remove_cached(&paths(&["x"]), &logged.log()).is_err());
        assert!(git.apply_cached("", false, &logged.log()).is_err());
        assert!(git.restore(&paths(&["x"]), &logged.log()).is_err());
        assert!(git.clean(&paths(&["x"]), &logged.log()).is_err());
        assert!(
            repo.git()
                .remove_cached(&paths(&["missing"]), &logged.log())
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

    #[test]
    fn conflict_marker_lines() {
        let cases: [(&str, bool); 8] = [
            ("<<<<<<< HEAD", true),
            (">>>>>>> 9b0c2a9 (topic)", true),
            ("<<<<<<<", true),
            (">>>>>>>\r", true),
            ("=======", false),
            ("<<<<<<<<", false),
            (" <<<<<<< indented", false),
            ("text", false),
        ];
        for (line, marker) in cases {
            assert_eq!(is_marker(line.as_bytes()), marker, "{line:?}");
        }
    }

    #[test]
    fn conflict_markers_in_files() {
        let repo: TestRepo = TestRepo::new();
        let logged: Logged = Logged::new();
        repo.write(
            "conflicted.txt",
            "<<<<<<< HEAD\nmain\n=======\ntopic\n>>>>>>> t\n",
        );
        repo.write("resolved.md", "Title\n=======\n\ntext\n");
        assert_eq!(
            repo.git()
                .has_conflict_markers("conflicted.txt", &logged.log()),
            Ok(true)
        );
        assert_eq!(
            repo.git()
                .has_conflict_markers("resolved.md", &logged.log()),
            Ok(false)
        );
        assert_eq!(
            repo.git()
                .has_conflict_markers("deleted.txt", &logged.log()),
            Ok(false)
        );
        std::fs::create_dir(repo.path().join("dir")).unwrap();
        assert!(matches!(
            repo.git().has_conflict_markers("dir", &logged.log()),
            Err(GitError::Read(_))
        ));
        assert_eq!(logged.lines().len(), 1);
        assert!(logged.lines()[0].starts_with("Error: reading dir: "));
    }
}
