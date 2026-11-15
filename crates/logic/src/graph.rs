//! The commit graph layout: one row of typed cells per commit.
//!
//! Every column is a lane that waits for a commit. A commit takes the lane that waits for it,
//! or a new lane when no lane does (a branch tip). Other lanes waiting for the same commit join
//! it in its row. After the commit, its lane waits for the first parent, and every other
//! parent gets a lane of its own, or joins a lane already waiting for it.

use crate::model::Commit;

/// Which side of a cell the commit of the row is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitSide {
    /// The commit is to the left of the cell.
    Left,
    /// The commit is to the right of the cell.
    Right,
}

/// The shape drawn in a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// Nothing.
    Empty,
    /// The commit of the row.
    Commit,
    /// A lane passing through.
    Vertical,
    /// A horizontal connection passing through an empty column.
    Horizontal,
    /// A horizontal connection crossing a lane.
    Crossing,
    /// A lane coming from above that ends by joining the commit.
    Join(CommitSide),
    /// A new lane going down, for a parent of the commit.
    Fork(CommitSide),
    /// A lane that continues and is connected to the commit.
    Tee(CommitSide),
}

/// One cell of a graph row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    /// What is drawn.
    pub shape: Shape,
    /// The lane whose color the shape has.
    pub lane: usize,
    /// The lane whose color a horizontal connection to the next column has, if there is one.
    pub link_right: Option<usize>,
}

/// The graph cells of one commit's row, left to right.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GraphRow {
    /// The cells.
    pub cells: Vec<Cell>,
}

impl GraphRow {
    /// The lane of the row's commit, whose color the commit's line has.
    #[must_use]
    pub fn commit_lane(&self) -> Option<usize> {
        self.cells
            .iter()
            .find(|cell: &&Cell| cell.shape == Shape::Commit)
            .map(|cell: &Cell| cell.lane)
    }
}

/// Builds graph rows commit by commit, so that loading more commits continues the graph.
#[derive(Debug, Clone, Default)]
pub struct GraphBuilder {
    lanes: Vec<Option<String>>,
}

impl GraphBuilder {
    /// The row for the next commit in topological order.
    pub fn push(&mut self, commit: &Commit) -> GraphRow {
        let waiting: Vec<usize> = self.columns_waiting_for(&commit.hash);
        let column: usize = waiting
            .first()
            .copied()
            .unwrap_or_else(|| self.free_column());
        let before: Vec<Option<String>> = self.lanes.clone();
        for lane in &mut self.lanes {
            if lane.as_deref() == Some(commit.hash.as_str()) {
                *lane = None;
            }
        }
        self.set(column, commit.parents.first().cloned());
        let mut forks: Vec<usize> = Vec::new();
        let mut tees: Vec<usize> = Vec::new();
        for parent in commit.parents.iter().skip(1) {
            if let Some(existing) = self.columns_waiting_for(parent).first().copied() {
                tees.push(existing);
            } else {
                let free: usize = self.free_column();
                self.set(free, Some(parent.clone()));
                forks.push(free);
            }
        }
        let row: GraphRow = self.row(column, &waiting, &forks, &tees, &before);
        while self.lanes.last().is_some_and(Option::is_none) {
            self.lanes.pop();
        }
        row
    }

    fn columns_waiting_for(&self, hash: &str) -> Vec<usize> {
        self.lanes
            .iter()
            .enumerate()
            .filter(|(_, lane): &(usize, &Option<String>)| lane.as_deref() == Some(hash))
            .map(|(index, _): (usize, &Option<String>)| index)
            .collect()
    }

    fn free_column(&self) -> usize {
        self.lanes
            .iter()
            .position(Option::is_none)
            .unwrap_or(self.lanes.len())
    }

    fn set(&mut self, column: usize, value: Option<String>) {
        if column >= self.lanes.len() {
            self.lanes.resize(column + 1, None);
        }
        // The resize above makes the index valid.
        self.lanes[column] = value;
    }

    /// The cells of a row. `before` are the lanes above the row; the builder's lanes are the
    /// lanes below it.
    fn row(
        &self,
        column: usize,
        waiting: &[usize],
        forks: &[usize],
        tees: &[usize],
        before: &[Option<String>],
    ) -> GraphRow {
        let width: usize = before.len().max(self.lanes.len()).max(column + 1);
        let above = |i: usize| before.get(i).is_some_and(Option::is_some);
        let side = |i: usize| {
            if i < column {
                CommitSide::Right
            } else {
                CommitSide::Left
            }
        };
        let mut cells: Vec<Cell> = (0..width)
            .map(|i: usize| {
                let shape: Shape = if i == column {
                    Shape::Commit
                } else if forks.contains(&i) || tees.contains(&i) {
                    if above(i) {
                        Shape::Tee(side(i))
                    } else {
                        Shape::Fork(side(i))
                    }
                } else if waiting.contains(&i) {
                    Shape::Join(side(i))
                } else if above(i) {
                    Shape::Vertical
                } else {
                    Shape::Empty
                };
                Cell {
                    shape,
                    lane: i,
                    link_right: None,
                }
            })
            .collect();
        let connected = waiting.iter().skip(1).chain(forks).chain(tees).copied();
        for target in connected {
            let (from, to): (usize, usize) = (column.min(target), column.max(target));
            for (i, cell) in cells.iter_mut().enumerate().take(to).skip(from) {
                cell.link_right = Some(target);
                if i > from {
                    cell.shape = match cell.shape {
                        Shape::Vertical => Shape::Crossing,
                        Shape::Empty => Shape::Horizontal,
                        other => other,
                    };
                }
            }
        }
        GraphRow { cells }
    }
}

/// The rows for `commits`, newest first.
#[must_use]
pub fn layout(commits: &[Commit]) -> Vec<GraphRow> {
    let mut builder: GraphBuilder = GraphBuilder::default();
    commits.iter().map(|c: &Commit| builder.push(c)).collect()
}

#[cfg(test)]
mod tests;
