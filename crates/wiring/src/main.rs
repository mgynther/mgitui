//! The mgitui binary. Terminal-only code, excluded from coverage.

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use wiring::log::{LogBuffer, system_clock};

fn main() -> ExitCode {
    let log: LogBuffer = LogBuffer::new(system_clock);
    let current: PathBuf = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    match wiring::startup::open_repository(&git::Git::new(&current), &log) {
        Ok(_git) => ExitCode::SUCCESS,
        Err(message) => {
            let _ = writeln!(std::io::stderr(), "{message}");
            ExitCode::FAILURE
        }
    }
}
