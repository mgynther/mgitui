//! Drawing a list of files.

use crate::buffer::Buffer;
use crate::layout::Rect;
use crate::style::Style;

/// How a file changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileChange {
    /// Content changed.
    Modified,
    /// The file is new.
    Added,
    /// The file was deleted.
    Deleted,
    /// The file was renamed.
    Renamed,
    /// The file was copied.
    Copied,
    /// The file type changed.
    TypeChanged,
    /// The file is not tracked by git.
    Untracked,
    /// The file has a merge conflict.
    Conflict,
}

/// A file in a list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRow {
    /// How the file changed.
    pub change: FileChange,
    /// The path.
    pub path: String,
    /// The path before a rename or copy.
    pub original_path: Option<String>,
    /// Whether the file mode changed.
    pub mode_changed: bool,
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
        buffer.put_str(rect.x, y, &label(row), style, rect.width);
    }
}

/// A status letter and the path, for example `M src/main.rs` or `R old -> new (mode)`.
fn label(row: &FileRow) -> String {
    let letter: char = match row.change {
        FileChange::Modified => 'M',
        FileChange::Added => 'A',
        FileChange::Deleted => 'D',
        FileChange::Renamed => 'R',
        FileChange::Copied => 'C',
        FileChange::TypeChanged => 'T',
        FileChange::Untracked => '?',
        FileChange::Conflict => 'U',
    };
    let path: String = match &row.original_path {
        Some(original) => format!("{original} -> {}", row.path),
        None => row.path.clone(),
    };
    let mode: &str = if row.mode_changed { " (mode)" } else { "" };
    format!("{letter} {path}{mode}")
}

#[cfg(test)]
mod tests {
    use super::{FileChange, FileRow, label, render};

    fn row(change: FileChange, path: &str, selected: bool) -> FileRow {
        FileRow {
            change,
            path: path.to_string(),
            original_path: None,
            mode_changed: false,
            selected,
        }
    }

    #[test]
    fn labels() {
        let cases: [(FileChange, &str); 8] = [
            (FileChange::Modified, "M p"),
            (FileChange::Added, "A p"),
            (FileChange::Deleted, "D p"),
            (FileChange::Renamed, "R p"),
            (FileChange::Copied, "C p"),
            (FileChange::TypeChanged, "T p"),
            (FileChange::Untracked, "? p"),
            (FileChange::Conflict, "U p"),
        ];
        for (change, text) in cases {
            assert_eq!(label(&row(change, "p", false)), text);
        }
        let renamed: FileRow = FileRow {
            change: FileChange::Renamed,
            path: "p".to_string(),
            original_path: Some("o".to_string()),
            mode_changed: true,
            selected: false,
        };
        assert_eq!(label(&renamed), "R o -> p (mode)");
    }
    use crate::buffer::Buffer;
    use crate::layout::{Rect, Size};

    #[test]
    fn selected_row_style_depends_on_focus() {
        let rows: Vec<FileRow> = vec![
            row(FileChange::Modified, "a", false),
            row(FileChange::Untracked, "b", true),
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
