//! Tests that a panic restores the terminal. A panic hook is global to a program, and the test
//! needs a real terminal, so the test runs this test program again as a child process in a
//! pseudo-terminal from `script` (util-linux), where the child panics on purpose.
#![cfg(target_os = "linux")]

// Clippy allows `unwrap` in test code only inside a `cfg(test)` module.
#[cfg(test)]
mod pty {
    use std::path::PathBuf;
    use std::process::{Command, Output};

    use terminal::device::{Terminal, restore_on_panic};

    /// Set for the child process, which panics instead of starting a child itself.
    const CHILD: &str = "MGITUI_PANIC_TEST_CHILD";

    /// The message the child panics with.
    const MESSAGE: &str = "panicking on purpose";

    #[test]
    fn a_panic_restores_the_terminal() {
        if std::env::var_os(CHILD).is_some() {
            let terminal: Terminal = Terminal::enter().unwrap();
            restore_on_panic(terminal.saved().to_string());
            panic!("{MESSAGE}");
        }
        let exe: PathBuf = std::env::current_exe().unwrap();
        let typescript: PathBuf = std::env::temp_dir().join(format!(
            "mgitui-panic-test-{}.typescript",
            std::process::id()
        ));
        let command: String = format!(
            "{} pty::a_panic_restores_the_terminal --exact --nocapture --test-threads=1; \
             stty -a",
            exe.display()
        );
        let output: Output = Command::new("script")
            .args(["--quiet", "--flush", "--command", &command])
            .arg(&typescript)
            .env(CHILD, "1")
            // `script` runs the command with `$SHELL`, which may be fish or another shell that
            // does not understand the POSIX command above.
            .env("SHELL", "/bin/sh")
            .stdin(std::process::Stdio::null())
            .output()
            .unwrap();
        assert!(output.status.success());
        let raw: String =
            String::from_utf8_lossy(&std::fs::read(&typescript).unwrap()).into_owned();
        let _ = std::fs::remove_file(&typescript);
        // The child entered raw mode and the alternate screen, and the hook left both before
        // the panic message was printed.
        let entered: usize = raw.find("\x1b[?1049h").unwrap();
        let left: usize = raw.rfind("\x1b[?1049l").unwrap();
        let message: usize = raw.find(MESSAGE).unwrap();
        assert!(entered < left && left < message, "{raw}");
        let settings: &str = &raw[message..];
        assert!(
            settings.contains(" icanon ") && settings.contains(" echo "),
            "{raw}"
        );
    }
}
