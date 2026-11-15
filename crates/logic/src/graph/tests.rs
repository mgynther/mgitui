use super::{Cell, CommitSide, GraphBuilder, GraphRow, Shape, layout};
use crate::model::Commit;

/// A commit `hash` with parents given as a space separated list.
fn commit(hash: &str, parents: &str) -> Commit {
    Commit {
        hash: hash.to_string(),
        parents: parents.split_whitespace().map(str::to_string).collect(),
        author: String::new(),
        date: String::new(),
        committer: String::new(),
        committer_date: String::new(),
        subject: String::new(),
    }
}

/// Draws a row the way the ui does, two characters per column, trailing spaces removed.
fn draw(row: &GraphRow) -> String {
    let mut text: String = String::new();
    for cell in &row.cells {
        text.push(match cell.shape {
            Shape::Empty => ' ',
            Shape::Commit => '●',
            Shape::Vertical => '│',
            Shape::Horizontal => '─',
            Shape::Crossing => '┼',
            Shape::Join(CommitSide::Left) => '╯',
            Shape::Join(CommitSide::Right) => '╰',
            Shape::Fork(CommitSide::Left) => '╮',
            Shape::Fork(CommitSide::Right) => '╭',
            Shape::Tee(CommitSide::Left) => '┤',
            Shape::Tee(CommitSide::Right) => '├',
        });
        text.push(if cell.link_right.is_some() {
            '─'
        } else {
            ' '
        });
    }
    text.trim_end().to_string()
}

fn drawn(commits: &[Commit]) -> Vec<String> {
    layout(commits).iter().map(draw).collect()
}

#[test]
fn the_commit_lane_is_the_lane_of_the_commit_cell() {
    let rows: Vec<GraphRow> = layout(&[commit("m", "d f"), commit("f", "b"), commit("d", "b")]);
    assert_eq!(rows[1].commit_lane(), Some(1));
    assert_eq!(rows[2].commit_lane(), Some(0));
    assert_eq!(GraphRow::default().commit_lane(), None);
}

#[test]
fn linear_history_is_one_lane() {
    assert_eq!(
        drawn(&[commit("c", "b"), commit("b", "a"), commit("a", "")]),
        vec!["●", "●", "●"]
    );
}

#[test]
fn branch_and_merge() {
    // m merges f into d; f and d both come from b.
    let commits: Vec<Commit> = vec![
        commit("m", "d f"),
        commit("f", "b"),
        commit("d", "b"),
        commit("b", "a"),
        commit("a", ""),
    ];
    assert_eq!(drawn(&commits), vec!["●─╮", "│ ●", "● │", "●─╯", "●"]);
}

#[test]
fn two_branch_tips_from_one_commit() {
    let commits: Vec<Commit> = vec![commit("x", "b"), commit("y", "b"), commit("b", "")];
    assert_eq!(drawn(&commits), vec!["●", "│ ●", "●─╯"]);
}

#[test]
fn merge_into_a_lane_that_already_waits() {
    // m has parents p and q; q is already waited for by lane 1 because of r.
    let commits: Vec<Commit> = vec![
        commit("r", "q"),
        commit("m", "p q"),
        commit("q", "p"),
        commit("p", ""),
    ];
    assert_eq!(drawn(&commits), vec!["●", "├─●", "● │", "●─╯"]);
}

#[test]
fn several_lanes_joining_one_commit() {
    let commits: Vec<Commit> = vec![
        commit("t", "base"),
        commit("m", "x y"),
        commit("y", "base"),
        commit("x", "base"),
        commit("base", ""),
    ];
    assert_eq!(
        drawn(&commits),
        vec!["●", "│ ●─╮", "│ │ ●", "│ ● │", "●─╯─╯"]
    );
}

#[test]
fn connections_cross_lanes() {
    // Lane 1 waits for the unrelated root r while lane 2 joins base in column 0.
    let commits: Vec<Commit> = vec![
        commit("t", "base"),
        commit("u", "r"),
        commit("v", "base"),
        commit("base", ""),
        commit("r", ""),
    ];
    assert_eq!(drawn(&commits), vec!["●", "│ ●", "│ │ ●", "●─┼─╯", "  ●"]);
    let rows: Vec<GraphRow> = layout(&commits);
    assert_eq!(
        rows[3].cells[1],
        Cell {
            shape: Shape::Crossing,
            lane: 1,
            link_right: Some(2)
        }
    );
}

#[test]
fn connections_pass_empty_columns() {
    let commits: Vec<Commit> = vec![
        commit("t", "base"),
        commit("u", "x"),
        commit("v", "base"),
        commit("x", ""),
        commit("base", ""),
    ];
    assert_eq!(drawn(&commits), vec!["●", "│ ●", "│ │ ●", "│ ● │", "●───╯"]);
}

#[test]
fn octopus_merge_and_roots() {
    let commits: Vec<Commit> = vec![
        commit("o", "a b c"),
        commit("c", ""),
        commit("b", ""),
        commit("a", ""),
    ];
    assert_eq!(drawn(&commits), vec!["●─╮─╮", "│ │ ●", "│ ●", "●"]);
}

#[test]
fn freed_column_is_reused_for_a_fork() {
    // j joins k's column at m, and m's second parent gets the freed column.
    let commits: Vec<Commit> = vec![
        commit("k", "m"),
        commit("j", "m"),
        commit("m", "a z"),
        commit("z", ""),
        commit("a", ""),
    ];
    assert_eq!(drawn(&commits), vec!["●", "│ ●", "●─┤", "│ ●", "●"]);
}

#[test]
fn fork_into_a_free_column_on_the_left() {
    let commits: Vec<Commit> = vec![
        commit("k", "z"),
        commit("j", "m"),
        commit("z", ""),
        commit("m", "a w"),
        commit("w", ""),
        commit("a", ""),
    ];
    assert_eq!(
        drawn(&commits),
        vec!["●", "│ ●", "● │", "╭─●", "● │", "  ●"]
    );
}

#[test]
fn builder_continues_across_chunks() {
    let commits: Vec<Commit> = vec![
        commit("m", "d f"),
        commit("f", "b"),
        commit("d", "b"),
        commit("b", ""),
    ];
    let mut builder: GraphBuilder = GraphBuilder::default();
    let first: Vec<GraphRow> = commits[..2].iter().map(|c| builder.push(c)).collect();
    let rest: Vec<GraphRow> = commits[2..].iter().map(|c| builder.push(c)).collect();
    let joined: Vec<GraphRow> = first.into_iter().chain(rest).collect();
    assert_eq!(joined, layout(&commits));
}

#[test]
fn lanes_have_their_column_color_and_links_the_target_color() {
    let rows: Vec<GraphRow> = layout(&[commit("m", "a b"), commit("b", ""), commit("a", "")]);
    assert_eq!(
        rows[0].cells,
        vec![
            Cell {
                shape: Shape::Commit,
                lane: 0,
                link_right: Some(1)
            },
            Cell {
                shape: Shape::Fork(CommitSide::Left),
                lane: 1,
                link_right: None
            },
        ]
    );
}
