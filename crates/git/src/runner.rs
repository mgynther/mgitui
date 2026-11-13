//! Running git as a child process.

use std::ffi::OsString;
use std::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{self, Stdio};

use crate::command::Command;
use crate::log::{Log, LogLevel};

/// Configuration that keeps git's output in the format this application parses, whatever the
/// user's git configuration is.
const FIXED_CONFIG: [&str; 5] = [
    "color.ui=never",
    "core.quotePath=false",
    "diff.noprefix=false",
    "diff.mnemonicPrefix=false",
    "log.showSignature=false",
];

/// The result of a git command that succeeded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    /// The exit code.
    pub code: i32,
    /// Standard output, decoded as UTF-8 with invalid bytes replaced.
    pub stdout: String,
    /// Standard error, decoded as UTF-8 with invalid bytes replaced.
    pub stderr: String,
}

/// Why a git command failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitError {
    /// Git could not be started.
    Spawn(String),
    /// Git exited with a failing exit code, or was stopped by a signal (`None`).
    Failed {
        /// The exit code.
        code: Option<i32>,
        /// Standard error.
        stderr: String,
    },
    /// The output could not be parsed.
    Parse(String),
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitError::Spawn(reason) => write!(f, "could not run git: {reason}"),
            GitError::Failed {
                code: Some(code), ..
            } => write!(f, "git failed with exit code {code}"),
            GitError::Failed { code: None, .. } => write!(f, "git was stopped by a signal"),
            GitError::Parse(reason) => write!(f, "could not parse git output: {reason}"),
        }
    }
}

/// Runs git commands in one repository directory.
#[derive(Debug, Clone)]
pub struct Git {
    program: OsString,
    dir: PathBuf,
    env: Vec<(OsString, OsString)>,
    temp_dir: PathBuf,
}

impl Git {
    /// Runs `git` from `PATH` in `dir`.
    #[must_use]
    pub fn new(dir: &Path) -> Self {
        Git {
            program: OsString::from("git"),
            dir: dir.to_path_buf(),
            env: Vec::new(),
            temp_dir: std::env::temp_dir(),
        }
    }

    /// Writes temporary files, such as commit messages, to `dir` instead of the system temp
    /// directory.
    #[must_use]
    pub fn with_temp_dir(mut self, dir: &Path) -> Self {
        self.temp_dir = dir.to_path_buf();
        self
    }

    /// The directory for temporary files.
    #[must_use]
    pub fn temp_dir(&self) -> &Path {
        &self.temp_dir
    }

    /// Uses `program` instead of `git` from `PATH`.
    #[must_use]
    pub fn with_program(mut self, program: &str) -> Self {
        self.program = OsString::from(program);
        self
    }

    /// Sets an environment variable for every git command.
    #[must_use]
    pub fn with_env(mut self, key: &str, value: &str) -> Self {
        self.env.push((OsString::from(key), OsString::from(value)));
        self
    }

    /// Returns the same git setup running in another directory.
    #[must_use]
    pub fn in_dir(&self, dir: &Path) -> Self {
        Git {
            dir: dir.to_path_buf(),
            ..self.clone()
        }
    }

    /// The directory git runs in.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Runs `command`, writing `stdin` to git's standard input when given. The command, its exit
    /// code and standard error are logged; standard output is logged when the command fails.
    ///
    /// # Errors
    ///
    /// Returns [`GitError::Spawn`] when git cannot be started and [`GitError::Failed`] when it
    /// exits with a code that is not a success code for `command`.
    pub fn run(
        &self,
        command: &Command,
        stdin: Option<&str>,
        log: &Log<'_>,
    ) -> Result<Output, GitError> {
        let args: Vec<String> = command.args();
        let shown: String = format!("git {}", args.join(" "));
        let mut process: process::Command = process::Command::new(&self.program);
        for config in FIXED_CONFIG {
            process.arg("-c").arg(config);
        }
        process
            .args(&args)
            .current_dir(&self.dir)
            .envs(self.env.iter().map(|(k, v): &(OsString, OsString)| (k, v)))
            .stdin(if stdin.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let output: process::Output = match spawn_and_wait(&mut process, stdin) {
            Ok(output) => output,
            Err(error) => {
                let error: GitError = GitError::Spawn(error.to_string());
                log(LogLevel::Error, &format!("{shown}: {error}"));
                return Err(error);
            }
        };
        let stdout: String = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr: String = String::from_utf8_lossy(&output.stderr).into_owned();
        let code: Option<i32> = output.status.code();
        let succeeded: bool = code.is_some_and(|c: i32| command.success_codes().contains(&c));
        let level: LogLevel = if succeeded {
            LogLevel::Info
        } else {
            LogLevel::Error
        };
        let code_text: String = code.map_or_else(|| "signal".to_string(), |c: i32| c.to_string());
        log(level, &format!("{shown} (exit {code_text})"));
        if !stderr.is_empty() {
            log(level, &stderr);
        }
        match code {
            Some(code) if succeeded => Ok(Output {
                code,
                stdout,
                stderr,
            }),
            _ => {
                if !stdout.is_empty() {
                    log(LogLevel::Error, &stdout);
                }
                Err(GitError::Failed { code, stderr })
            }
        }
    }

    /// The root directory of the repository that contains the directory git runs in.
    ///
    /// # Errors
    ///
    /// Fails when the directory is not inside a git repository or git cannot be run.
    pub fn toplevel(&self, log: &Log<'_>) -> Result<PathBuf, GitError> {
        let output: Output = self.run(&Command::ShowToplevel, None, log)?;
        Ok(PathBuf::from(output.stdout.trim_end_matches('\n')))
    }
}

fn spawn_and_wait(
    process: &mut process::Command,
    stdin: Option<&str>,
) -> std::io::Result<process::Output> {
    let mut child: process::Child = process.spawn()?;
    // Git reads all of its input before writing output, so writing the whole input first
    // cannot block.
    if let (Some(input), Some(mut pipe)) = (stdin, child.stdin.take()) {
        pipe.write_all(input.as_bytes())?;
    }
    child.wait_with_output()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{Git, GitError, Output};
    use crate::command::Command;
    use crate::test_repo::{Logged, TempDir, TestRepo};

    #[test]
    fn toplevel_of_repository_and_subdirectory() {
        let repo: TestRepo = TestRepo::new();
        repo.write("sub/file.txt", "x\n");
        let logged: Logged = Logged::new();
        let top: PathBuf = repo.git().toplevel(&logged.log()).unwrap();
        assert_eq!(top, repo.path().canonicalize().unwrap());
        let sub: PathBuf = repo
            .git()
            .in_dir(&repo.path().join("sub"))
            .toplevel(&logged.log())
            .unwrap();
        assert_eq!(sub, top);
        assert_eq!(
            logged.lines(),
            vec![
                "Info: git rev-parse --show-toplevel (exit 0)",
                "Info: git rev-parse --show-toplevel (exit 0)",
            ]
        );
    }

    #[test]
    fn toplevel_outside_repository_fails_and_logs() {
        let dir: TempDir = TempDir::new();
        let logged: Logged = Logged::new();
        let result: Result<PathBuf, GitError> =
            TestRepo::isolated(Git::new(dir.path())).toplevel(&logged.log());
        assert!(matches!(
            result,
            Err(GitError::Failed {
                code: Some(128),
                ..
            })
        ));
        let lines: Vec<String> = logged.lines();
        assert_eq!(lines[0], "Error: git rev-parse --show-toplevel (exit 128)");
        assert!(lines[1].starts_with("Error: fatal: not a git repository"));
    }

    #[test]
    fn missing_program_fails_to_spawn() {
        let dir: TempDir = TempDir::new();
        let logged: Logged = Logged::new();
        let result: Result<PathBuf, GitError> = Git::new(dir.path())
            .with_program("/nonexistent/git")
            .toplevel(&logged.log());
        assert!(matches!(result, Err(GitError::Spawn(_))));
        assert!(
            logged.lines()[0]
                .starts_with("Error: git rev-parse --show-toplevel: could not run git")
        );
    }

    /// Creates an executable shell script that stands in for git. The file is written by a
    /// child process: if this process wrote it, another test thread could fork while the file
    /// is open for writing, and running the script would fail with "Text file busy".
    fn fake_git(dir: &TempDir, body: &str) -> String {
        let path: PathBuf = dir.path().join("fake-git");
        let status: std::process::ExitStatus = std::process::Command::new("sh")
            .arg("-c")
            .arg("printf '#!/bin/sh\\n%s\\n' \"$1\" > \"$2\" && chmod +x \"$2\"")
            .arg("sh")
            .arg(body)
            .arg(&path)
            .status()
            .unwrap();
        assert!(status.success());
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn stdin_is_passed_and_output_returned() {
        let dir: TempDir = TempDir::new();
        let program: String = fake_git(&dir, "cat; echo note >&2");
        let logged: Logged = Logged::new();
        let output: Output = Git::new(dir.path())
            .with_program(&program)
            .run(&Command::ShowToplevel, Some("input\n"), &logged.log())
            .unwrap();
        assert_eq!(
            output,
            Output {
                code: 0,
                stdout: "input\n".to_string(),
                stderr: "note\n".to_string()
            }
        );
        assert_eq!(
            logged.lines(),
            vec![
                "Info: git rev-parse --show-toplevel (exit 0)",
                "Info: note\n"
            ]
        );
    }

    #[test]
    fn input_not_read_by_git_fails() {
        let dir: TempDir = TempDir::new();
        let program: String = fake_git(&dir, "exit 0");
        let logged: Logged = Logged::new();
        let input: String = "x".repeat(1 << 20);
        let result: Result<Output, GitError> = Git::new(dir.path()).with_program(&program).run(
            &Command::ShowToplevel,
            Some(&input),
            &logged.log(),
        );
        assert!(matches!(result, Err(GitError::Spawn(_))));
    }

    #[test]
    fn stdout_of_failing_command_is_logged() {
        let dir: TempDir = TempDir::new();
        let program: String = fake_git(&dir, "echo out; exit 3");
        let logged: Logged = Logged::new();
        let result: Result<Output, GitError> = Git::new(dir.path()).with_program(&program).run(
            &Command::ShowToplevel,
            None,
            &logged.log(),
        );
        assert_eq!(
            result,
            Err(GitError::Failed {
                code: Some(3),
                stderr: String::new()
            })
        );
        assert_eq!(
            logged.lines(),
            vec![
                "Error: git rev-parse --show-toplevel (exit 3)",
                "Error: out\n"
            ]
        );
    }

    #[test]
    fn command_stopped_by_signal_fails() {
        let dir: TempDir = TempDir::new();
        let program: String = fake_git(&dir, "kill -9 $$");
        let logged: Logged = Logged::new();
        let result: Result<Output, GitError> = Git::new(dir.path()).with_program(&program).run(
            &Command::ShowToplevel,
            None,
            &logged.log(),
        );
        assert_eq!(
            result,
            Err(GitError::Failed {
                code: None,
                stderr: String::new()
            })
        );
        assert_eq!(
            logged.lines(),
            vec!["Error: git rev-parse --show-toplevel (exit signal)"]
        );
    }

    #[test]
    fn error_messages() {
        let cases: [(GitError, &str); 4] = [
            (GitError::Spawn("no".to_string()), "could not run git: no"),
            (
                GitError::Failed {
                    code: Some(1),
                    stderr: String::new(),
                },
                "git failed with exit code 1",
            ),
            (
                GitError::Failed {
                    code: None,
                    stderr: String::new(),
                },
                "git was stopped by a signal",
            ),
            (
                GitError::Parse("bad".to_string()),
                "could not parse git output: bad",
            ),
        ];
        for (error, message) in cases {
            assert_eq!(error.to_string(), message);
        }
    }

    #[test]
    fn dir_is_where_git_runs() {
        let dir: TempDir = TempDir::new();
        assert_eq!(Git::new(dir.path()).dir(), dir.path());
    }
}
