//! The Diff view: file lists, the file diff and the commit message.

use crate::buffer::Buffer;
use crate::layout::{DiffLayout, Rect, Size, diff_layout};
use crate::screen::{draw_separator, draw_title};
use crate::style::{Color, Style};
use crate::views::diff_text::{self, DiffRowView};
use crate::views::file_list::{self, FileRow};

/// Which pane of the Diff view has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffFocus {
    /// The unstaged file list.
    Unstaged,
    /// The staged file list.
    Staged,
    /// The file diff.
    FileDiff,
}

/// What the Diff view shows. The lists contain only the visible rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffView {
    /// The focused pane.
    pub focus: DiffFocus,
    /// The visible files with unstaged changes.
    pub unstaged: Vec<FileRow>,
    /// The visible files with staged changes.
    pub staged: Vec<FileRow>,
    /// The title of the file diff pane.
    pub diff_title: String,
    /// The visible rows of the file diff.
    pub diff: Vec<DiffRowView>,
    /// The title of the commit message pane.
    pub message_title: String,
    /// The commit message.
    pub message: String,
    /// Whether there are no changes at all.
    pub no_changes: bool,
}

/// Draws the Diff view.
pub fn render(buffer: &mut Buffer, view: &DiffView, size: Size) {
    let panes: DiffLayout = diff_layout(size);
    draw_title(
        buffer,
        panes.unstaged,
        "Unstaged",
        view.focus == DiffFocus::Unstaged,
    );
    file_list::render(
        buffer,
        panes.unstaged.content(),
        &view.unstaged,
        view.focus == DiffFocus::Unstaged,
    );
    draw_title(
        buffer,
        panes.staged,
        "Staged",
        view.focus == DiffFocus::Staged,
    );
    file_list::render(
        buffer,
        panes.staged.content(),
        &view.staged,
        view.focus == DiffFocus::Staged,
    );
    draw_title(
        buffer,
        panes.file_diff,
        &view.diff_title,
        view.focus == DiffFocus::FileDiff,
    );
    let diff_area: Rect = panes.file_diff.content();
    if view.no_changes {
        buffer.put_str(
            diff_area.x + 1,
            diff_area.y,
            "No changes",
            Style::fg(Color::Gray),
            diff_area.width,
        );
    } else {
        diff_text::render(
            buffer,
            diff_area,
            &view.diff,
            view.focus == DiffFocus::FileDiff,
        );
    }
    draw_title(buffer, panes.message, &view.message_title, false);
    let message_area: Rect = panes.message.content();
    for (y, line) in
        (message_area.y..message_area.y + message_area.height).zip(view.message.lines())
    {
        buffer.put_str(message_area.x, y, line, Style::PLAIN, message_area.width);
    }
    draw_separator(buffer, panes.file_diff);
    draw_separator(buffer, panes.message);
}

#[cfg(test)]
mod tests {
    use super::{DiffFocus, DiffView, render};
    use crate::buffer::Buffer;
    use crate::layout::Size;
    use crate::views::diff_text::{DiffRowKind, DiffRowView};
    use crate::views::file_list::FileRow;

    const SIZE: Size = Size {
        width: 90,
        height: 30,
    };

    fn view(focus: DiffFocus, no_changes: bool) -> DiffView {
        DiffView {
            focus,
            unstaged: vec![FileRow {
                label: "M a.txt".to_string(),
                selected: true,
            }],
            staged: vec![FileRow {
                label: "A b.txt".to_string(),
                selected: true,
            }],
            diff_title: "Diff a.txt".to_string(),
            diff: vec![DiffRowView {
                kind: DiffRowKind::Added,
                text: "line".to_string(),
                cursor: true,
                marked: false,
            }],
            message_title: "Commit message".to_string(),
            message: "Subject\n\nBody".to_string(),
            no_changes,
        }
    }

    #[test]
    fn draws_all_panes() {
        let mut buffer: Buffer = Buffer::new(SIZE);
        render(&mut buffer, &view(DiffFocus::Unstaged, false), SIZE);
        assert!(buffer.row_text(1).starts_with("─ Unstaged ─"));
        assert!(buffer.row_text(1).contains("│─ Diff a.txt ─"));
        assert!(buffer.row_text(2).starts_with("M a.txt"));
        assert!(buffer.row_text(2).contains("│ +line"));
        assert!(buffer.cell(0, 2).unwrap().style.reverse);
        assert!(buffer.row_text(15).starts_with("─ Staged ─"));
        assert!(buffer.row_text(16).starts_with("A b.txt"));
        assert!(buffer.cell(0, 16).unwrap().style.bold);
        assert!(buffer.row_text(21).contains("─ Commit message ─"));
        assert!(buffer.row_text(22).contains("│Subject"));
        assert!(buffer.row_text(24).contains("│Body"));
    }

    #[test]
    fn focus_on_staged_and_diff() {
        let mut buffer: Buffer = Buffer::new(SIZE);
        render(&mut buffer, &view(DiffFocus::Staged, false), SIZE);
        assert!(buffer.cell(0, 16).unwrap().style.reverse);
        render(&mut buffer, &view(DiffFocus::FileDiff, false), SIZE);
        assert!(buffer.row_text(2).contains("│▶+line"));
    }

    #[test]
    fn no_changes_note() {
        let mut buffer: Buffer = Buffer::new(SIZE);
        render(&mut buffer, &view(DiffFocus::Unstaged, true), SIZE);
        assert!(buffer.row_text(2).contains("│ No changes"));
    }
}
