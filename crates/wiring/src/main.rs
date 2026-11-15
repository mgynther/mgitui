//! The mgitui binary: connects the terminal to a session and runs the main loop. It needs a
//! real terminal, so it is excluded from coverage and checked by hand.

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use terminal::device::{self, Terminal};
use ui::exit::{ExitProblem, exit_text};
use wiring::log::{LogBuffer, system_clock};
use wiring::session::Session;

fn main() -> ExitCode {
    let log: LogBuffer = LogBuffer::new(system_clock);
    let current: PathBuf = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
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
    let for_panic: String = saved.clone();
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        device::restore(&for_panic);
        default_hook(info);
    }));
    let for_handoff: String = saved.clone();
    let editor = move |path: &Path| device::run_vim(&saved, path);
    let handoff = move |run: &mut dyn FnMut()| device::hand_over(&for_handoff, run);
    let mut session: Session = Session::new(git, log, Box::new(editor), Box::new(handoff));
    let result: io::Result<()> = event_loop(&terminal, &mut session);
    terminal.leave();
    result
}

fn event_loop(terminal: &Terminal, session: &mut Session) -> io::Result<()> {
    let mut buffer: [u8; 1024] = [0; 1024];
    let mut drawn: Option<ui::layout::Size> = None;
    let mut dirty: bool = true;
    while !session.should_quit() {
        let size: ui::layout::Size = wiring::convert::ui_size(terminal.size()?);
        if drawn != Some(size) {
            session.resize(size);
            dirty = true;
        }
        if dirty {
            terminal.draw(&session.render(size).to_ansi())?;
            drawn = Some(size);
            dirty = false;
        }
        let count: usize = terminal.read(&mut buffer)?;
        if let Some(bytes) = buffer.get(..count).filter(|b: &&[u8]| !b.is_empty()) {
            session.input(bytes);
            dirty = true;
        }
    }
    Ok(())
}
