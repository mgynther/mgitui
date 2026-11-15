//! The mgitui binary: connects the terminal to a session and runs the main loop. The terminal
//! tests in `tests/terminal.rs` run it in a pseudo-terminal.

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use terminal::device::{self, Terminal};
use ui::exit::{ExitProblem, exit_text};
use wiring::log::{LogBuffer, system_clock};
use wiring::session::Session;

fn main() -> ExitCode {
    let log: LogBuffer = LogBuffer::new(system_clock);
    let current: PathBuf = wiring::startup::start_dir(std::env::current_dir());
    let git: git::Git = match wiring::startup::open_repository(&git::Git::new(&current), &log) {
        Ok(git) => git,
        Err(problem) => return fail(&problem),
    };
    match run(git, log) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => fail(&ExitProblem::Terminal {
            detail: error.to_string(),
        }),
    }
}

fn fail(problem: &ExitProblem) -> ExitCode {
    let _ = writeln!(io::stderr(), "{}", exit_text(problem));
    ExitCode::FAILURE
}

fn run(git: git::Git, log: LogBuffer) -> io::Result<()> {
    let terminal: Terminal = Terminal::enter()?;
    let saved: String = terminal.saved().to_string();
    device::restore_on_panic(saved.clone());
    let for_handoff: String = saved.clone();
    let editor = move |path: &Path| device::run_vim(&saved, path);
    let handoff = move |run: &mut dyn FnMut()| device::hand_over(&for_handoff, run);
    let mut session: Session = Session::new(git, log, Box::new(editor), Box::new(handoff));
    let result: io::Result<()> = wiring::event_loop::run(&terminal, &mut session);
    terminal.leave();
    result
}
