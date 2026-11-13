//! The mgitui binary: the terminal and the main loop. Terminal-only code, excluded from
//! coverage.

mod terminal;

use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use terminal::Terminal;
use wiring::log::{LogBuffer, system_clock};
use wiring::session::Session;

fn main() -> ExitCode {
    let log: LogBuffer = LogBuffer::new(system_clock);
    let current: PathBuf = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let git: git::Git = match wiring::startup::open_repository(&git::Git::new(&current), &log) {
        Ok(git) => git,
        Err(message) => return fail(&message),
    };
    let session: Session = Session::new(git, log);
    match run(session) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => fail(&format!("mgitui: {error}")),
    }
}

fn fail(message: &str) -> ExitCode {
    let _ = writeln!(io::stderr(), "{message}");
    ExitCode::FAILURE
}

fn run(mut session: Session) -> io::Result<()> {
    let terminal: Terminal = Terminal::enter()?;
    let saved: String = terminal.saved().to_string();
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        terminal::restore(&saved);
        default_hook(info);
    }));
    let result: io::Result<()> = event_loop(&terminal, &mut session);
    terminal.leave();
    result
}

fn event_loop(terminal: &Terminal, session: &mut Session) -> io::Result<()> {
    let mut buffer: [u8; 1024] = [0; 1024];
    let mut drawn: Option<ui::layout::Size> = None;
    let mut dirty: bool = true;
    while !session.should_quit() {
        let size: ui::layout::Size = terminal.size()?;
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
