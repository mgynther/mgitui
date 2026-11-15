//! Helpers for wiring tests: temporary directories and repositories.

use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

/// A clock that always returns the same time.
pub fn fixed_clock() -> u64 {
    1_735_787_045
}

/// A directory under the system temp directory, removed when dropped.
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new() -> Self {
        let id: usize = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let path: PathBuf =
            std::env::temp_dir().join(format!("mgitui-wiring-test-{}-{id}", process::id()));
        std::fs::create_dir_all(&path).unwrap();
        TempDir(path)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The environment that isolates git in tests from the user's configuration and makes commits
/// reproducible.
const ISOLATED: [(&str, &str); 8] = [
    ("GIT_CONFIG_GLOBAL", "/dev/null"),
    ("GIT_CONFIG_NOSYSTEM", "1"),
    ("GIT_AUTHOR_NAME", "Ada Author"),
    ("GIT_AUTHOR_EMAIL", "ada@example.com"),
    ("GIT_AUTHOR_DATE", "1735787045 +0000"),
    ("GIT_COMMITTER_NAME", "Cy Committer"),
    ("GIT_COMMITTER_EMAIL", "cy@example.com"),
    ("GIT_COMMITTER_DATE", "1735787045 +0000"),
];

/// Runs raw git in `dir` for test setup, with the [`ISOLATED`] environment.
pub fn git_ok(dir: &Path, args: &[&str]) -> String {
    let output: process::Output = process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .envs(ISOLATED)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

/// The pager for git in tests. It reads and drops the output: when tests run in a terminal,
/// commands that page, like `git show`, would otherwise wait in less until q is pressed.
pub const TEST_PAGER: &str = "cat >/dev/null";

/// The application's git runner for `dir` in tests. It is isolated like [`git_ok`], so the
/// application's commits match those of the test setup, and it also fixes the timezone, the
/// locale and the pager, so that output does not depend on the machine running the tests.
pub fn test_git(dir: &Path) -> git::Git {
    ISOLATED
        .iter()
        .fold(
            git::Git::new(dir),
            |git: git::Git, (key, value): &(&str, &str)| git.with_env(key, value),
        )
        .with_env("TZ", "UTC")
        .with_env("LC_ALL", "C")
        .with_env("GIT_PAGER", TEST_PAGER)
}

/// Creates an empty repository in `dir` with `main` as the initial branch.
pub fn init_repo(dir: &Path) {
    git_ok(dir, &["init", "-q", "-b", "main"]);
}
