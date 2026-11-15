//! The real terminal: raw mode, size, reading input, drawing and handing the terminal to Vim.
//! It needs a real terminal, so it is excluded from coverage and checked by hand.

use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use crate::size::{Size, parse_stty_size};

/// Switches to the alternate screen and hides the cursor.
const ENTER: &str = "\x1b[?1049h\x1b[?25l";
/// Resets styles, shows the cursor and leaves the alternate screen.
const LEAVE: &str = "\x1b[0m\x1b[?25h\x1b[?1049l";
/// Raw mode where reads return after at most 0.1 seconds.
const RAW: [&str; 6] = ["raw", "-echo", "min", "0", "time", "1"];

/// The terminal in raw mode. Restored by [`Terminal::leave`].
pub struct Terminal {
    saved: String,
}

// Reading, drawing and the size take `&self` although they do not use it: they are only
// meaningful while the terminal is in raw mode, which a `Terminal` value stands for.
impl Terminal {
    /// Saves the terminal settings, switches to raw mode and to the alternate screen.
    ///
    /// # Errors
    ///
    /// Fails when `stty` fails, for example when standard input is not a terminal.
    pub fn enter() -> io::Result<Terminal> {
        let saved: String = stty(&["-g"])?.trim().to_string();
        resume()?;
        Ok(Terminal { saved })
    }

    /// The saved terminal settings, for restoring from a panic hook or for running Vim.
    #[must_use]
    pub fn saved(&self) -> &str {
        &self.saved
    }

    /// Restores the settings saved by [`Terminal::enter`].
    pub fn leave(&self) {
        restore(&self.saved);
    }

    /// The terminal size.
    ///
    /// # Errors
    ///
    /// Fails when `stty` fails or prints something unexpected.
    pub fn size(&self) -> io::Result<Size> {
        let output: String = stty(&["size"])?;
        parse_stty_size(&output)
            .ok_or_else(|| io::Error::other(format!("unexpected stty size output {output:?}")))
    }

    /// Reads available input. Returns 0 when nothing was typed within 0.1 seconds.
    ///
    /// # Errors
    ///
    /// Fails when standard input cannot be read.
    pub fn read(&self, buffer: &mut [u8]) -> io::Result<usize> {
        io::stdin().lock().read(buffer)
    }

    /// Writes a frame to the terminal.
    ///
    /// # Errors
    ///
    /// Fails when standard output cannot be written.
    pub fn draw(&self, frame: &str) -> io::Result<()> {
        write_out(frame)
    }
}

/// Restores terminal settings and leaves the alternate screen. Errors are ignored: this runs
/// while exiting or panicking.
pub fn restore(saved: &str) {
    let _ = write_out(LEAVE);
    let _ = stty(&[saved]);
}

/// Switches back to raw mode and the alternate screen.
///
/// # Errors
///
/// Fails when `stty` fails or standard output cannot be written.
pub fn resume() -> io::Result<()> {
    stty(&RAW)?;
    write_out(ENTER)
}

/// Hands the terminal to `run`, such as git running Vim, and takes it back afterwards.
///
/// # Errors
///
/// Fails when the terminal cannot be taken back.
pub fn hand_over(saved: &str, run: &mut dyn FnMut()) -> io::Result<()> {
    restore(saved);
    run();
    resume()
}

/// Hands the terminal to Vim for editing `path` and takes it back when Vim exits.
///
/// # Errors
///
/// Fails when Vim cannot be started or exits with an error, or the terminal cannot be taken
/// back.
pub fn run_vim(saved: &str, path: &Path) -> io::Result<()> {
    restore(saved);
    let status: io::Result<std::process::ExitStatus> = Command::new("vim").arg(path).status();
    resume()?;
    let status: std::process::ExitStatus = status?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!("vim exited with {status}")))
    }
}

fn write_out(text: &str) -> io::Result<()> {
    let mut out: io::StdoutLock<'_> = io::stdout().lock();
    out.write_all(text.as_bytes())?;
    out.flush()
}

fn stty(args: &[&str]) -> io::Result<String> {
    let output: std::process::Output = Command::new("stty")
        .args(args)
        .stdin(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(io::Error::other(format!("stty {} failed", args.join(" "))))
    }
}
