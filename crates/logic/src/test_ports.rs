//! A test implementation of [`Ports`] that records calls and returns prepared answers.

use std::cell::RefCell;

use crate::log::LogLevel;
use crate::model::{
    AmendBase, ChangeKind, ChangedFile, Commit, DiffLine, FileDiff, Hunk, LineKind, Refs, Side,
    WorkStatus,
};
use crate::note::{BranchNameProblem, InputProblem};
use crate::ports::{BranchName, CommitMessage, EditedMessage, Failed, Ports, SearchText};

/// Records calls as text and answers from its fields.
#[expect(
    clippy::struct_excessive_bools,
    reason = "a test double with one switch per failing port"
)]
pub struct TestPorts {
    /// Calls and log messages, in order.
    pub calls: RefCell<Vec<String>>,
    /// The answer to `status`; `None` fails.
    pub status: RefCell<Option<WorkStatus>>,
    /// Whether `file_diff` fails.
    pub diff_fails: bool,
    /// The diff returned for every file; by default one built from the path.
    pub diff: Option<FileDiff>,
    /// Whether staging, unstaging, applying patches and discarding fail.
    pub stage_fails: bool,
    /// The answer to `has_commits`; `None` fails.
    pub has_commits: Option<bool>,
    /// The answer to `last_commit_has_parent`; `None` fails.
    pub has_parent: Option<bool>,
    /// The answer to `amended_files`; `None` fails.
    pub amended: Option<Vec<ChangedFile>>,
    /// The answer to `last_commit_message`; `None` fails.
    pub last_message: Option<String>,
    /// What the editor appends to the text; `None` fails.
    pub editor_appends: Option<String>,
    /// Whether the editor exits with an error, after appending `editor_appends`.
    pub editor_exits_with_error: bool,
    /// Whether committing fails.
    pub commit_fails: bool,
    /// The whole history; `commits` returns chunks of it. `None` fails.
    pub history: RefCell<Option<Vec<Commit>>>,
    /// The answer to `refs`; `None` fails.
    pub refs: RefCell<Option<Refs>>,
    /// Whether loading commit details and showing a commit fail.
    pub details_fail: bool,
    /// Whether branch actions fail: creating, checking out, deleting, fetching, resetting,
    /// cherry-picking and rebasing.
    pub branch_fails: bool,
}

impl TestPorts {
    pub fn new() -> Self {
        TestPorts {
            calls: RefCell::new(Vec::new()),
            status: RefCell::new(Some(WorkStatus {
                staged: vec![],
                unstaged: vec![],
            })),
            diff_fails: false,
            diff: None,
            stage_fails: false,
            has_commits: Some(true),
            has_parent: Some(true),
            amended: Some(Vec::new()),
            last_message: Some("Last\n".to_string()),
            editor_appends: Some("edited\n".to_string()),
            editor_exits_with_error: false,
            commit_fails: false,
            history: RefCell::new(Some(Vec::new())),
            refs: RefCell::new(Some(Refs {
                labels: vec![],
                head: None,
            })),
            details_fail: false,
            branch_fails: false,
        }
    }

    /// Ports whose status has the given unstaged and staged paths, all modified.
    pub fn with_files(unstaged: &[&str], staged: &[&str]) -> Self {
        let ports: TestPorts = TestPorts::new();
        *ports.status.borrow_mut() = Some(WorkStatus {
            unstaged: unstaged.iter().map(|p: &&str| modified(p)).collect(),
            staged: staged.iter().map(|p: &&str| modified(p)).collect(),
        });
        ports
    }

    fn branch_result(&self, call: String) -> Result<(), Failed> {
        self.calls.borrow_mut().push(call);
        if self.branch_fails {
            Err(Failed)
        } else {
            Ok(())
        }
    }

    /// Records `call` and returns the prepared diff, or one built from `path`.
    fn diff_result(&self, call: String, path: Option<&String>) -> Result<FileDiff, Failed> {
        self.calls.borrow_mut().push(call);
        if self.diff_fails {
            Err(Failed)
        } else {
            Ok(self
                .diff
                .clone()
                .unwrap_or_else(|| diff_of(path.map_or("", String::as_str))))
        }
    }

    fn details<T>(&self, value: T) -> Result<T, Failed> {
        if self.details_fail {
            Err(Failed)
        } else {
            Ok(value)
        }
    }

    fn record(&self, call: String) -> Result<(), Failed> {
        self.calls.borrow_mut().push(call);
        if self.stage_fails {
            Err(Failed)
        } else {
            Ok(())
        }
    }

    pub fn calls(&self) -> Vec<String> {
        self.calls.borrow().clone()
    }

    pub fn clear(&self) {
        self.calls.borrow_mut().clear();
    }
}

/// A fixture builder: a commit whose values are placeholders, only so that a fixture
/// compiles. Never assume anything about an instance beyond that it compiles: a test sets
/// every field it depends on, for example `Commit { hash, ..placeholder_commit() }`, and
/// relies on nothing else. A new field then needs a value only here.
pub fn placeholder_commit() -> Commit {
    Commit {
        hash: "placeholder hash".to_string(),
        parents: vec!["placeholder parent".to_string()],
        author: "placeholder author".to_string(),
        author_email: "placeholder author email".to_string(),
        date: "placeholder date".to_string(),
        committer: "placeholder committer".to_string(),
        committer_email: "placeholder committer email".to_string(),
        committer_date: "placeholder committer date".to_string(),
        subject: "placeholder subject".to_string(),
    }
}

/// A linear history of `count` commits, newest first, with hashes `c<n>` and subjects
/// `commit <n>`.
pub fn linear_history(count: usize) -> Vec<Commit> {
    (0..count)
        .rev()
        .map(|n: usize| Commit {
            hash: format!("c{n}"),
            parents: if n == 0 {
                vec![]
            } else {
                vec![format!("c{}", n - 1)]
            },
            author: if n.is_multiple_of(2) { "Ada" } else { "Bo" }.to_string(),
            subject: format!("commit {n}"),
            ..placeholder_commit()
        })
        .collect()
}

/// A plain content change, so that single lines of the file can be staged.
pub fn modified(path: &str) -> ChangedFile {
    ChangedFile {
        path: path.to_string(),
        original_path: None,
        kind: ChangeKind::Modified,
        mode_changed: false,
    }
}

/// A diff with one hunk whose lines are `text` split at `|`, all added.
pub fn diff_of(text: &str) -> FileDiff {
    FileDiff {
        header: vec!["diff --git a/f b/f".to_string()],
        hunks: vec![Hunk {
            header: "@@ -1 +1 @@".to_string(),
            old_start: 1,
            old_count: 0,
            new_start: 1,
            new_count: 1,
            lines: text
                .split('|')
                .map(|t: &str| DiffLine {
                    kind: LineKind::Added,
                    text: t.to_string(),
                })
                .collect(),
        }],
        binary: false,
    }
}

impl Ports for TestPorts {
    fn log(&self, level: LogLevel, message: &str) {
        self.calls
            .borrow_mut()
            .push(format!("{level:?}: {message}"));
    }

    fn validate_search(&self, input: &str) -> Result<SearchText, InputProblem> {
        if input.is_empty() {
            Err(InputProblem::EmptySearch)
        } else {
            Ok(SearchText(input.to_string()))
        }
    }

    fn status(&self) -> Result<WorkStatus, Failed> {
        self.calls.borrow_mut().push("status".to_string());
        self.status.borrow().clone().ok_or(Failed)
    }

    fn file_diff(&self, paths: &[String], side: Side) -> Result<FileDiff, Failed> {
        self.diff_result(format!("diff {side:?} {}", paths.join(" ")), paths.last())
    }

    fn untracked_file_diff(&self, path: &str) -> Result<FileDiff, Failed> {
        self.diff_result(format!("diff untracked {path}"), Some(&path.to_string()))
    }

    fn stage_paths(&self, paths: &[String]) -> Result<(), Failed> {
        self.record(format!("stage {}", paths.join(" ")))
    }

    fn stage_all(&self) -> Result<(), Failed> {
        self.record("stage all".to_string())
    }

    fn unstage_paths(&self, paths: &[String]) -> Result<(), Failed> {
        self.record(format!("unstage {}", paths.join(" ")))
    }

    fn remove_from_index(&self, paths: &[String]) -> Result<(), Failed> {
        self.record(format!("remove from index {}", paths.join(" ")))
    }

    fn restore_paths(&self, paths: &[String]) -> Result<(), Failed> {
        self.record(format!("restore {}", paths.join(" ")))
    }

    fn delete_untracked(&self, paths: &[String]) -> Result<(), Failed> {
        self.record(format!("delete untracked {}", paths.join(" ")))
    }

    fn apply_to_index(&self, patch: &str, reverse: bool) -> Result<(), Failed> {
        self.record(format!("apply reverse={reverse}\n{patch}"))
    }

    fn has_commits(&self) -> Result<bool, Failed> {
        self.calls.borrow_mut().push("has commits".to_string());
        self.has_commits.ok_or(Failed)
    }

    fn last_commit_has_parent(&self) -> Result<bool, Failed> {
        self.calls.borrow_mut().push("has parent".to_string());
        self.has_parent.ok_or(Failed)
    }

    fn amended_files(&self, base: AmendBase) -> Result<Vec<ChangedFile>, Failed> {
        self.calls
            .borrow_mut()
            .push(format!("amended files since {base:?}"));
        self.amended.clone().ok_or(Failed)
    }

    fn amended_file_diff(&self, paths: &[String], base: AmendBase) -> Result<FileDiff, Failed> {
        self.diff_result(
            format!("diff amended since {base:?} {}", paths.join(" ")),
            paths.last(),
        )
    }

    fn unstage_from_amend(&self, paths: &[String]) -> Result<(), Failed> {
        self.record(format!("unstage from amend {}", paths.join(" ")))
    }

    fn last_commit_message(&self) -> Result<String, Failed> {
        self.calls.borrow_mut().push("last message".to_string());
        self.last_message.clone().ok_or(Failed)
    }

    fn edit_message(&self, text: &str) -> Result<EditedMessage, Failed> {
        self.calls.borrow_mut().push(format!("edit {text:?}"));
        self.editor_appends
            .as_ref()
            .map(|added: &String| EditedMessage {
                text: format!("{text}{added}"),
                editor_succeeded: !self.editor_exits_with_error,
            })
            .ok_or(Failed)
    }

    fn validate_commit_message(&self, text: &str) -> Result<CommitMessage, InputProblem> {
        if text.trim().is_empty() {
            Err(InputProblem::EmptyCommitMessage)
        } else {
            Ok(CommitMessage(text.to_string()))
        }
    }

    fn commit(&self, message: &CommitMessage, amend: bool) -> Result<(), Failed> {
        self.calls
            .borrow_mut()
            .push(format!("commit amend={amend} {:?}", message.0));
        if self.commit_fails {
            Err(Failed)
        } else {
            Ok(())
        }
    }

    fn commits(&self, skip: usize, count: usize) -> Result<Vec<Commit>, Failed> {
        self.calls
            .borrow_mut()
            .push(format!("commits {skip} {count}"));
        let history: Option<Vec<Commit>> = self.history.borrow().clone();
        history
            .map(|all: Vec<Commit>| all.into_iter().skip(skip).take(count).collect())
            .ok_or(Failed)
    }

    fn refs(&self) -> Result<Refs, Failed> {
        self.calls.borrow_mut().push("refs".to_string());
        self.refs.borrow().clone().ok_or(Failed)
    }

    fn commit_message(&self, hash: &str) -> Result<String, Failed> {
        self.calls.borrow_mut().push(format!("message {hash}"));
        self.details(format!("Message of {hash}\n\nBody\n"))
    }

    fn commit_files(&self, hash: &str) -> Result<Vec<ChangedFile>, Failed> {
        self.calls.borrow_mut().push(format!("files {hash}"));
        self.details(vec![
            modified(&format!("{hash}.txt")),
            modified("other.txt"),
        ])
    }

    fn validate_branch_name(&self, input: &str) -> Result<BranchName, InputProblem> {
        if input.contains(' ') {
            Err(InputProblem::BranchName(
                BranchNameProblem::ForbiddenCharacter(' '),
            ))
        } else {
            Ok(BranchName(input.to_string()))
        }
    }

    fn validate_checkout_choice(&self, input: &str, count: usize) -> Result<usize, InputProblem> {
        match input.parse::<usize>() {
            // A broken validator, to check that logic does not trust it blindly.
            Ok(99) => Ok(98),
            Ok(n) if n >= 1 && n <= count => Ok(n - 1),
            _ => Err(InputProblem::OutOfRange { count }),
        }
    }

    fn create_branch(&self, name: &BranchName, at: &str) -> Result<(), Failed> {
        self.branch_result(format!("create {} at {at}", name.0))
    }

    fn delete_branch(&self, name: &str) -> Result<(), Failed> {
        self.branch_result(format!("delete {name}"))
    }

    fn remote_update(&self) -> Result<(), Failed> {
        self.branch_result("remote update".to_string())
    }

    fn fetch_all_prune(&self) -> Result<(), Failed> {
        self.branch_result("fetch --all --prune".to_string())
    }

    fn show_commit(&self, hash: &str) -> Result<(), Failed> {
        self.calls.borrow_mut().push(format!("show-commit {hash}"));
        self.details(())
    }

    fn reset_keep(&self, hash: &str) -> Result<(), Failed> {
        self.branch_result(format!("reset --keep {hash}"))
    }

    fn move_branch(&self, name: &str, hash: &str) -> Result<(), Failed> {
        self.branch_result(format!("move {name} to {hash}"))
    }

    fn cherry_pick(&self, hash: &str) -> Result<(), Failed> {
        self.branch_result(format!("cherry-pick {hash}"))
    }

    fn rebase(&self, branch: &str, onto: &str) -> Result<(), Failed> {
        self.branch_result(format!("rebase {branch} onto {onto}"))
    }

    fn rebase_interactive(&self, branch: &str, onto: &str) -> Result<(), Failed> {
        self.branch_result(format!("rebase -i {branch} onto {onto}"))
    }

    fn checkout(&self, branch: &str) -> Result<(), Failed> {
        self.branch_result(format!("checkout {branch}"))
    }

    fn commit_file_diff(&self, hash: &str, paths: &[String]) -> Result<FileDiff, Failed> {
        let shown: String = paths.join(" ");
        self.calls.borrow_mut().push(format!("show {hash} {shown}"));
        self.details(diff_of(&format!("{hash}|{shown}|x|y")))
    }
}
