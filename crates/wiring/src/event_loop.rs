//! The main loop: draws the session, reads input and redraws, until the user quits.

use std::io;

use crate::convert;
use crate::session::Session;

/// What the main loop needs from the terminal.
pub trait Device {
    /// The terminal size.
    ///
    /// # Errors
    ///
    /// Fails when the size cannot be read.
    fn size(&self) -> io::Result<ui::layout::Size>;

    /// Reads available input into `buffer` and returns how many bytes were read: 0 when
    /// nothing was typed for a while.
    ///
    /// # Errors
    ///
    /// Fails when the input cannot be read.
    fn read(&self, buffer: &mut [u8]) -> io::Result<usize>;

    /// Writes a frame.
    ///
    /// # Errors
    ///
    /// Fails when the frame cannot be written.
    fn draw(&self, frame: &str) -> io::Result<()>;
}

impl Device for terminal::device::Terminal {
    fn size(&self) -> io::Result<ui::layout::Size> {
        terminal::device::Terminal::size(self).map(convert::ui_size)
    }

    fn read(&self, buffer: &mut [u8]) -> io::Result<usize> {
        terminal::device::Terminal::read(self, buffer)
    }

    fn draw(&self, frame: &str) -> io::Result<()> {
        terminal::device::Terminal::draw(self, frame)
    }
}

/// Runs `session` on `device` until the user quits. A frame is drawn at the start, when the
/// size changes and after input; when nothing happens, nothing is drawn.
///
/// # Errors
///
/// Fails when the device fails.
pub fn run(device: &dyn Device, session: &mut Session) -> io::Result<()> {
    let mut buffer: [u8; 1024] = [0; 1024];
    let mut drawn: Option<ui::layout::Size> = None;
    let mut dirty: bool = true;
    while !session.should_quit() {
        let size: ui::layout::Size = device.size()?;
        if drawn != Some(size) {
            session.resize(size);
            dirty = true;
        }
        if dirty {
            device.draw(&session.render(size).to_ansi())?;
            drawn = Some(size);
            dirty = false;
        }
        let count: usize = device.read(&mut buffer)?;
        if let Some(bytes) = buffer.get(..count).filter(|b: &&[u8]| !b.is_empty()) {
            session.input(bytes);
            dirty = true;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
