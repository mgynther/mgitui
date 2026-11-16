//! Tests that run the real `mgitui` binary in a pseudo-terminal, which `script` from util-linux
//! provides, for what only a real terminal shows: raw mode, drawing, resizing, handing the
//! terminal to Vim and restoring it. Stand-in `vim` and `stty` scripts on `PATH` take the place
//! of the real programs where a test needs to control them.
#![cfg(target_os = "linux")]

// Clippy allows `unwrap` in test code only inside a `cfg(test)` module.
#[cfg(test)]
mod pty {
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::process::{Child, ChildStdin, Command, ExitStatus, Output, Stdio};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

    /// How long to wait for the app to show something before the test fails.
    const TIMEOUT: Duration = Duration::from_secs(10);

    /// The start of the tab row, which every frame draws first.
    const FRAME_START: &str = " F1 Branches ";

    /// The environment that isolates git from the user's configuration and makes commits
    /// reproducible, the same as the wiring unit tests use.
    const ISOLATED: [(&str, &str); 10] = [
        ("GIT_CONFIG_GLOBAL", "/dev/null"),
        ("GIT_CONFIG_NOSYSTEM", "1"),
        ("GIT_AUTHOR_NAME", "Ada Author"),
        ("GIT_AUTHOR_EMAIL", "ada@example.com"),
        ("GIT_AUTHOR_DATE", "1735787045 +0000"),
        ("GIT_COMMITTER_NAME", "Cy Committer"),
        ("GIT_COMMITTER_EMAIL", "cy@example.com"),
        ("GIT_COMMITTER_DATE", "1735787045 +0000"),
        ("TZ", "UTC"),
        ("LC_ALL", "C"),
    ];

    /// A directory under the system temp directory, removed when dropped.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let id: usize = NEXT_ID.fetch_add(1, Ordering::Relaxed);
            let path: PathBuf = std::env::temp_dir()
                .join(format!("mgitui-terminal-test-{}-{id}", std::process::id()));
            std::fs::create_dir_all(&path).unwrap();
            TempDir(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn git(dir: &Path, args: &[&str]) -> String {
        let output: Output = Command::new("git")
            .args(args)
            .current_dir(dir)
            .envs(ISOLATED)
            .output()
            .unwrap();
        assert!(output.status.success(), "git {args:?} failed");
        String::from_utf8(output.stdout).unwrap()
    }

    /// A repository with one commit `first commit` on `main`.
    fn repository() -> TempDir {
        let dir: TempDir = TempDir::new();
        git(dir.path(), &["init", "-q", "-b", "main"]);
        git(
            dir.path(),
            &["commit", "-q", "--allow-empty", "-m", "first commit"],
        );
        dir
    }

    /// Writes an executable shell script `name` with `body` into `dir`. A child process writes
    /// it: if this process did, another test thread could fork while the file is open for
    /// writing, and running the script would fail with "Text file busy".
    fn write_script(dir: &Path, name: &str, body: &str) {
        let path: PathBuf = dir.join(name);
        let status: ExitStatus = Command::new("sh")
            .arg("-c")
            .arg("printf '#!/bin/sh\\n%s\\n' \"$1\" > \"$2\" && chmod +x \"$2\"")
            .arg("sh")
            .arg(body)
            .arg(&path)
            .status()
            .unwrap();
        assert!(status.success());
    }

    /// What `script` showed once `mgitui` and the shell after it exited.
    struct Finished {
        /// The exit code of `mgitui`.
        code: String,
        /// The terminal settings after `mgitui` exited, from `stty -a`.
        settings: String,
        /// Everything written to the terminal, with escape sequences.
        raw: String,
    }

    /// `mgitui` running in an 80 by 24 pseudo-terminal in a repository, with stand-in scripts
    /// first on `PATH`. After the app exits, the shell prints its exit code and the terminal
    /// settings with `stty -a`, so that tests can check they were restored.
    struct Session {
        child: Child,
        input: ChildStdin,
        output: PathBuf,
        dir: TempDir,
    }

    /// How a session is started, beyond the stand-in scripts.
    struct Setup<'a> {
        /// The only programs on `PATH` besides the stand-ins; `None` keeps the whole `PATH`.
        only: Option<&'a [&'a str]>,
        /// Shell redirections for `mgitui`, such as `> /dev/full`.
        redirect: &'a str,
    }

    impl Session {
        fn start(dir: TempDir, stand_ins: &[(&str, &str)]) -> Self {
            let setup: Setup<'_> = Setup {
                only: None,
                redirect: "",
            };
            Session::start_with(dir, stand_ins, &setup)
        }

        fn start_with(dir: TempDir, stand_ins: &[(&str, &str)], setup: &Setup<'_>) -> Self {
            let bin: PathBuf = dir.path().join(".git").join("stand-ins");
            std::fs::create_dir_all(&bin).unwrap();
            for (name, body) in stand_ins {
                write_script(&bin, name, body);
            }
            let path: String = match setup.only {
                Some(tools) => {
                    link_tools(&bin, tools);
                    bin.display().to_string()
                }
                None => format!(
                    "{}:{}",
                    bin.display(),
                    std::env::var("PATH").unwrap_or_default()
                ),
            };
            let output: PathBuf = dir.path().join(".git").join("typescript");
            let command: String = format!(
                "tty > .git/tty; stty rows 24 cols 80; {} {}; code=$?; echo; echo exit=$code; \
                 stty -a",
                env!("CARGO_BIN_EXE_mgitui"),
                setup.redirect
            );
            let mut child: Child = Command::new(real("script"))
                .args(["--quiet", "--flush", "--return", "--command", &command])
                .arg(&output)
                .current_dir(dir.path())
                .envs(ISOLATED)
                .env("PATH", path)
                // `script` runs the command with `$SHELL`, which may be fish or another shell
                // that does not understand the POSIX command above.
                .env("SHELL", "/bin/sh")
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap();
            let input: ChildStdin = child.stdin.take().unwrap();
            Session {
                child,
                input,
                output,
                dir,
            }
        }

        /// Everything written to the terminal so far, with escape sequences.
        fn raw(&self) -> String {
            String::from_utf8_lossy(&std::fs::read(&self.output).unwrap_or_default()).into_owned()
        }

        /// Everything written to the terminal so far, without escape sequences.
        fn text(&self) -> String {
            strip_escapes(&self.raw())
        }

        /// The text of the last frame drawn.
        fn frame(&self) -> String {
            let text: String = self.text();
            let start: usize = text.rfind(FRAME_START).unwrap_or(0);
            text[start..].to_string()
        }

        /// Waits until the terminal has shown `expected` after the first `seen` characters of
        /// [`Session::text`], and returns the length of the text then.
        fn wait_for(&self, expected: &str, seen: usize) -> usize {
            let start: Instant = Instant::now();
            loop {
                let text: String = self.text();
                if text
                    .get(seen..)
                    .is_some_and(|new: &str| new.contains(expected))
                {
                    return text.len();
                }
                assert!(
                    start.elapsed() < TIMEOUT,
                    "{expected:?} not shown after {seen} characters:\n{text}"
                );
                std::thread::sleep(Duration::from_millis(20));
            }
        }

        fn press(&mut self, keys: &[u8]) {
            self.input.write_all(keys).unwrap();
            self.input.flush().unwrap();
        }

        /// Sets the size of the pseudo-terminal, as resizing a terminal window does.
        fn resize(&self, rows: usize, columns: usize) {
            let device: String =
                std::fs::read_to_string(self.dir.path().join(".git").join("tty")).unwrap();
            let status: ExitStatus = Command::new("stty")
                .args(["-F", device.trim(), "rows"])
                .arg(rows.to_string())
                .arg("cols")
                .arg(columns.to_string())
                .status()
                .unwrap();
            assert!(status.success());
        }

        /// A file that a stand-in script wrote next to itself.
        fn written_by_stand_in(&self, file: &str) -> String {
            let path: PathBuf = self.dir.path().join(".git").join("stand-ins").join(file);
            std::fs::read_to_string(path).unwrap()
        }

        /// Waits until `script` exits.
        fn finish(mut self) -> Finished {
            let start: Instant = Instant::now();
            while self.child.try_wait().unwrap().is_none() {
                assert!(start.elapsed() < TIMEOUT, "mgitui did not exit");
                std::thread::sleep(Duration::from_millis(20));
            }
            let raw: String = self.raw();
            let text: String = strip_escapes(&raw);
            let (_, after): (&str, &str) = text.rsplit_once("exit=").unwrap();
            let (code, settings): (&str, &str) = after.split_once('\n').unwrap();
            Finished {
                code: code.trim().to_string(),
                settings: settings.to_string(),
                raw,
            }
        }
    }

    /// `text` without ANSI escape sequences, which start with ESC and end with a letter.
    fn strip_escapes(text: &str) -> String {
        let mut out: String = String::new();
        let mut in_escape: bool = false;
        for c in text.chars() {
            if c == '\x1b' {
                in_escape = true;
            } else if in_escape {
                in_escape = !c.is_ascii_alphabetic();
            } else {
                out.push(c);
            }
        }
        out
    }

    /// Checks that the terminal was handed over once, with `shown` written by the program it was
    /// handed to: on a cleared screen, and without leaving the alternate screen before the last
    /// frame, so that the program's output never reaches the screen the shell uses.
    fn assert_handed_over_on_a_blank_alternate_screen(finished: &Finished, shown: &str) {
        let raw: &str = &finished.raw;
        let cleared: usize = raw.find("\x1b[H\x1b[2J").unwrap();
        assert_eq!(raw.matches("\x1b[H\x1b[2J").count(), 1, "{raw}");
        assert!(cleared < raw.find(shown).unwrap(), "{raw}");
        let last_frame: usize = raw.rfind(FRAME_START).unwrap();
        assert!(cleared < last_frame, "{raw}");
        assert!(!raw[..last_frame].contains("\x1b[?1049l"), "{raw}");
    }

    /// Whether `stty -a` output shows a terminal in normal mode: line editing and echo on.
    fn is_normal_mode(settings: &str) -> bool {
        settings.contains(" icanon ") && settings.contains(" echo ")
    }

    /// The path of the real `tool`, for a stand-in that passes calls on to it.
    fn real(tool: &str) -> String {
        let output: Output = Command::new("sh")
            .args(["-c", &format!("command -v {tool}")])
            .output()
            .unwrap();
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    }

    /// Links the real `tools` into `dir`, for a `PATH` that has only them.
    fn link_tools(dir: &Path, tools: &[&str]) {
        for tool in tools {
            std::os::unix::fs::symlink(real(tool), dir.join(tool)).unwrap();
        }
    }

    /// A stand-in `stty` that fails, exiting with 1, when called with `argument` and `condition`
    /// holds, and passes every other call on to the real `stty`.
    fn failing_stty(argument: &str, condition: &str) -> String {
        format!(
            "if [ \"$1\" = {argument} ] && {condition}; then exit 1; fi\nexec {} \"$@\"",
            real("stty")
        )
    }

    #[test]
    fn starts_in_raw_mode_and_switches_views() {
        let mut session: Session = Session::start(repository(), &[]);
        let mut shown: usize = session.wait_for("first commit", 0);
        assert!(session.text().contains(FRAME_START));
        // In raw mode a key arrives without Enter and is not echoed: F2 switches the view at
        // once, and the `~` typed before it never shows.
        session.press(b"~\x1bOQ");
        shown = session.wait_for("No changes", shown);
        assert!(!session.text().contains('~'));
        session.press(b"\x1bOR");
        shown = session.wait_for("─ Log ─", shown);
        session.press(b"\x1bOP");
        session.wait_for("first commit", shown);
        session.press(b"qy");
        assert_eq!(session.finish().code, "0");
    }

    #[test]
    fn redraws_when_the_terminal_is_resized() {
        let mut session: Session = Session::start(repository(), &[]);
        let mut shown: usize = session.wait_for("first commit", 0);
        session.resize(20, 70);
        shown = session.wait_for("terminal too small (needs 80x24)", shown);
        session.resize(24, 80);
        session.wait_for("first commit", shown);
        session.press(b"qy");
        assert_eq!(session.finish().code, "0");
    }

    #[test]
    fn vim_edits_the_message_and_leaving_it_keeps_the_message() {
        // The first run appends a line, as typing and `:wq` does; the second leaves the file
        // unchanged, as `:q!` does. Each run saves the terminal settings it ran with.
        let vim: &str = "stty -a > \"$0.settings\"\n\
            if [ -f \"$0.done\" ]; then exit 0; fi\n\
            touch \"$0.done\"\n\
            printf 'From vim\\n' >> \"$1\"";
        let mut session: Session = Session::start(repository(), &[("vim", vim)]);
        let mut shown: usize = session.wait_for("first commit", 0);
        session.press(b"\x1bOQ");
        shown = session.wait_for("Commit message", shown);
        session.press(b"e");
        shown = session.wait_for("From vim", shown);
        assert!(is_normal_mode(&session.written_by_stand_in("vim.settings")));
        session.press(b"e");
        shown = session.wait_for("Commit message", shown);
        assert_eq!(session.frame().matches("From vim").count(), 1);
        // Back from Vim the terminal is in raw mode again: the `~` is not echoed.
        session.press(b"~\x1bOP");
        session.wait_for("first commit", shown);
        assert!(!session.frame().contains('~'));
        session.press(b"qy");
        assert_eq!(session.finish().code, "0");
    }

    #[test]
    fn vim_exiting_with_an_error_keeps_the_message() {
        // As `:cq` after typing: the edit is thrown away.
        let vim: &str = "printf 'From vim\\n' >> \"$1\"\necho from-vim\nexit 1";
        let mut session: Session = Session::start(repository(), &[("vim", vim)]);
        let mut shown: usize = session.wait_for("first commit", 0);
        session.press(b"\x1bOQ");
        shown = session.wait_for("Commit message", shown);
        session.press(b"e");
        session.wait_for("Editing the message was cancelled", shown);
        assert!(!session.frame().contains("From vim"));
        session.press(b"qy");
        let finished: Finished = session.finish();
        assert_eq!(finished.code, "0");
        assert_handed_over_on_a_blank_alternate_screen(&finished, "from-vim");
    }

    #[test]
    fn vim_ended_by_a_signal_is_shown() {
        let vim: &str = "kill -9 $$";
        let mut session: Session = Session::start(repository(), &[("vim", vim)]);
        let mut shown: usize = session.wait_for("first commit", 0);
        session.press(b"\x1bOQ");
        shown = session.wait_for("Commit message", shown);
        session.press(b"e");
        session.wait_for("Editing the message failed, see the Log view", shown);
        session.press(b"qy");
        assert_eq!(session.finish().code, "0");
    }

    #[test]
    fn interactive_rebase_hands_the_terminal_to_git_and_vim() {
        let dir: TempDir = repository();
        git(dir.path(), &["switch", "-q", "-c", "topic"]);
        git(
            dir.path(),
            &["commit", "-q", "--allow-empty", "-m", "topic work"],
        );
        git(dir.path(), &["switch", "-q", "main"]);
        git(
            dir.path(),
            &["commit", "-q", "--allow-empty", "-m", "main work"],
        );
        // git runs `vim` on the todo list; leaving it unchanged rebases every commit.
        let vim: &str = "stty -a > \"$0.settings\"\necho from-vim";
        let mut session: Session = Session::start(dir, &[("vim", vim)]);
        let mut shown: usize = session.wait_for("topic work", 0);
        session.press(b"/topic work\ri");
        shown = session.wait_for("Interactively rebase topic onto (1 main):", shown);
        session.press(b"1\r");
        session.wait_for("Interactively rebased topic onto main", shown);
        assert!(is_normal_mode(&session.written_by_stand_in("vim.settings")));
        assert_eq!(
            git(session.dir.path(), &["log", "--format=%s", "topic"]),
            "topic work\nmain work\nfirst commit\n"
        );
        session.press(b"qy");
        let finished: Finished = session.finish();
        assert_eq!(finished.code, "0");
        assert_handed_over_on_a_blank_alternate_screen(&finished, "from-vim");
    }

    #[test]
    fn emptying_the_todo_list_cancels_the_interactive_rebase() {
        let dir: TempDir = repository();
        git(dir.path(), &["switch", "-q", "-c", "topic"]);
        git(
            dir.path(),
            &["commit", "-q", "--allow-empty", "-m", "topic work"],
        );
        git(dir.path(), &["switch", "-q", "main"]);
        git(
            dir.path(),
            &["commit", "-q", "--allow-empty", "-m", "main work"],
        );
        let before: String = git(dir.path(), &["rev-parse", "topic"]);
        let vim: &str = ": > \"$1\"";
        let mut session: Session = Session::start(dir, &[("vim", vim)]);
        let mut shown: usize = session.wait_for("topic work", 0);
        session.press(b"/topic work\ri");
        shown = session.wait_for("Interactively rebase topic onto (1 main):", shown);
        session.press(b"1\r");
        shown = session.wait_for("The interactive rebase was cancelled", shown);
        assert_eq!(git(session.dir.path(), &["rev-parse", "topic"]), before);
        // Git's reason shows on the terminal while it runs, and in the log.
        assert!(session.raw().contains("error: nothing to do"));
        session.press(b"\x1bOR");
        session.wait_for("error: nothing to do", shown);
        session.press(b"qy");
        assert_eq!(session.finish().code, "0");
    }

    #[test]
    fn quitting_restores_the_terminal() {
        let mut session: Session = Session::start(repository(), &[]);
        let shown: usize = session.wait_for("first commit", 0);
        session.press(b"q");
        session.wait_for("Quit? y/n", shown);
        session.press(b"y");
        let finished: Finished = session.finish();
        assert_eq!(finished.code, "0");
        assert!(is_normal_mode(&finished.settings), "{}", finished.settings);
        // The last escape sequences show the cursor and leave the alternate screen.
        let quit: usize = finished.raw.rfind("Quit? y/n").unwrap();
        assert!(finished.raw[quit..].contains("\x1b[?25h"));
        assert!(finished.raw[quit..].contains("\x1b[?1049l"));
    }

    #[test]
    fn a_failing_terminal_size_ends_the_app_and_restores_the_terminal() {
        let stty: String = format!(
            "if [ \"$1\" = size ]; then echo 'not a size'; exit 0; fi\nexec {} \"$@\"",
            real("stty")
        );
        let session: Session = Session::start(repository(), &[("stty", &stty)]);
        session.wait_for("mgitui: unexpected stty size output", 0);
        let finished: Finished = session.finish();
        assert_eq!(finished.code, "1");
        assert!(is_normal_mode(&finished.settings), "{}", finished.settings);
    }

    #[test]
    fn outside_a_repository_it_says_so() {
        let dir: TempDir = TempDir::new();
        let output: Output = Command::new(env!("CARGO_BIN_EXE_mgitui"))
            .current_dir(dir.path())
            .envs(ISOLATED)
            .env("GIT_CEILING_DIRECTORIES", std::env::temp_dir())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        let stderr: String = String::from_utf8(output.stderr).unwrap();
        assert!(
            stderr.starts_with(&format!(
                "mgitui: {} is not inside a git repository",
                dir.path().display()
            )),
            "{stderr}"
        );
    }

    #[test]
    fn without_a_terminal_it_says_so() {
        let dir: TempDir = repository();
        let output: Output = Command::new(env!("CARGO_BIN_EXE_mgitui"))
            .current_dir(dir.path())
            .envs(ISOLATED)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        let stderr: String = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains("mgitui: stty -g failed"), "{stderr}");
    }

    #[test]
    fn a_failing_raw_mode_ends_the_app() {
        let stty: String = failing_stty("raw", "true");
        let session: Session = Session::start(repository(), &[("stty", &stty)]);
        session.wait_for("mgitui: stty raw -echo min 0 time 1 failed", 0);
        let finished: Finished = session.finish();
        assert_eq!(finished.code, "1");
        assert!(is_normal_mode(&finished.settings), "{}", finished.settings);
    }

    #[test]
    fn a_failing_size_query_ends_the_app_and_restores_the_terminal() {
        let stty: String = failing_stty("size", "true");
        let session: Session = Session::start(repository(), &[("stty", &stty)]);
        session.wait_for("mgitui: stty size failed", 0);
        let finished: Finished = session.finish();
        assert_eq!(finished.code, "1");
        assert!(is_normal_mode(&finished.settings), "{}", finished.settings);
    }

    #[test]
    fn taking_the_terminal_back_from_vim_can_fail() {
        // Raw mode fails once Vim has run, so the app cannot take the terminal back. It shows
        // the failure in the terminal as it is, with line editing on, and quits on Enter.
        let vim: &str = "touch \"$0.ran\"";
        let stty: String = failing_stty("raw", "[ -f \"$(dirname \"$0\")/vim.ran\" ]");
        let mut session: Session = Session::start(repository(), &[("vim", vim), ("stty", &stty)]);
        let mut shown: usize = session.wait_for("first commit", 0);
        session.press(b"\x1bOQ");
        shown = session.wait_for("Commit message", shown);
        session.press(b"e");
        session.wait_for("Editing the message failed, see the Log view", shown);
        session.press(b"qy\n");
        assert_eq!(session.finish().code, "0");
    }

    #[test]
    fn a_missing_vim_is_shown() {
        let setup: Setup<'_> = Setup {
            only: Some(&["git", "stty", "tty"]),
            redirect: "",
        };
        let mut session: Session = Session::start_with(repository(), &[], &setup);
        let mut shown: usize = session.wait_for("first commit", 0);
        session.press(b"\x1bOQ");
        shown = session.wait_for("Commit message", shown);
        session.press(b"e");
        session.wait_for("Editing the message failed, see the Log view", shown);
        session.press(b"qy");
        assert_eq!(session.finish().code, "0");
    }

    #[test]
    fn a_failing_terminal_output_ends_the_app() {
        // Every write to /dev/full fails, so the first write to the terminal fails.
        let setup: Setup<'_> = Setup {
            only: None,
            redirect: "> /dev/full",
        };
        let session: Session = Session::start_with(repository(), &[], &setup);
        session.wait_for("mgitui: No space left on device", 0);
        assert_eq!(session.finish().code, "1");
    }

    #[test]
    fn a_missing_stty_is_shown() {
        let dir: TempDir = repository();
        let bin: PathBuf = dir.path().join(".git").join("only-git");
        std::fs::create_dir_all(&bin).unwrap();
        link_tools(&bin, &["git"]);
        let output: Output = Command::new(env!("CARGO_BIN_EXE_mgitui"))
            .current_dir(dir.path())
            .envs(ISOLATED)
            .env("PATH", &bin)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        let stderr: String = String::from_utf8(output.stderr).unwrap();
        assert!(
            stderr.starts_with("mgitui: No such file or directory"),
            "{stderr}"
        );
    }
}
