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

/// One log line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogRow {
    /// Seconds since 1970-01-01 00:00 UTC.
    pub time: u64,
    /// The layer that logged the line.
    pub layer: String,
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
            row.layer,
            level_name(row.level),
            row.message
        );
        buffer.put_str(content.x, y, &text, style, content.width);
    }
}

fn level_color(level: LogLevel) -> Color {
    match level {
        LogLevel::Error => Color::Red,
        LogLevel::Warn => Color::Yellow,
        LogLevel::Info => Color::Default,
        LogLevel::Debug => Color::Gray,
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
    use super::{LogRow, LogView, render};
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
            layer: "git".to_string(),
            level,
            message: message.to_string(),
            selected,
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
