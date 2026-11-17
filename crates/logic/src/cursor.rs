//! A selection in a scrolling list.

/// The selected item of a list and the first item shown.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    selected: usize,
    offset: usize,
}

impl Cursor {
    /// The selected item.
    #[must_use]
    pub fn selected(self) -> usize {
        self.selected
    }

    /// The first item shown.
    #[must_use]
    pub fn offset(self) -> usize {
        self.offset
    }

    /// Selects the item `count` items further down a list of `len` items shown in `height`
    /// rows, or the last item when there are fewer below.
    pub fn down(&mut self, count: usize, len: usize, height: usize) {
        self.select(self.selected.saturating_add(count), len, height);
    }

    /// Selects the item `count` items further up, or the first item when there are fewer
    /// above.
    pub fn up(&mut self, count: usize, len: usize, height: usize) {
        self.select(self.selected.saturating_sub(count), len, height);
    }

    /// Selects the first item.
    pub fn top(&mut self) {
        self.selected = 0;
        self.offset = 0;
    }

    /// Selects the last item.
    pub fn bottom(&mut self, len: usize, height: usize) {
        self.select(len.saturating_sub(1), len, height);
    }

    /// Selects item `index`, or the last item when `index` is past the end.
    pub fn select(&mut self, index: usize, len: usize, height: usize) {
        self.selected = index.min(len.saturating_sub(1));
        self.scroll(len, height);
    }

    /// Keeps the selection valid after the list or the number of rows changed.
    pub fn clamp(&mut self, len: usize, height: usize) {
        self.select(self.selected, len, height);
    }

    /// Scrolls so that the selection is shown, without leaving empty rows at the end.
    fn scroll(&mut self, len: usize, height: usize) {
        self.offset = self.offset.min(len.saturating_sub(height));
        if self.selected < self.offset {
            self.offset = self.selected;
        } else if self.selected >= self.offset + height {
            self.offset = (self.selected + 1).saturating_sub(height.max(1));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Cursor;

    fn at(selected: usize, offset: usize) -> Cursor {
        Cursor { selected, offset }
    }

    #[test]
    fn moving_down_scrolls_at_the_bottom_edge() {
        let mut cursor: Cursor = Cursor::default();
        cursor.down(1, 10, 3);
        cursor.down(1, 10, 3);
        assert_eq!(cursor, at(2, 0));
        cursor.down(1, 10, 3);
        assert_eq!(cursor, at(3, 1));
        assert_eq!((cursor.selected(), cursor.offset()), (3, 1));
    }

    #[test]
    fn moving_up_scrolls_at_the_top_edge() {
        let mut cursor: Cursor = at(3, 3);
        cursor.up(1, 10, 3);
        assert_eq!(cursor, at(2, 2));
        let mut first: Cursor = Cursor::default();
        first.up(1, 10, 3);
        assert_eq!(first, at(0, 0));
    }

    #[test]
    fn top_bottom_and_ends() {
        let mut cursor: Cursor = Cursor::default();
        cursor.bottom(10, 3);
        assert_eq!(cursor, at(9, 7));
        cursor.down(1, 10, 3);
        assert_eq!(cursor, at(9, 7));
        cursor.top();
        assert_eq!(cursor, at(0, 0));
    }

    #[test]
    fn moving_by_many_stops_at_the_ends() {
        let mut cursor: Cursor = Cursor::default();
        cursor.down(20, 50, 10);
        assert_eq!(cursor, at(20, 11));
        cursor.down(20, 30, 10);
        assert_eq!(cursor, at(29, 20));
        cursor.down(20, 30, 10);
        assert_eq!(cursor, at(29, 20));
        cursor.up(20, 30, 10);
        assert_eq!(cursor, at(9, 9));
        cursor.up(20, 30, 10);
        assert_eq!(cursor, at(0, 0));
        cursor.down(usize::MAX, 30, 10);
        assert_eq!(cursor, at(29, 20));
    }

    #[test]
    fn empty_list_and_zero_height() {
        let mut cursor: Cursor = at(5, 5);
        cursor.clamp(0, 3);
        assert_eq!(cursor, at(0, 0));
        cursor.select(4, 10, 0);
        assert_eq!(cursor, at(4, 4));
    }

    #[test]
    fn clamp_after_list_shrinks_or_rows_grow() {
        let mut cursor: Cursor = at(9, 7);
        cursor.clamp(5, 3);
        assert_eq!(cursor, at(4, 2));
        cursor.clamp(5, 10);
        assert_eq!(cursor, at(4, 0));
    }
}
