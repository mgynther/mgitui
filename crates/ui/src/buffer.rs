//! A grid of styled characters that is drawn to the terminal in one go.

use std::fmt::Write;

use crate::layout::{Rect, Size};
use crate::style::Style;

/// Tab stops in drawn text are this many columns apart.
const TAB_WIDTH: usize = 4;

/// One character cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    /// The character.
    pub ch: char,
    /// Its style.
    pub style: Style,
}

/// A screen-sized grid of cells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Buffer {
    size: Size,
    cells: Vec<Cell>,
}

impl Buffer {
    /// Creates a buffer filled with spaces.
    #[must_use]
    pub fn new(size: Size) -> Self {
        Buffer {
            size,
            cells: vec![
                Cell {
                    ch: ' ',
                    style: Style::PLAIN
                };
                size.width * size.height
            ],
        }
    }

    /// The size of the buffer.
    #[must_use]
    pub fn size(&self) -> Size {
        self.size
    }

    /// The cell at column `x`, row `y`, if inside the buffer.
    #[must_use]
    pub fn cell(&self, x: usize, y: usize) -> Option<Cell> {
        if x < self.size.width {
            self.cells.get(y * self.size.width + x).copied()
        } else {
            None
        }
    }

    /// Sets the cell at column `x`, row `y`. Cells outside the buffer are ignored.
    pub fn set(&mut self, x: usize, y: usize, ch: char, style: Style) {
        if x < self.size.width
            && let Some(cell) = self.cells.get_mut(y * self.size.width + x)
        {
            *cell = Cell { ch, style };
        }
    }

    /// Draws `text` starting at column `x`, row `y`, using at most `max_width` columns. Tabs are
    /// expanded and other control characters are shown as `?`. Returns the number of columns
    /// used.
    pub fn put_str(
        &mut self,
        x: usize,
        y: usize,
        text: &str,
        style: Style,
        max_width: usize,
    ) -> usize {
        let mut column: usize = 0;
        for ch in text.chars() {
            if column >= max_width {
                break;
            }
            if ch == '\t' {
                let next_stop: usize = (column / TAB_WIDTH + 1) * TAB_WIDTH;
                while column < next_stop && column < max_width {
                    self.set(x + column, y, ' ', style);
                    column += 1;
                }
            } else {
                let shown: char = if ch.is_control() { '?' } else { ch };
                self.set(x + column, y, shown, style);
                column += 1;
            }
        }
        column
    }

    /// Fills a rectangle with spaces in `style`.
    pub fn fill(&mut self, rect: Rect, style: Style) {
        for y in rect.y..rect.y + rect.height {
            for x in rect.x..rect.x + rect.width {
                self.set(x, y, ' ', style);
            }
        }
    }

    /// The characters of row `y`, without styles. Useful in tests.
    #[must_use]
    pub fn row_text(&self, y: usize) -> String {
        (0..self.size.width)
            .filter_map(|x: usize| self.cell(x, y))
            .map(|c: Cell| c.ch)
            .collect()
    }

    /// The escape sequences and text that draw the whole buffer on the terminal.
    #[must_use]
    pub fn to_ansi(&self) -> String {
        let mut out: String = String::new();
        for (y, row) in self.cells.chunks(self.size.width.max(1)).enumerate() {
            let _ = write!(out, "\x1b[{};1H", y + 1);
            let mut current: Option<Style> = None;
            for cell in row {
                if current != Some(cell.style) {
                    out.push_str(&cell.style.sgr());
                    current = Some(cell.style);
                }
                out.push(cell.ch);
            }
        }
        out.push_str("\x1b[0m");
        out
    }
}

#[cfg(test)]
mod tests {
    use super::{Buffer, Cell};
    use crate::layout::{Rect, Size};
    use crate::style::{Color, Style};

    fn small() -> Buffer {
        Buffer::new(Size {
            width: 6,
            height: 2,
        })
    }

    #[test]
    fn new_buffer_is_blank() {
        let buffer: Buffer = small();
        assert_eq!(
            buffer.size(),
            Size {
                width: 6,
                height: 2
            }
        );
        assert_eq!(buffer.row_text(0), "      ");
        assert_eq!(buffer.cell(6, 0), None);
        assert_eq!(buffer.cell(0, 2), None);
    }

    #[test]
    fn put_str_clips_and_sanitizes() {
        let mut buffer: Buffer = small();
        assert_eq!(buffer.put_str(1, 0, "a\u{7}bcdefg", Style::PLAIN, 4), 4);
        assert_eq!(buffer.row_text(0), " a?bc ");
        assert_eq!(buffer.put_str(0, 1, "", Style::PLAIN, 4), 0);
        assert_eq!(buffer.put_str(0, 1, "abc", Style::PLAIN, 0), 0);
    }

    #[test]
    fn put_str_expands_tabs() {
        let mut buffer: Buffer = Buffer::new(Size {
            width: 10,
            height: 1,
        });
        assert_eq!(buffer.put_str(0, 0, "a\tb", Style::PLAIN, 10), 5);
        assert_eq!(buffer.row_text(0), "a   b     ");
        assert_eq!(buffer.put_str(0, 0, "\t\tc", Style::PLAIN, 6), 6);
        assert_eq!(buffer.row_text(0), "          ");
    }

    #[test]
    fn writes_outside_are_ignored() {
        let mut buffer: Buffer = small();
        buffer.set(6, 0, 'x', Style::PLAIN);
        buffer.set(0, 2, 'x', Style::PLAIN);
        buffer.put_str(5, 1, "xyz", Style::PLAIN, 3);
        assert_eq!(buffer.row_text(1), "     x");
    }

    #[test]
    fn fill_sets_style() {
        let mut buffer: Buffer = small();
        let red: Style = Style::fg(Color::Red);
        buffer.fill(
            Rect {
                x: 1,
                y: 1,
                width: 2,
                height: 1,
            },
            red,
        );
        assert_eq!(
            buffer.cell(1, 1),
            Some(Cell {
                ch: ' ',
                style: red
            })
        );
        assert_eq!(buffer.cell(3, 1).map(|c: Cell| c.style), Some(Style::PLAIN));
    }

    #[test]
    fn ansi_output_changes_style_only_when_needed() {
        let mut buffer: Buffer = Buffer::new(Size {
            width: 3,
            height: 1,
        });
        buffer.put_str(1, 0, "ab", Style::fg(Color::Green), 2);
        assert_eq!(buffer.to_ansi(), "\x1b[1;1H\x1b[0;39m \x1b[0;32mab\x1b[0m");
        assert_eq!(
            Buffer::new(Size {
                width: 0,
                height: 3
            })
            .to_ansi(),
            "\x1b[0m"
        );
    }
}
