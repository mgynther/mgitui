//! Colors and text styles.

/// A terminal color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    /// The terminal's default color.
    Default,
    /// Red.
    Red,
    /// Green.
    Green,
    /// Yellow.
    Yellow,
    /// Blue.
    Blue,
    /// Magenta.
    Magenta,
    /// Cyan.
    Cyan,
    /// Gray.
    Gray,
    /// Orange, from the 256 color palette.
    Orange,
}

impl Color {
    /// The SGR parameters that select this color as the foreground color.
    #[must_use]
    pub fn sgr(self) -> &'static str {
        match self {
            Color::Default => "39",
            Color::Red => "31",
            Color::Green => "32",
            Color::Yellow => "33",
            Color::Blue => "34",
            Color::Magenta => "35",
            Color::Cyan => "36",
            Color::Gray => "90",
            Color::Orange => "38;5;208",
        }
    }
}

/// How a piece of text is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Style {
    /// Foreground color.
    pub fg: Color,
    /// Bold text.
    pub bold: bool,
    /// Swapped foreground and background, used for selections.
    pub reverse: bool,
}

impl Style {
    /// The default style.
    pub const PLAIN: Style = Style {
        fg: Color::Default,
        bold: false,
        reverse: false,
    };

    /// Plain text in `fg`.
    #[must_use]
    pub fn fg(fg: Color) -> Style {
        Style { fg, ..Style::PLAIN }
    }

    /// The same style in bold.
    #[must_use]
    pub fn bold(self) -> Style {
        Style { bold: true, ..self }
    }

    /// The same style, reversed when `reverse` is true.
    #[must_use]
    pub fn reversed_if(self, reverse: bool) -> Style {
        Style {
            reverse: self.reverse || reverse,
            ..self
        }
    }

    /// The SGR escape sequence that switches to this style.
    #[must_use]
    pub fn sgr(self) -> String {
        let mut params: String = format!("0;{}", self.fg.sgr());
        if self.bold {
            params.push_str(";1");
        }
        if self.reverse {
            params.push_str(";7");
        }
        format!("\x1b[{params}m")
    }
}

#[cfg(test)]
mod tests {
    use super::{Color, Style};

    #[test]
    fn color_codes() {
        let cases: [(Color, &str); 9] = [
            (Color::Default, "39"),
            (Color::Red, "31"),
            (Color::Green, "32"),
            (Color::Yellow, "33"),
            (Color::Blue, "34"),
            (Color::Magenta, "35"),
            (Color::Cyan, "36"),
            (Color::Gray, "90"),
            (Color::Orange, "38;5;208"),
        ];
        for (color, code) in cases {
            assert_eq!(color.sgr(), code);
        }
    }

    #[test]
    fn style_sequences() {
        assert_eq!(Style::PLAIN.sgr(), "\x1b[0;39m");
        assert_eq!(Style::fg(Color::Red).bold().sgr(), "\x1b[0;31;1m");
        assert_eq!(Style::PLAIN.reversed_if(true).sgr(), "\x1b[0;39;7m");
        assert_eq!(Style::PLAIN.reversed_if(false), Style::PLAIN);
    }
}
