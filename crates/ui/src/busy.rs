//! The text shown while the application is busy before its user interface is shown.

use crate::buffer::Buffer;
use crate::layout::Size;
use crate::style::Style;

/// What the application is busy with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Busy {
    /// Running `git gc` at startup.
    RunningGc,
}

/// The text for `busy`.
#[must_use]
pub fn busy_text(busy: Busy) -> &'static str {
    match busy {
        Busy::RunningGc => "Running gc",
    }
}

/// Draws the text for `busy` in the top left corner. The buffer is only as large as the text,
/// so that it needs no terminal size; the first frame of the user interface covers it.
#[must_use]
pub fn render_busy(busy: Busy) -> Buffer {
    let text: &str = busy_text(busy);
    let width: usize = text.chars().count();
    let mut buffer: Buffer = Buffer::new(Size { width, height: 1 });
    buffer.put_str(0, 0, text, Style::PLAIN, width);
    buffer
}

#[cfg(test)]
mod tests {
    use super::{Busy, busy_text, render_busy};
    use crate::buffer::Buffer;
    use crate::layout::Size;

    #[test]
    fn busy_texts() {
        assert_eq!(busy_text(Busy::RunningGc), "Running gc");
    }

    #[test]
    fn renders_only_the_text() {
        let buffer: Buffer = render_busy(Busy::RunningGc);
        assert_eq!(
            buffer.size(),
            Size {
                width: 10,
                height: 1
            }
        );
        assert_eq!(buffer.row_text(0), "Running gc");
    }
}
