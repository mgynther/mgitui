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

/// Runs raw git in `dir` with a configuration isolated from the user's, for test setup.
pub fn git_ok(dir: &Path, args: &[&str]) -> String {
    let output: process::Output = process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "Ada Author")
        .env("GIT_AUTHOR_EMAIL", "ada@example.com")
        .env("GIT_AUTHOR_DATE", "1735787045 +0000")
        .env("GIT_COMMITTER_NAME", "Cy Committer")
        .env("GIT_COMMITTER_EMAIL", "cy@example.com")
        .env("GIT_COMMITTER_DATE", "1735787045 +0000")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

/// Creates an empty repository in `dir` with `main` as the initial branch.
pub fn init_repo(dir: &Path) {
    git_ok(dir, &["init", "-q", "-b", "main"]);
}
