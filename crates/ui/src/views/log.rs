//! The Log view.

use crate::buffer::Buffer;
use crate::layout::{Rect, Size, log_layout};
use crate::log::LogLevel;
use crate::screen::draw_title;
use crate::style::{Color, Style};
use crate::time::format_utc;

/// The visible part of the log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogView {
    /// The visible lines, top first.
    pub rows: Vec<LogRow>,
}

/// The layer that logged a line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    /// The git layer.
    Git,
    /// The logic layer.
    Logic,
    /// The ui layer.
    Ui,
    /// The validation layer.
    Validation,
    /// The wiring layer.
    Wiring,
}

/// One log line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogRow {
    /// Seconds since 1970-01-01 00:00 UTC.
    pub time: u64,
    /// The layer that logged the line.
    pub layer: Layer,
    /// The severity.
    pub level: LogLevel,
    /// The message.
    pub message: String,
    /// Whether the line is selected.
    pub selected: bool,
}

/// Draws the Log view.
pub fn render(buffer: &mut Buffer, view: &LogView, size: Size) {
    let pane: Rect = log_layout(size).list;
    draw_title(buffer, pane, "Log", true);
    let content: Rect = pane.content();
    for (y, row) in (content.y..content.y + content.height).zip(&view.rows) {
        let style: Style = Style::fg(level_color(row.level)).reversed_if(row.selected);
        buffer.fill(
            Rect {
                y,
                height: 1,
                ..content
            },
            style,
        );
        let text: String = format!(
            "{} {:<10} {:<5} {}",
            format_utc(row.time),
            layer_name(row.layer),
            level_name(row.level),
            without_escape_sequences(&row.message)
        );
        buffer.put_str(content.x, y, &text, style, content.width);
    }
}

/// `text` without the terminal escape sequences that git writes, such as `ESC [ K` that
/// clears the rest of the line: an escape, `[`, parameters, and a final character from `@` to
/// `~`. Other control characters are left for the buffer, which shows them as `?`.
fn without_escape_sequences(text: &str) -> String {
    let mut out: String = String::new();
    let mut chars: std::iter::Peekable<std::str::Chars<'_>> = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' && chars.next_if_eq(&'[').is_some() {
            for in_sequence in chars.by_ref() {
                if ('@'..='~').contains(&in_sequence) {
                    break;
                }
            }
        } else {
            out.push(ch);
        }
    }
    out
}

fn level_color(level: LogLevel) -> Color {
    match level {
        LogLevel::Error => Color::Red,
        LogLevel::Warn => Color::Yellow,
        LogLevel::Info => Color::Default,
        LogLevel::Debug => Color::Gray,
    }
}

fn layer_name(layer: Layer) -> &'static str {
    match layer {
        Layer::Git => "git",
        Layer::Logic => "logic",
        Layer::Ui => "ui",
        Layer::Validation => "validation",
        Layer::Wiring => "wiring",
    }
}

fn level_name(level: LogLevel) -> &'static str {
    match level {
        LogLevel::Error => "Error",
        LogLevel::Warn => "Warn",
        LogLevel::Info => "Info",
        LogLevel::Debug => "Debug",
    }
}

#[cfg(test)]
mod tests {
    use super::{Layer, LogRow, LogView, layer_name, render, without_escape_sequences};
    use crate::buffer::Buffer;
    use crate::layout::Size;
    use crate::log::LogLevel;
    use crate::style::Color;

    const SIZE: Size = Size {
        width: 80,
        height: 24,
    };

    fn row(level: LogLevel, message: &str, selected: bool) -> LogRow {
        LogRow {
            time: 1_735_787_045,
            layer: Layer::Git,
            level,
            message: message.to_string(),
            selected,
        }
    }

    #[test]
    fn escape_sequences_are_left_out() {
        let cases: [(&str, &str); 5] = [
            ("\x1b[KSuccessfully rebased", "Successfully rebased"),
            ("\x1b[1;31mred\x1b[m plain", "red plain"),
            ("lone \x1b escape", "lone \x1b escape"),
            ("unfinished \x1b[12", "unfinished "),
            ("tab\tstays", "tab\tstays"),
        ];
        for (text, shown) in cases {
            assert_eq!(without_escape_sequences(text), shown);
        }
        let view: LogView = LogView {
            rows: vec![row(LogLevel::Info, "\x1b[KDone.", false)],
        };
        let mut buffer: Buffer = Buffer::new(SIZE);
        render(&mut buffer, &view, SIZE);
        assert_eq!(
            buffer.row_text(2).trim_end(),
            "2025-01-02 03:04:05 git        Info  Done."
        );
    }

    #[test]
    fn layer_names() {
        let cases: [(Layer, &str); 5] = [
            (Layer::Git, "git"),
            (Layer::Logic, "logic"),
            (Layer::Ui, "ui"),
            (Layer::Validation, "validation"),
            (Layer::Wiring, "wiring"),
        ];
        for (layer, name) in cases {
            assert_eq!(layer_name(layer), name);
        }
    }

    #[test]
    fn rows_show_time_layer_level_and_color() {
        let view: LogView = LogView {
            rows: vec![
                row(LogLevel::Error, "failed", false),
                row(LogLevel::Warn, "careful", false),
                row(LogLevel::Info, "fine", true),
                row(LogLevel::Debug, "detail", false),
            ],
        };
        let mut buffer: Buffer = Buffer::new(SIZE);
        render(&mut buffer, &view, SIZE);
        assert!(buffer.row_text(1).starts_with("─ Log ─"));
        assert_eq!(
            buffer.row_text(2).trim_end(),
            "2025-01-02 03:04:05 git        Error failed"
        );
        assert_eq!(
            buffer.row_text(3).trim_end(),
            "2025-01-02 03:04:05 git        Warn  careful"
        );
        assert_eq!(buffer.cell(0, 2).unwrap().style.fg, Color::Red);
        assert_eq!(buffer.cell(0, 3).unwrap().style.fg, Color::Yellow);
        assert!(buffer.cell(79, 4).unwrap().style.reverse);
        assert_eq!(buffer.cell(0, 5).unwrap().style.fg, Color::Gray);
        assert!(buffer.row_text(5).contains("Debug detail"));
    }
}
