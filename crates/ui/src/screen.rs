//! The screen model given to the ui layer, and drawing it.

use crate::buffer::Buffer;
use crate::event::ViewId;
use crate::layout::{self, MIN_SIZE, Rect, Size};
use crate::style::{Color, Style};

/// Everything drawn on the screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Screen {
    /// The tab row.
    pub tabs: Tabs,
    /// The status and prompt line.
    pub status: Status,
    /// The active view.
    pub body: Body,
}

/// The tab row at the top.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tabs {
    /// The active view.
    pub active: ViewId,
    /// Whether there are staged or unstaged changes, which shows the Diff tab in red.
    pub diff_has_changes: bool,
    /// Whether there are unseen warnings or errors, which shows the Log tab in red.
    pub log_has_problems: bool,
}

/// The status and prompt line at the bottom.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// Nothing to show.
    Empty,
    /// A short note.
    Note(String),
    /// A short note about a problem, shown in red.
    Problem(String),
    /// A prompt with the text typed so far.
    Prompt {
        /// The prompt label, for example `/` or `Branch name: `.
        label: String,
        /// The typed text.
        text: String,
    },
}

/// The content of the active view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Body {
    /// The Branches view.
    Branches,
    /// The Diff view.
    Diff,
    /// The Log view.
    Log,
}

/// Draws `screen` into a buffer of `size`.
#[must_use]
pub fn render(screen: &Screen, size: Size) -> Buffer {
    let mut buffer: Buffer = Buffer::new(size);
    if size.is_too_small() {
        let message: String = format!(
            "terminal too small (needs {}x{})",
            MIN_SIZE.width, MIN_SIZE.height
        );
        buffer.put_str(0, 0, &message, Style::PLAIN, size.width);
        return buffer;
    }
    draw_tabs(&mut buffer, screen.tabs);
    draw_status(&mut buffer, &screen.status, size);
    match screen.body {
        Body::Branches => {
            let panes: layout::BranchesLayout = layout::branches_layout(size);
            draw_title(&mut buffer, panes.graph, "Commits", true);
            draw_title(&mut buffer, panes.info, "Commit", false);
            draw_title(&mut buffer, panes.file_diff, "Diff", false);
            draw_title(&mut buffer, panes.files, "Files", false);
            draw_separator(&mut buffer, panes.files);
        }
        Body::Diff => {
            let panes: layout::DiffLayout = layout::diff_layout(size);
            draw_title(&mut buffer, panes.unstaged, "Unstaged", true);
            draw_title(&mut buffer, panes.staged, "Staged", false);
            draw_title(&mut buffer, panes.file_diff, "Diff", false);
            draw_title(&mut buffer, panes.message, "Commit message", false);
            draw_separator(&mut buffer, panes.file_diff);
            draw_separator(&mut buffer, panes.message);
        }
        Body::Log => {
            draw_title(&mut buffer, layout::log_layout(size).list, "Log", true);
        }
    }
    buffer
}

fn draw_tabs(buffer: &mut Buffer, tabs: Tabs) {
    let entries: [(ViewId, &str, bool); 3] = [
        (ViewId::Branches, " F1 Branches ", false),
        (ViewId::Diff, " F2 Diff ", tabs.diff_has_changes),
        (ViewId::Log, " F3 Log ", tabs.log_has_problems),
    ];
    let mut x: usize = 0;
    for (view, label, red) in entries {
        let color: Color = if red { Color::Red } else { Color::Default };
        let style: Style = Style::fg(color).reversed_if(view == tabs.active);
        x += buffer.put_str(x, 0, label, style, label.len());
        x += 1;
    }
}

fn draw_status(buffer: &mut Buffer, status: &Status, size: Size) {
    let y: usize = size.height - 1;
    match status {
        Status::Empty => 0,
        Status::Note(text) => buffer.put_str(0, y, text, Style::PLAIN, size.width),
        Status::Problem(text) => buffer.put_str(0, y, text, Style::fg(Color::Red), size.width),
        Status::Prompt { label, text } => {
            let used: usize = buffer.put_str(0, y, label, Style::PLAIN.bold(), size.width);
            let typed: usize = buffer.put_str(used, y, text, Style::PLAIN, size.width - used);
            buffer.put_str(used + typed, y, " ", Style::PLAIN.reversed_if(true), 1)
        }
    };
}

/// Draws a pane's title row: the title on a line, reversed when the pane has focus.
pub fn draw_title(buffer: &mut Buffer, rect: Rect, title: &str, focused: bool) {
    if rect.height == 0 {
        return;
    }
    for x in rect.x..rect.x + rect.width {
        buffer.set(x, rect.y, '─', Style::fg(Color::Gray));
    }
    let label: String = format!(" {title} ");
    let style: Style = Style::PLAIN.bold().reversed_if(focused);
    buffer.put_str(
        rect.x + 1,
        rect.y,
        &label,
        style,
        rect.width.saturating_sub(1),
    );
}

/// Draws a vertical line in the column left of `rect`.
fn draw_separator(buffer: &mut Buffer, rect: Rect) {
    if let Some(x) = rect.x.checked_sub(1) {
        for y in rect.y..rect.y + rect.height {
            buffer.set(x, y, '│', Style::fg(Color::Gray));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Body, Screen, Status, Tabs, draw_separator, draw_title, render};
    use crate::buffer::Buffer;
    use crate::event::ViewId;
    use crate::layout::{Rect, Size};
    use crate::style::{Color, Style};

    const SIZE: Size = Size {
        width: 80,
        height: 24,
    };

    fn screen(active: ViewId, status: Status, body: Body) -> Screen {
        Screen {
            tabs: Tabs {
                active,
                diff_has_changes: true,
                log_has_problems: false,
            },
            status,
            body,
        }
    }

    #[test]
    fn too_small_shows_message_only() {
        let buffer: Buffer = render(
            &screen(ViewId::Log, Status::Empty, Body::Log),
            Size {
                width: 40,
                height: 5,
            },
        );
        assert_eq!(
            buffer.row_text(0).trim_end(),
            "terminal too small (needs 80x24)"
        );
        assert_eq!(buffer.row_text(4).trim_end(), "");
    }

    #[test]
    fn tabs_show_active_view_and_red_diff() {
        let buffer: Buffer = render(&screen(ViewId::Diff, Status::Empty, Body::Diff), SIZE);
        assert!(
            buffer
                .row_text(0)
                .starts_with(" F1 Branches   F2 Diff   F3 Log ")
        );
        let diff: Style = buffer.cell(15, 0).unwrap().style;
        assert_eq!(diff, Style::fg(Color::Red).reversed_if(true));
        let branches: Style = buffer.cell(1, 0).unwrap().style;
        assert_eq!(branches, Style::PLAIN);
    }

    #[test]
    fn red_log_tab() {
        let mut s: Screen = screen(ViewId::Log, Status::Empty, Body::Log);
        s.tabs.log_has_problems = true;
        let buffer: Buffer = render(&s, SIZE);
        assert_eq!(buffer.cell(25, 0).unwrap().style.fg, Color::Red);
    }

    #[test]
    fn status_line_variants() {
        let note: Buffer = render(
            &screen(ViewId::Log, Status::Note("hello".to_string()), Body::Log),
            SIZE,
        );
        assert_eq!(note.row_text(23).trim_end(), "hello");
        let problem: Buffer = render(
            &screen(ViewId::Log, Status::Problem("bad".to_string()), Body::Log),
            SIZE,
        );
        assert_eq!(problem.cell(0, 23).unwrap().style.fg, Color::Red);
        let prompt: Buffer = render(
            &screen(
                ViewId::Log,
                Status::Prompt {
                    label: "/".to_string(),
                    text: "abc".to_string(),
                },
                Body::Log,
            ),
            SIZE,
        );
        assert_eq!(prompt.row_text(23).trim_end(), "/abc");
        assert!(prompt.cell(4, 23).unwrap().style.reverse);
    }

    #[test]
    fn view_panes_have_titles() {
        let branches: Buffer = render(
            &screen(ViewId::Branches, Status::Empty, Body::Branches),
            SIZE,
        );
        assert!(branches.row_text(1).starts_with("─ Commits ─"));
        assert!(branches.row_text(12).contains(" Files "));
        let diff: Buffer = render(&screen(ViewId::Diff, Status::Empty, Body::Diff), SIZE);
        assert!(diff.row_text(1).starts_with("─ Unstaged ─"));
        assert!(diff.row_text(1).contains("│─ Diff ─"));
        let log: Buffer = render(&screen(ViewId::Log, Status::Empty, Body::Log), SIZE);
        assert!(log.row_text(1).starts_with("─ Log ─"));
    }

    #[test]
    fn empty_panes_draw_nothing() {
        let mut buffer: Buffer = Buffer::new(SIZE);
        draw_title(
            &mut buffer,
            Rect {
                x: 0,
                y: 0,
                width: 5,
                height: 0,
            },
            "x",
            true,
        );
        draw_separator(
            &mut buffer,
            Rect {
                x: 0,
                y: 0,
                width: 5,
                height: 3,
            },
        );
        assert_eq!(buffer, Buffer::new(SIZE));
    }
}
