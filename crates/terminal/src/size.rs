//! Reading the terminal size.

/// The size of the terminal in characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    /// The number of rows.
    pub rows: usize,
    /// The number of columns.
    pub columns: usize,
}

/// Parses the output of `stty size`, which is `rows columns`.
#[must_use]
pub fn parse_stty_size(output: &str) -> Option<Size> {
    let mut parts = output.split_whitespace();
    let rows: usize = parts.next()?.parse().ok()?;
    let columns: usize = parts.next()?.parse().ok()?;
    Some(Size { rows, columns })
}

#[cfg(test)]
mod tests {
    use super::{Size, parse_stty_size};

    #[test]
    fn parses_rows_and_columns() {
        assert_eq!(
            parse_stty_size("24 80\n"),
            Some(Size {
                rows: 24,
                columns: 80
            })
        );
    }

    #[test]
    fn rejects_bad_output() {
        assert_eq!(parse_stty_size(""), None);
        assert_eq!(parse_stty_size("24"), None);
        assert_eq!(parse_stty_size("x 80"), None);
        assert_eq!(parse_stty_size("24 y"), None);
    }
}
