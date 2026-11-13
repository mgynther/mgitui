use super::{
    BranchesFocus, BranchesView, CommitRow, CommitSide, GraphCell, GraphShape, LabelView, glyph,
    lane_color, render,
};
use crate::buffer::Buffer;
use crate::layout::Size;
use crate::style::Color;
use crate::views::diff_text::{DiffRowKind, DiffRowView};
use crate::views::file_list::FileRow;

const SIZE: Size = Size {
    width: 90,
    height: 30,
};

fn cell(shape: GraphShape, lane: usize, link_right: Option<usize>) -> GraphCell {
    GraphCell {
        shape,
        lane,
        link_right,
    }
}

fn row(cells: Vec<GraphCell>, labels: Vec<LabelView>, selected: bool, head: bool) -> CommitRow {
    CommitRow {
        cells,
        labels,
        subject: "Subject".to_string(),
        author: "Ada Author".to_string(),
        date: "2025-01-02 03:04".to_string(),
        selected,
        head,
    }
}

fn view(focus: BranchesFocus, rows: Vec<CommitRow>) -> BranchesView {
    BranchesView {
        focus,
        rows,
        info: vec!["commit abc".to_string(), "Author: Ada".to_string()],
        files: vec![FileRow {
            label: "M a.txt".to_string(),
            selected: true,
        }],
        diff_title: "Diff a.txt".to_string(),
        diff: vec![DiffRowView {
            kind: DiffRowKind::Added,
            text: "new".to_string(),
            cursor: true,
            marked: false,
        }],
    }
}

fn draw(view: &BranchesView) -> Buffer {
    let mut buffer: Buffer = Buffer::new(SIZE);
    render(&mut buffer, view, SIZE);
    buffer
}

#[test]
fn commit_row_with_graph_labels_author_and_date() {
    let commit: CommitRow = row(
        vec![
            cell(GraphShape::Commit, 0, Some(1)),
            cell(GraphShape::Fork(CommitSide::Left), 1, None),
        ],
        vec![
            LabelView::Local {
                name: "main".to_string(),
                head: true,
            },
            LabelView::Remote {
                remote: "origin".to_string(),
                branch: "main".to_string(),
            },
            LabelView::Tag("v1".to_string()),
        ],
        true,
        true,
    );
    let buffer: Buffer = draw(&view(BranchesFocus::Graph, vec![commit]));
    let text: String = buffer.row_text(2);
    assert!(
        text.starts_with("◉─╮  [main] [origin/main] <v1> Subject"),
        "{text}"
    );
    assert!(
        text.ends_with("Ada Author         2025-01-02 03:04"),
        "{text}"
    );
    assert_eq!(buffer.cell(0, 2).unwrap().style.fg, Color::Blue);
    assert_eq!(buffer.cell(1, 2).unwrap().style.fg, Color::Magenta);
    assert!(buffer.cell(0, 2).unwrap().style.reverse);
    assert!(buffer.cell(6, 2).unwrap().style.bold);
    assert_eq!(buffer.cell(6, 2).unwrap().style.fg, Color::Green);
    assert_eq!(buffer.cell(14, 2).unwrap().style.fg, Color::Orange);
    assert_eq!(buffer.cell(21, 2).unwrap().style.fg, Color::Green);
    assert_eq!(buffer.cell(27, 2).unwrap().style.fg, Color::Yellow);
}

#[test]
fn unfocused_selection_is_bold_and_other_panes_are_drawn() {
    let commit: CommitRow = row(vec![cell(GraphShape::Commit, 0, None)], vec![], true, false);
    let other: CommitRow = row(
        vec![cell(GraphShape::Commit, 0, None)],
        vec![LabelView::Local {
            name: "dev".to_string(),
            head: false,
        }],
        false,
        false,
    );
    let buffer: Buffer = draw(&view(BranchesFocus::Files, vec![commit, other]));
    assert!(buffer.row_text(2).starts_with("●  Subject"));
    assert!(buffer.cell(3, 2).unwrap().style.bold);
    assert!(!buffer.cell(3, 2).unwrap().style.reverse);
    assert!(buffer.row_text(3).starts_with("●  [dev] Subject"));
    assert!(!buffer.cell(3, 3).unwrap().style.bold);
    assert!(buffer.row_text(15).starts_with("─ Commit ─"));
    assert!(buffer.row_text(16).starts_with("commit abc"));
    assert!(buffer.row_text(15).contains("│─ Files ─"));
    assert!(buffer.row_text(16).contains("│M a.txt"));
    assert!(buffer.row_text(22).starts_with("─ Diff a.txt ─"));
    assert!(buffer.row_text(23).starts_with(" +new"));
    let diff_focus: Buffer = draw(&view(BranchesFocus::FileDiff, vec![]));
    assert!(diff_focus.row_text(23).starts_with("▶+new"));
}

#[test]
fn wide_graphs_are_cut_off() {
    let cells: Vec<GraphCell> = (0..20)
        .map(|lane: usize| cell(GraphShape::Vertical, lane, None))
        .collect();
    let buffer: Buffer = draw(&view(
        BranchesFocus::Graph,
        vec![row(cells, vec![], false, false)],
    ));
    let text: String = buffer.row_text(2);
    assert!(
        text.starts_with(&format!("{}│ … Subject", "│ ".repeat(13))),
        "{text}"
    );
}

#[test]
fn glyphs_and_lane_colors() {
    let cases: [(GraphShape, char); 11] = [
        (GraphShape::Empty, ' '),
        (GraphShape::Commit, '●'),
        (GraphShape::Vertical, '│'),
        (GraphShape::Horizontal, '─'),
        (GraphShape::Crossing, '┼'),
        (GraphShape::Join(CommitSide::Left), '╯'),
        (GraphShape::Join(CommitSide::Right), '╰'),
        (GraphShape::Fork(CommitSide::Left), '╮'),
        (GraphShape::Fork(CommitSide::Right), '╭'),
        (GraphShape::Tee(CommitSide::Left), '┤'),
        (GraphShape::Tee(CommitSide::Right), '├'),
    ];
    for (shape, expected) in cases {
        assert_eq!(glyph(shape, false), expected);
    }
    assert_eq!(glyph(GraphShape::Commit, true), '◉');
    assert_eq!(lane_color(6), Color::Blue);
    assert_eq!(lane_color(5), Color::Red);
}
