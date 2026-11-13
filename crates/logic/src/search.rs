//! Searching lists, wrapping around at the ends like Vim.

/// Which way a search goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Towards the end of the list.
    Forward,
    /// Towards the start of the list.
    Backward,
}

/// A search match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Found {
    /// The matching item.
    pub index: usize,
    /// Whether the search went past the end or start of the list.
    pub wrapped: bool,
}

/// Finds the next item in a list of `len` items for which `matches` is true, starting after
/// `from` in `direction` and wrapping around. `from` itself is checked last.
pub fn find(
    len: usize,
    from: usize,
    direction: Direction,
    matches: impl Fn(usize) -> bool,
) -> Option<Found> {
    (1..=len)
        .map(|step: usize| match direction {
            Direction::Forward => Found {
                index: (from + step) % len,
                wrapped: from + step >= len,
            },
            Direction::Backward => Found {
                index: (from + len - step) % len,
                wrapped: step > from,
            },
        })
        .find(|found: &Found| matches(found.index))
}

/// Whether `text` contains `term`, ignoring case.
#[must_use]
pub fn text_matches(text: &str, term: &str) -> bool {
    text.to_lowercase().contains(&term.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::{Direction, Found, find, text_matches};

    const ITEMS: [&str; 5] = ["alpha", "beta", "gamma", "beta", "delta"];

    fn search(from: usize, direction: Direction, term: &str) -> Option<Found> {
        find(ITEMS.len(), from, direction, |i: usize| {
            text_matches(ITEMS[i], term)
        })
    }

    #[test]
    fn forward_and_wrapping() {
        assert_eq!(
            search(0, Direction::Forward, "beta"),
            Some(Found {
                index: 1,
                wrapped: false
            })
        );
        assert_eq!(
            search(1, Direction::Forward, "beta"),
            Some(Found {
                index: 3,
                wrapped: false
            })
        );
        assert_eq!(
            search(3, Direction::Forward, "beta"),
            Some(Found {
                index: 1,
                wrapped: true
            })
        );
    }

    #[test]
    fn backward_and_wrapping() {
        assert_eq!(
            search(3, Direction::Backward, "beta"),
            Some(Found {
                index: 1,
                wrapped: false
            })
        );
        assert_eq!(
            search(1, Direction::Backward, "beta"),
            Some(Found {
                index: 3,
                wrapped: true
            })
        );
    }

    #[test]
    fn only_match_is_the_start() {
        assert_eq!(
            search(2, Direction::Forward, "GAM"),
            Some(Found {
                index: 2,
                wrapped: true
            })
        );
    }

    #[test]
    fn no_match_or_empty_list() {
        assert_eq!(search(0, Direction::Forward, "zeta"), None);
        assert_eq!(find(0, 0, Direction::Forward, |_| true), None);
    }
}
