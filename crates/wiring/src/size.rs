//! Reading the terminal size.

/// Parses the output of `stty size`, which is `rows columns`.
#[must_use]
pub fn parse_stty_size(output: &str) -> Option<ui::layout::Size> {
    let mut parts = output.split_whitespace();
    let rows: usize = parts.next()?.parse().ok()?;
    let columns: usize = parts.next()?.parse().ok()?;
    Some(ui::layout::Size {
        width: columns,
        height: rows,
    })
}

#[cfg(test)]
mod tests {
    use super::parse_stty_size;
    use ui::layout::Size;

    #[test]
    fn parses_rows_and_columns() {
        assert_eq!(
            parse_stty_size("24 80\n"),
            Some(Size {
                width: 80,
                height: 24
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
