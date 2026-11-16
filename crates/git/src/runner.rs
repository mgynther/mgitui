//! Running git as a child process.

use std::ffi::OsString;
use std::fmt;
use std::io::{Read, Write};
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

/// The configuration to run `command` with: [`FIXED_CONFIG`] for output this application
/// parses, none for output the user reads, so that their colors and other settings apply.
fn fixed_config(command: &Command) -> &'static [&'static str] {
    match command {
        Command::ShowCommit { .. } => &[],
        _ => &FIXED_CONFIG,
    }
}

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
    /// A file in the working tree could not be read.
    Read(String),
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
            GitError::Read(reason) => write!(f, "could not read a file: {reason}"),
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
        for config in fixed_config(command) {
            process.arg("-c").arg(config);
        }
        self.prepare(&mut process, command, &args);
        process
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

    /// Runs `command` with the terminal as its input and output, so that git can run an editor.
    /// Standard error is copied to the terminal as it arrives and also logged, so that the log
    /// shows why git stopped. Standard output goes to the terminal only.
    ///
    /// # Errors
    ///
    /// Returns [`GitError::Spawn`] when git cannot be started and [`GitError::Failed`] when it
    /// exits with a code that is not a success code for `command`.
    pub fn run_interactive(&self, command: &Command, log: &Log<'_>) -> Result<Output, GitError> {
        let args: Vec<String> = command.args();
        let shown: String = format!("git {}", args.join(" "));
        let mut process: process::Command = process::Command::new(&self.program);
        for config in fixed_config(command) {
            process.arg("-c").arg(config);
        }
        self.prepare(&mut process, command, &args);
        process.stdin(Stdio::inherit()).stdout(Stdio::inherit());
        let (status, stderr): (process::ExitStatus, String) =
            run_copying_stderr(process).map_err(|error: std::io::Error| {
                let error: GitError = GitError::Spawn(error.to_string());
                log(LogLevel::Error, &format!("{shown}: {error}"));
                error
            })?;
        let code: Option<i32> = status.code();
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
                stdout: String::new(),
                stderr,
            }),
            _ => Err(GitError::Failed { code, stderr }),
        }
    }

    /// Sets the arguments, directory and environment of a git process. The command's own
    /// environment comes first so that [`Git::with_env`] can override it.
    fn prepare(&self, process: &mut process::Command, command: &Command, args: &[String]) {
        process
            .args(args)
            .current_dir(&self.dir)
            .envs(command.env().iter().copied())
            .envs(self.env.iter().map(|(k, v): &(OsString, OsString)| (k, v)));
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

/// Runs `process` with standard error through a pipe, copying it to this program's standard
/// error and returning a copy. The process is dropped once started: it holds the pipe's
/// writing end, and reading would never see the end of the output while that is open.
fn run_copying_stderr(
    mut process: process::Command,
) -> std::io::Result<(process::ExitStatus, String)> {
    std::io::pipe().and_then(
        move |(reader, writer): (std::io::PipeReader, std::io::PipeWriter)| {
            process.stderr(writer);
            let spawned: std::io::Result<process::Child> = process.spawn();
            drop(process);
            spawned.and_then(|mut child: process::Child| {
                let copied: String = copy_to_stderr(reader);
                child
                    .wait()
                    .map(|status: process::ExitStatus| (status, copied))
            })
        },
    )
}

/// Copies everything from `reader` to standard error until the writing end is closed, and
/// returns it. A failing standard error does not stop the copying.
fn copy_to_stderr(mut reader: std::io::PipeReader) -> String {
    let mut copied: Vec<u8> = Vec::new();
    let mut buffer: [u8; 4096] = [0; 4096];
    while let Ok(read) = reader.read(&mut buffer) {
        let Some(chunk) = buffer.get(..read).filter(|chunk: &&[u8]| !chunk.is_empty()) else {
            break;
        };
        let _ = std::io::stderr().write_all(chunk);
        copied.extend_from_slice(chunk);
    }
    String::from_utf8_lossy(&copied).into_owned()
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

    use super::{FIXED_CONFIG, Git, GitError, Output, fixed_config};
    use crate::command::Command;
    use crate::test_repo::{Logged, TempDir, TestRepo, fake_git};

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
    fn interactive_command_logs_its_standard_error() {
        let dir: TempDir = TempDir::new();
        let program: String = fake_git(&dir, "echo refused >&2; exit 3");
        let logged: Logged = Logged::new();
        let result: Result<Output, GitError> = Git::new(dir.path())
            .with_program(&program)
            .run_interactive(&Command::Status, &logged.log());
        assert_eq!(
            result,
            Err(GitError::Failed {
                code: Some(3),
                stderr: "refused\n".to_string()
            })
        );
        assert_eq!(
            logged.lines(),
            vec![
                "Error: git status --porcelain=v2 -z --untracked-files=all (exit 3)",
                "Error: refused\n"
            ]
        );
    }

    #[test]
    fn interactive_command_stopped_by_signal_fails() {
        let dir: TempDir = TempDir::new();
        let program: String = fake_git(&dir, "kill -9 $$");
        let logged: Logged = Logged::new();
        let result: Result<Output, GitError> = Git::new(dir.path())
            .with_program(&program)
            .run_interactive(&Command::Status, &logged.log());
        assert_eq!(
            result,
            Err(GitError::Failed {
                code: None,
                stderr: String::new()
            })
        );
        assert_eq!(
            logged.lines(),
            vec!["Error: git status --porcelain=v2 -z --untracked-files=all (exit signal)"]
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
        let cases: [(GitError, &str); 5] = [
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
            (
                GitError::Read("gone".to_string()),
                "could not read a file: gone",
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

    #[test]
    fn output_for_the_user_keeps_the_user_configuration() {
        assert_eq!(fixed_config(&Command::Status), FIXED_CONFIG);
        let show: Command = Command::ShowCommit {
            hash: "abc".to_string(),
        };
        let none: &[&str] = &[];
        assert_eq!(fixed_config(&show), none);
    }
}
