//! Where the panes of each view are on the screen.

/// A terminal size in character cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    /// Columns.
    pub width: usize,
    /// Rows.
    pub height: usize,
}

/// The smallest terminal size the user interface is drawn in.
pub const MIN_SIZE: Size = Size {
    width: 80,
    height: 24,
};

impl Size {
    /// Whether the terminal is smaller than [`MIN_SIZE`].
    #[must_use]
    pub fn is_too_small(self) -> bool {
        self.width < MIN_SIZE.width || self.height < MIN_SIZE.height
    }
}

/// A rectangle of character cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    /// Left column.
    pub x: usize,
    /// Top row.
    pub y: usize,
    /// Columns.
    pub width: usize,
    /// Rows.
    pub height: usize,
}

impl Rect {
    /// The area below the pane's title row.
    #[must_use]
    pub fn content(self) -> Rect {
        Rect {
            y: self.y + 1,
            height: self.height.saturating_sub(1),
            ..self
        }
    }
}

/// The area between the tab row and the status line.
#[must_use]
pub fn body(size: Size) -> Rect {
    Rect {
        x: 0,
        y: 1,
        width: size.width,
        height: size.height.saturating_sub(2),
    }
}

/// Panes of the Branches view. Each pane starts with a title row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BranchesLayout {
    /// The commit graph.
    pub graph: Rect,
    /// Information about the selected commit.
    pub info: Rect,
    /// The diff of the selected file.
    pub file_diff: Rect,
    /// The files changed by the selected commit.
    pub files: Rect,
}

/// Panes of the Diff view. Each pane starts with a title row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiffLayout {
    /// Files with unstaged changes.
    pub unstaged: Rect,
    /// Files with staged changes.
    pub staged: Rect,
    /// The diff of the selected file.
    pub file_diff: Rect,
    /// The commit message.
    pub message: Rect,
}

/// Panes of the Log view. Each pane starts with a title row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogLayout {
    /// The log lines.
    pub list: Rect,
}

/// The layout of the Branches view.
#[must_use]
pub fn branches_layout(size: Size) -> BranchesLayout {
    let body: Rect = body(size);
    let graph_height: usize = body.height / 3;
    let bottom_y: usize = body.y + graph_height;
    let bottom_height: usize = body.height - graph_height;
    let left_width: usize = size.width * 2 / 3;
    let info_height: usize = bottom_height / 2;
    BranchesLayout {
        graph: Rect {
            height: graph_height,
            ..body
        },
        info: Rect {
            x: 0,
            y: bottom_y,
            width: left_width,
            height: info_height,
        },
        file_diff: Rect {
            x: 0,
            y: bottom_y + info_height,
            width: left_width,
            height: bottom_height - info_height,
        },
        files: Rect {
            x: left_width + 1,
            y: bottom_y,
            width: size.width.saturating_sub(left_width + 1),
            height: bottom_height,
        },
    }
}

/// The layout of the Diff view.
#[must_use]
pub fn diff_layout(size: Size) -> DiffLayout {
    let body: Rect = body(size);
    let left_width: usize = size.width / 3;
    let unstaged_height: usize = body.height / 2;
    let message_height: usize = body.height.min(8);
    let right_x: usize = left_width + 1;
    let right_width: usize = size.width.saturating_sub(right_x);
    DiffLayout {
        unstaged: Rect {
            width: left_width,
            height: unstaged_height,
            ..body
        },
        staged: Rect {
            x: 0,
            y: body.y + unstaged_height,
            width: left_width,
            height: body.height - unstaged_height,
        },
        file_diff: Rect {
            x: right_x,
            y: body.y,
            width: right_width,
            height: body.height - message_height,
        },
        message: Rect {
            x: right_x,
            y: body.y + body.height - message_height,
            width: right_width,
            height: message_height,
        },
    }
}

/// The layout of the Log view.
#[must_use]
pub fn log_layout(size: Size) -> LogLayout {
    LogLayout { list: body(size) }
}

#[cfg(test)]
mod tests {
    use super::{
        BranchesLayout, DiffLayout, LogLayout, MIN_SIZE, Rect, Size, branches_layout, diff_layout,
        log_layout,
    };

    const SIZE: Size = Size {
        width: 90,
        height: 32,
    };

    #[test]
    fn too_small() {
        assert!(!MIN_SIZE.is_too_small());
        assert!(
            Size {
                width: 79,
                height: 24
            }
            .is_too_small()
        );
        assert!(
            Size {
                width: 80,
                height: 23
            }
            .is_too_small()
        );
    }

    #[test]
    fn content_is_below_title() {
        let rect: Rect = Rect {
            x: 1,
            y: 2,
            width: 3,
            height: 4,
        };
        assert_eq!(
            rect.content(),
            Rect {
                x: 1,
                y: 3,
                width: 3,
                height: 3
            }
        );
        let empty: Rect = Rect { height: 0, ..rect };
        assert_eq!(empty.content().height, 0);
    }

    #[test]
    fn branches_panes() {
        assert_eq!(
            branches_layout(SIZE),
            BranchesLayout {
                graph: Rect {
                    x: 0,
                    y: 1,
                    width: 90,
                    height: 10
                },
                info: Rect {
                    x: 0,
                    y: 11,
                    width: 60,
                    height: 10
                },
                file_diff: Rect {
                    x: 0,
                    y: 21,
                    width: 60,
                    height: 10
                },
                files: Rect {
                    x: 61,
                    y: 11,
                    width: 29,
                    height: 20
                },
            }
        );
    }

    #[test]
    fn diff_panes() {
        assert_eq!(
            diff_layout(SIZE),
            DiffLayout {
                unstaged: Rect {
                    x: 0,
                    y: 1,
                    width: 30,
                    height: 15
                },
                staged: Rect {
                    x: 0,
                    y: 16,
                    width: 30,
                    height: 15
                },
                file_diff: Rect {
                    x: 31,
                    y: 1,
                    width: 59,
                    height: 22
                },
                message: Rect {
                    x: 31,
                    y: 23,
                    width: 59,
                    height: 8
                },
            }
        );
    }

    #[test]
    fn log_pane() {
        assert_eq!(
            log_layout(SIZE),
            LogLayout {
                list: Rect {
                    x: 0,
                    y: 1,
                    width: 90,
                    height: 30
                }
            }
        );
    }

    #[test]
    fn tiny_sizes_do_not_overflow() {
        let tiny: Size = Size {
            width: 0,
            height: 0,
        };
        assert_eq!(branches_layout(tiny).files.width, 0);
        assert_eq!(diff_layout(tiny).message.height, 0);
    }
}
