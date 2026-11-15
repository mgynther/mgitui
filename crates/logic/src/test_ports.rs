//! A test implementation of [`Ports`] that records calls and returns prepared answers.

use std::cell::RefCell;

use crate::log::LogLevel;
use crate::model::{
    ChangeKind, ChangedFile, Commit, DiffLine, FileDiff, Hunk, LineKind, Refs, Side, WorkStatus,
};
use crate::ports::{BranchName, CommitMessage, Failed, Ports, SearchText};

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
    /// The answer to `last_commit_message`; `None` fails.
    pub last_message: Option<String>,
    /// What the editor appends to the text; `None` fails.
    pub editor_appends: Option<String>,
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
            status: RefCell::new(Some(WorkStatus::default())),
            diff_fails: false,
            diff: None,
            stage_fails: false,
            has_commits: Some(true),
            last_message: Some("Last\n".to_string()),
            editor_appends: Some("edited\n".to_string()),
            commit_fails: false,
            history: RefCell::new(Some(Vec::new())),
            refs: RefCell::new(Some(Refs::default())),
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
            date: "2025-01-02 03:04".to_string(),
            subject: format!("commit {n}"),
        })
        .collect()
}

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

    fn validate_search(&self, input: &str) -> Result<SearchText, String> {
        if input.is_empty() {
            Err("search text is empty".to_string())
        } else {
            Ok(SearchText(input.to_string()))
        }
    }

    fn status(&self) -> Result<WorkStatus, Failed> {
        self.calls.borrow_mut().push("status".to_string());
        self.status.borrow().clone().ok_or(Failed)
    }

    fn file_diff(&self, file: &ChangedFile, side: Side) -> Result<FileDiff, Failed> {
        self.calls
            .borrow_mut()
            .push(format!("diff {side:?} {}", file.path));
        if self.diff_fails {
            Err(Failed)
        } else {
            Ok(self.diff.clone().unwrap_or_else(|| diff_of(&file.path)))
        }
    }

    fn stage_file(&self, file: &ChangedFile) -> Result<(), Failed> {
        self.record(format!("stage {}", file.path))
    }

    fn unstage_file(&self, file: &ChangedFile) -> Result<(), Failed> {
        self.record(format!("unstage {}", file.path))
    }

    fn discard_file(&self, file: &ChangedFile) -> Result<(), Failed> {
        self.record(format!("discard {}", file.path))
    }

    fn apply_to_index(&self, patch: &str, reverse: bool) -> Result<(), Failed> {
        self.record(format!("apply reverse={reverse}\n{patch}"))
    }

    fn has_commits(&self) -> Result<bool, Failed> {
        self.calls.borrow_mut().push("has commits".to_string());
        self.has_commits.ok_or(Failed)
    }

    fn last_commit_message(&self) -> Result<String, Failed> {
        self.calls.borrow_mut().push("last message".to_string());
        self.last_message.clone().ok_or(Failed)
    }

    fn edit_message(&self, text: &str) -> Result<String, Failed> {
        self.calls.borrow_mut().push(format!("edit {text:?}"));
        self.editor_appends
            .as_ref()
            .map(|added: &String| format!("{text}{added}"))
            .ok_or(Failed)
    }

    fn validate_commit_message(&self, text: &str) -> Result<CommitMessage, String> {
        if text.trim().is_empty() {
            Err("commit message is empty".to_string())
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

    fn validate_branch_name(&self, input: &str) -> Result<BranchName, String> {
        if input.is_empty() || input.contains(' ') {
            Err("invalid branch name".to_string())
        } else {
            Ok(BranchName(input.to_string()))
        }
    }

    fn validate_checkout_choice(&self, input: &str, count: usize) -> Result<usize, String> {
        match input.parse::<usize>() {
            // A broken validator, to check that logic does not trust it blindly.
            Ok(99) => Ok(98),
            Ok(n) if n >= 1 && n <= count => Ok(n - 1),
            _ => Err(format!("choice must be a number from 1 to {count}")),
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

    fn cherry_pick(&self, hash: &str) -> Result<(), Failed> {
        self.branch_result(format!("cherry-pick {hash}"))
    }

    fn rebase(&self, branch: &str, onto: &str, interactive: bool) -> Result<(), Failed> {
        let kind: &str = if interactive { "rebase -i" } else { "rebase" };
        self.branch_result(format!("{kind} {branch} onto {onto}"))
    }

    fn checkout(&self, branch: &str) -> Result<(), Failed> {
        self.branch_result(format!("checkout {branch}"))
    }

    fn commit_file_diff(&self, hash: &str, file: &ChangedFile) -> Result<FileDiff, Failed> {
        self.calls
            .borrow_mut()
            .push(format!("show {hash} {}", file.path));
        self.details(diff_of(&format!("{hash}|{}|x|y", file.path)))
    }
}
