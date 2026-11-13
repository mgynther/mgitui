//! Drawing the rows of a file diff, used by the Diff and Branches views.

use crate::buffer::Buffer;
use crate::layout::Rect;
use crate::style::{Color, Style};

/// What a diff row is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffRowKind {
    /// A hunk header.
    HunkHeader,
    /// An unchanged line.
    Context,
    /// An added line.
    Added,
    /// A removed line.
    Removed,
    /// `\ No newline at end of file`.
    NoNewline,
    /// A note instead of content.
    Note,
}

/// A row of a file diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffRowView {
    /// What the row is.
    pub kind: DiffRowKind,
    /// The text without a prefix character.
    pub text: String,
    /// Whether the cursor is on this row.
    pub cursor: bool,
    /// Whether the row is part of the line selection.
    pub marked: bool,
}

/// Draws `rows` into `rect`. The first column marks the cursor (when `focused`) and the line
/// selection.
pub fn render(buffer: &mut Buffer, rect: Rect, rows: &[DiffRowView], focused: bool) {
    for (y, row) in (rect.y..rect.y + rect.height).zip(rows) {
        let cursor: bool = row.cursor && focused;
        let style: Style = Style::fg(color(row.kind)).reversed_if(cursor);
        let gutter: char = if cursor {
            '▶'
        } else if row.marked {
            '┃'
        } else {
            ' '
        };
        buffer.set(rect.x, y, gutter, Style::fg(Color::Blue));
        let text: String = format!("{}{}", prefix(row.kind), row.text);
        buffer.put_str(rect.x + 1, y, &text, style, rect.width.saturating_sub(1));
    }
}

fn color(kind: DiffRowKind) -> Color {
    match kind {
        DiffRowKind::HunkHeader => Color::Cyan,
        DiffRowKind::Context => Color::Default,
        DiffRowKind::Added => Color::Green,
        DiffRowKind::Removed => Color::Red,
        DiffRowKind::NoNewline | DiffRowKind::Note => Color::Gray,
    }
}

fn prefix(kind: DiffRowKind) -> &'static str {
    match kind {
        DiffRowKind::HunkHeader | DiffRowKind::Note => "",
        DiffRowKind::Context => " ",
        DiffRowKind::Added => "+",
        DiffRowKind::Removed => "-",
        DiffRowKind::NoNewline => "\\",
    }
}

#[cfg(test)]
mod tests {
    use super::{DiffRowKind, DiffRowView, render};
    use crate::buffer::Buffer;
    use crate::layout::{Rect, Size};
    use crate::style::Color;

    fn row(kind: DiffRowKind, text: &str, cursor: bool, marked: bool) -> DiffRowView {
        DiffRowView {
            kind,
            text: text.to_string(),
            cursor,
            marked,
        }
    }

    #[test]
    fn rows_with_prefixes_colors_and_marks() {
        let rows: Vec<DiffRowView> = vec![
            row(DiffRowKind::HunkHeader, "@@ -1 +1 @@", false, false),
            row(DiffRowKind::Context, "same", false, false),
            row(DiffRowKind::Removed, "old", false, true),
            row(DiffRowKind::Added, "new", true, true),
            row(
                DiffRowKind::NoNewline,
                " No newline at end of file",
                false,
                false,
            ),
            row(DiffRowKind::Note, "Binary file", false, false),
        ];
        let mut buffer: Buffer = Buffer::new(Size {
            width: 30,
            height: 6,
        });
        render(
            &mut buffer,
            Rect {
                x: 0,
                y: 0,
                width: 30,
                height: 6,
            },
            &rows,
            true,
        );
        let lines: Vec<String> = (0..6)
            .map(|y| buffer.row_text(y).trim_end().to_string())
            .collect();
        assert_eq!(
            lines,
            vec![
                " @@ -1 +1 @@",
                "  same",
                "┃-old",
                "▶+new",
                " \\ No newline at end of file",
                " Binary file",
            ]
        );
        assert_eq!(buffer.cell(1, 0).unwrap().style.fg, Color::Cyan);
        assert_eq!(buffer.cell(1, 2).unwrap().style.fg, Color::Red);
        assert!(buffer.cell(1, 3).unwrap().style.reverse);
        assert_eq!(buffer.cell(1, 3).unwrap().style.fg, Color::Green);
        assert_eq!(buffer.cell(1, 5).unwrap().style.fg, Color::Gray);
    }

    #[test]
    fn cursor_is_hidden_without_focus() {
        let rows: Vec<DiffRowView> = vec![row(DiffRowKind::Added, "new", true, false)];
        let mut buffer: Buffer = Buffer::new(Size {
            width: 10,
            height: 1,
        });
        render(
            &mut buffer,
            Rect {
                x: 0,
                y: 0,
                width: 10,
                height: 1,
            },
            &rows,
            false,
        );
        assert_eq!(buffer.row_text(0).trim_end(), " +new");
        assert!(!buffer.cell(1, 0).unwrap().style.reverse);
    }
}
