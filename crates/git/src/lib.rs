//! The git layer of mgitui: runs command line git with typed arguments and parses its output.

pub mod log;

mod command;
mod runner;

#[cfg(test)]
mod test_repo;

pub use command::Command;
pub use runner::{Git, GitError, Output};
