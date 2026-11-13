//! The git layer of mgitui: runs command line git with typed arguments and parses its output.

pub mod log;

mod command;
mod diff;
mod runner;
mod status;

#[cfg(test)]
mod test_repo;

pub use command::Command;
pub use diff::{DiffLine, FileDiff, Hunk, LineKind, parse_diff};
pub use runner::{Git, GitError, Output};
pub use status::{ChangeKind, FileChange, Status, parse_status};
