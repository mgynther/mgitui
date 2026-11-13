//! Temporary directories and repositories for tests, isolated from the user's git setup.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::log::LogLevel;
use crate::runner::Git;

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

/// A directory under the system temp directory, removed when dropped.
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new() -> Self {
        let id: usize = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let path: PathBuf =
            std::env::temp_dir().join(format!("mgitui-git-test-{}-{id}", process::id()));
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

/// A git repository in a temporary directory with fixed identity and dates.
pub struct TestRepo {
    dir: TempDir,
}

impl TestRepo {
    /// Creates an empty repository with `main` as the initial branch.
    pub fn new() -> Self {
        let repo: TestRepo = TestRepo {
            dir: TempDir::new(),
        };
        repo.git_ok(&["init", "-q", "-b", "main"]);
        repo
    }

    /// Adds settings that make git independent of the user's configuration and environment.
    pub fn isolated(git: Git) -> Git {
        git.with_env("GIT_CONFIG_GLOBAL", "/dev/null")
            .with_env("GIT_CONFIG_NOSYSTEM", "1")
            .with_env("GIT_AUTHOR_NAME", "Ada Author")
            .with_env("GIT_AUTHOR_EMAIL", "ada@example.com")
            .with_env("GIT_AUTHOR_DATE", "1735787045 +0000")
            .with_env("GIT_COMMITTER_NAME", "Cy Committer")
            .with_env("GIT_COMMITTER_EMAIL", "cy@example.com")
            .with_env("GIT_COMMITTER_DATE", "1735787045 +0000")
            .with_env("TZ", "UTC")
            .with_env("LC_ALL", "C")
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// The git runner for this repository.
    pub fn git(&self) -> Git {
        TestRepo::isolated(Git::new(self.path()))
    }

    /// Runs raw git for test setup and returns its standard output.
    pub fn git_ok(&self, args: &[&str]) -> String {
        let mut command: process::Command = process::Command::new("git");
        command.args(args).current_dir(self.path());
        for (key, value) in [
            ("GIT_CONFIG_GLOBAL", "/dev/null"),
            ("GIT_CONFIG_NOSYSTEM", "1"),
            ("GIT_AUTHOR_NAME", "Ada Author"),
            ("GIT_AUTHOR_EMAIL", "ada@example.com"),
            ("GIT_AUTHOR_DATE", "1735787045 +0000"),
            ("GIT_COMMITTER_NAME", "Cy Committer"),
            ("GIT_COMMITTER_EMAIL", "cy@example.com"),
            ("GIT_COMMITTER_DATE", "1735787045 +0000"),
        ] {
            command.env(key, value);
        }
        let output: process::Output = command.output().unwrap();
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }

    /// Writes a file relative to the repository root, creating directories as needed.
    pub fn write(&self, path: &str, content: &str) {
        let full: PathBuf = self.path().join(path);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(full, content).unwrap();
    }
}

/// Collects log lines as `Level: message` for assertions.
pub struct Logged(RefCell<Vec<String>>);

impl Logged {
    pub fn new() -> Self {
        Logged(RefCell::new(Vec::new()))
    }

    pub fn log(&self) -> impl Fn(LogLevel, &str) + '_ {
        |level: LogLevel, message: &str| {
            self.0.borrow_mut().push(format!("{level:?}: {message}"));
        }
    }

    pub fn lines(&self) -> Vec<String> {
        self.0.borrow().clone()
    }
}
