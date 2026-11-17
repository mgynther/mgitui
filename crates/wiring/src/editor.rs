//! The functions that use the terminal, given to the session so that tests can replace them.

use std::io;
use std::path::Path;

/// Runs an editor on a file and returns how it exited.
pub type Editor = dyn Fn(&Path) -> io::Result<terminal::edit::EditorExit>;

/// Hands the terminal to a program run by the given function, such as git running Vim, and
/// takes it back afterwards.
pub type Handoff<'a> = dyn Fn(&mut dyn FnMut()) -> io::Result<()> + 'a;

/// Writes a frame to the terminal.
pub type Draw<'a> = dyn Fn(&str) -> io::Result<()> + 'a;
