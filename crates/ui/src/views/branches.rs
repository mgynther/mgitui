//! The Branches view: the commit graph, commit information, files and the file diff.

use crate::buffer::Buffer;
use crate::layout::{BranchesLayout, Rect, Size, branches_layout};
use crate::screen::{draw_separator, draw_title};
use crate::style::{Color, Style};
use crate::views::diff_text::{self, DiffRowView};
use crate::views::file_list::{self, FileRow};

/// Width of the date column.
const DATE_WIDTH: usize = 16;
/// Width of the author column.
const AUTHOR_WIDTH: usize = 18;
/// Colors of the lanes, used in turn.
const LANE_COLORS: [Color; 6] = [
    Color::Blue,
    Color::Magenta,
    Color::Cyan,
    Color::Yellow,
    Color::Green,
    Color::Red,
];

/// Which side of a cell the commit of the row is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitSide {
    /// To the left.
    Left,
    /// To the right.
    Right,
}

/// The shape of a graph cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphShape {
    /// Nothing.
    Empty,
    /// The commit of the row.
    Commit,
    /// A lane passing through.
    Vertical,
    /// A horizontal connection.
    Horizontal,
    /// A horizontal connection crossing a lane.
    Crossing,
    /// A lane from above joining the commit.
    Join(CommitSide),
    /// A new lane going down.
    Fork(CommitSide),
    /// A lane that continues and connects to the commit.
    Tee(CommitSide),
}

/// One cell of the graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraphCell {
    /// The shape.
    pub shape: GraphShape,
    /// The lane that gives the shape its color.
    pub lane: usize,
    /// The lane that gives a connection to the next column its color.
    pub link_right: Option<usize>,
}

/// A branch or tag label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LabelView {
    /// A local branch, bold when checked out.
    Local {
        /// The name.
        name: String,
        /// Whether it is checked out.
        head: bool,
    },
    /// A remote-tracking branch.
    Remote {
        /// The remote.
        remote: String,
        /// The branch on the remote.
        branch: String,
    },
    /// A tag.
    Tag(String),
}

/// The information about the selected commit shown in the commit box.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitInfo {
    /// The full hash, drawn bold in the color of the commit's line.
    pub hash: String,
    /// The lane of the commit in the graph.
    pub lane: Option<usize>,
    /// The author name.
    pub author: String,
    /// The author date.
    pub author_date: String,
    /// The committer name.
    pub committer: String,
    /// The committer date.
    pub committer_date: String,
    /// The lines of the full commit message.
    pub message: Vec<String>,
}

/// One commit row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitRow {
    /// The graph cells.
    pub cells: Vec<GraphCell>,
    /// Branch and tag labels.
    pub labels: Vec<LabelView>,
    /// The first line of the message.
    pub subject: String,
    /// The author.
    pub author: String,
    /// The date.
    pub date: String,
    /// Whether the row is selected.
    pub selected: bool,
    /// Whether `HEAD` points to the commit.
    pub head: bool,
}

/// Which pane of the Branches view has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchesFocus {
    /// The graph.
    Graph,
    /// The file list.
    Files,
    /// The file diff.
    FileDiff,
}

/// What the Branches view shows. Lists contain only the visible rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchesView {
    /// The focused pane.
    pub focus: BranchesFocus,
    /// The visible commit rows.
    pub rows: Vec<CommitRow>,
    /// The selected commit, for the commit information box.
    pub info: Option<CommitInfo>,
    /// The visible files of the selected commit.
    pub files: Vec<FileRow>,
    /// The title of the file diff pane.
    pub diff_title: String,
    /// The visible rows of the file diff.
    pub diff: Vec<DiffRowView>,
}

/// Draws the Branches view.
pub fn render(buffer: &mut Buffer, view: &BranchesView, size: Size) {
    let panes: BranchesLayout = branches_layout(size);
    draw_title(
        buffer,
        panes.graph,
        "Commits",
        view.focus == BranchesFocus::Graph,
    );
    let graph: Rect = panes.graph.content();
    let graph_width: usize = size.width / 3;
    for (y, row) in (graph.y..graph.y + graph.height).zip(&view.rows) {
        draw_commit_row(
            buffer,
            Rect {
                y,
                height: 1,
                ..graph
            },
            row,
            graph_width,
            view.focus,
        );
    }
    draw_title(buffer, panes.info, "Commit", false);
    if let Some(commit) = &view.info {
        draw_info(buffer, panes.info.content(), commit);
    }
    draw_title(
        buffer,
        panes.file_diff,
        &view.diff_title,
        view.focus == BranchesFocus::FileDiff,
    );
    diff_text::render(
        buffer,
        panes.file_diff.content(),
        &view.diff,
        view.focus == BranchesFocus::FileDiff,
    );
    draw_title(
        buffer,
        panes.files,
        "Files",
        view.focus == BranchesFocus::Files,
    );
    file_list::render(
        buffer,
        panes.files.content(),
        &view.files,
        view.focus == BranchesFocus::Files,
    );
    draw_separator(buffer, panes.files);
}

fn draw_commit_row(
    buffer: &mut Buffer,
    rect: Rect,
    row: &CommitRow,
    graph_width: usize,
    focus: BranchesFocus,
) {
    let reverse: bool = row.selected && focus == BranchesFocus::Graph;
    let base: Style = if row.selected && !reverse {
        Style::PLAIN.bold()
    } else {
        Style::PLAIN.reversed_if(reverse)
    };
    buffer.fill(rect, base);
    let date_x: usize = rect.x + rect.width.saturating_sub(DATE_WIDTH);
    let author_x: usize = date_x.saturating_sub(AUTHOR_WIDTH + 1);
    buffer.put_str(date_x, rect.y, &row.date, base, DATE_WIDTH);
    buffer.put_str(author_x, rect.y, &row.author, base, AUTHOR_WIDTH);
    let text_end: usize = author_x.saturating_sub(1);
    let mut x: usize = rect.x + draw_graph(buffer, rect.x, rect.y, row, graph_width, reverse);
    for label in &row.labels {
        x += 1;
        x += draw_label(
            buffer,
            x,
            rect.y,
            label,
            reverse,
            text_end.saturating_sub(x),
        );
    }
    buffer.put_str(
        x + 1,
        rect.y,
        &row.subject,
        base,
        text_end.saturating_sub(x + 1),
    );
}

/// Draws the graph cells, two columns each, within `width` columns. Returns the columns used.
fn draw_graph(
    buffer: &mut Buffer,
    x: usize,
    y: usize,
    row: &CommitRow,
    width: usize,
    reverse: bool,
) -> usize {
    let fits: usize = if row.cells.len() * 2 <= width {
        row.cells.len()
    } else {
        width.saturating_sub(1) / 2
    };
    let mut column: usize = x;
    for cell in row.cells.iter().take(fits) {
        let style: Style = Style::fg(lane_color(cell.lane)).reversed_if(reverse);
        buffer.set(column, y, glyph(cell.shape, row.head), style);
        let (link, link_style): (char, Style) = match cell.link_right {
            Some(lane) => ('─', Style::fg(lane_color(lane)).reversed_if(reverse)),
            None => (' ', Style::PLAIN.reversed_if(reverse)),
        };
        buffer.set(column + 1, y, link, link_style);
        column += 2;
    }
    if fits < row.cells.len() {
        buffer.set(column, y, '…', Style::fg(Color::Gray).reversed_if(reverse));
        column += 1;
    }
    column - x
}

/// Draws a label like `[main]`, `[origin/main]` or `<v1>`. Returns the columns used.
fn draw_label(
    buffer: &mut Buffer,
    x: usize,
    y: usize,
    label: &LabelView,
    reverse: bool,
    max_width: usize,
) -> usize {
    let green: Style = Style::fg(Color::Green).reversed_if(reverse);
    let parts: Vec<(String, Style)> = match label {
        LabelView::Local { name, head } => vec![(
            format!("[{name}]"),
            if *head { green.bold() } else { green },
        )],
        LabelView::Remote { remote, branch } => vec![
            ("[".to_string(), green),
            (
                format!("{remote}/"),
                Style::fg(Color::Orange).reversed_if(reverse),
            ),
            (format!("{branch}]"), green),
        ],
        LabelView::Tag(name) => vec![(
            format!("<{name}>"),
            Style::fg(Color::Yellow).reversed_if(reverse),
        )],
    };
    let mut used: usize = 0;
    for (text, style) in parts {
        used += buffer.put_str(x + used, y, &text, style, max_width.saturating_sub(used));
    }
    used
}

/// Draws the commit box: the hash in the color of the commit's line, then author, committer
/// and the message.
fn draw_info(buffer: &mut Buffer, area: Rect, commit: &CommitInfo) {
    let hash: Style = commit
        .lane
        .map_or(Style::PLAIN, |lane: usize| Style::fg(lane_color(lane)))
        .bold();
    let mut lines: Vec<Vec<(String, Style)>> = vec![
        vec![
            ("commit ".to_string(), Style::PLAIN),
            (commit.hash.clone(), hash),
        ],
        vec![(
            format!("Author: {}  Date: {}", commit.author, commit.author_date),
            Style::PLAIN,
        )],
        vec![(
            format!(
                "Committer: {}  Date: {}",
                commit.committer, commit.committer_date
            ),
            Style::PLAIN,
        )],
    ];
    lines.extend(
        commit
            .message
            .iter()
            .map(|line: &String| vec![(line.clone(), Style::PLAIN)]),
    );
    for (y, parts) in (area.y..area.y + area.height).zip(&lines) {
        let mut used: usize = 0;
        for (text, style) in parts {
            used += buffer.put_str(
                area.x + used,
                y,
                text,
                *style,
                area.width.saturating_sub(used),
            );
        }
    }
}

fn lane_color(lane: usize) -> Color {
    // The remainder is always a valid index.
    LANE_COLORS[lane % LANE_COLORS.len()]
}

fn glyph(shape: GraphShape, head: bool) -> char {
    match shape {
        GraphShape::Empty => ' ',
        GraphShape::Commit if head => '◉',
        GraphShape::Commit => '●',
        GraphShape::Vertical => '│',
        GraphShape::Horizontal => '─',
        GraphShape::Crossing => '┼',
        GraphShape::Join(CommitSide::Left) => '╯',
        GraphShape::Join(CommitSide::Right) => '╰',
        GraphShape::Fork(CommitSide::Left) => '╮',
        GraphShape::Fork(CommitSide::Right) => '╭',
        GraphShape::Tee(CommitSide::Left) => '┤',
        GraphShape::Tee(CommitSide::Right) => '├',
    }
}

#[cfg(test)]
mod tests;
