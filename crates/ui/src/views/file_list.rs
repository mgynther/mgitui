//! Drawing a list of files.

use crate::buffer::Buffer;
use crate::layout::Rect;
use crate::style::Style;

/// A file in a list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRow {
    /// The text shown, for example `M src/main.rs`.
    pub label: String,
    /// Whether the file is selected.
    pub selected: bool,
}

/// Draws `rows` into `rect`. The selected row is reversed when `focused`, bold otherwise.
pub fn render(buffer: &mut Buffer, rect: Rect, rows: &[FileRow], focused: bool) {
    for (y, row) in (rect.y..rect.y + rect.height).zip(rows) {
        let style: Style = match (row.selected, focused) {
            (true, true) => Style::PLAIN.reversed_if(true),
            (true, false) => Style::PLAIN.bold(),
            (false, _) => Style::PLAIN,
        };
        buffer.fill(
            Rect {
                y,
                height: 1,
                ..rect
            },
            style,
        );
        buffer.put_str(rect.x, y, &row.label, style, rect.width);
    }
}

#[cfg(test)]
mod tests {
    use super::{FileRow, render};
    use crate::buffer::Buffer;
    use crate::layout::{Rect, Size};

    #[test]
    fn selected_row_style_depends_on_focus() {
        let rows: Vec<FileRow> = vec![
            FileRow {
                label: "M a".to_string(),
                selected: false,
            },
            FileRow {
                label: "? b".to_string(),
                selected: true,
            },
        ];
        let rect: Rect = Rect {
            x: 0,
            y: 0,
            width: 5,
            height: 3,
        };
        let mut buffer: Buffer = Buffer::new(Size {
            width: 5,
            height: 3,
        });
        render(&mut buffer, rect, &rows, true);
        assert_eq!(buffer.row_text(0), "M a  ");
        assert!(buffer.cell(4, 1).unwrap().style.reverse);
        render(&mut buffer, rect, &rows, false);
        assert!(buffer.cell(0, 1).unwrap().style.bold);
        assert!(!buffer.cell(0, 0).unwrap().style.bold);
    }
}
