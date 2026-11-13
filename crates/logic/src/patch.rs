//! Building a patch that stages or unstages selected lines of a diff.

use std::collections::BTreeSet;
use std::fmt::Write;

use crate::model::{FileDiff, Hunk, LineKind};

/// A line of a hunk: (hunk index, line index).
pub type LineRef = (usize, usize);

/// Builds a patch with only the `selected` added and removed lines of `diff`.
///
/// For staging (`reverse` false) the diff is working tree against index and the patch is
/// applied to the index as is: unselected removed lines stay as context and unselected added
/// lines are left out. For unstaging (`reverse` true) the diff is index against `HEAD` and the
/// patch is applied in reverse: unselected added lines stay as context and unselected removed
/// lines are left out.
///
/// Returns `None` when no added or removed line is selected.
#[must_use]
pub fn build_patch(diff: &FileDiff, selected: &BTreeSet<LineRef>, reverse: bool) -> Option<String> {
    let mut patch: String = String::new();
    for line in &diff.header {
        patch.push_str(line);
        patch.push('\n');
    }
    // Line counts of the hunks already in the patch. Only hunks in the patch move the lines
    // after them, so the start of the side that changes is shifted by their difference.
    let mut old_total: usize = 0;
    let mut new_total: usize = 0;
    let mut any: bool = false;
    for (hunk_index, hunk) in diff.hunks.iter().enumerate() {
        let selected_in_hunk = |line: usize| selected.contains(&(hunk_index, line));
        let has_change: bool = hunk.lines.iter().enumerate().any(|(i, l)| {
            selected_in_hunk(i) && matches!(l.kind, LineKind::Added | LineKind::Removed)
        });
        if !has_change {
            continue;
        }
        any = true;
        let (body, old_count, new_count): (String, usize, usize) =
            hunk_body(hunk, &selected_in_hunk, reverse);
        let (old_start, new_start): (usize, usize) = if reverse {
            (
                (hunk.new_start + old_total).saturating_sub(new_total),
                hunk.new_start,
            )
        } else {
            (
                hunk.old_start,
                (hunk.old_start + new_total).saturating_sub(old_total),
            )
        };
        let _ = writeln!(
            patch,
            "@@ -{},{old_count} +{},{new_count} @@",
            start(old_start, old_count),
            start(new_start, new_count)
        );
        patch.push_str(&body);
        old_total += old_count;
        new_total += new_count;
    }
    any.then_some(patch)
}

/// The start line for a header: git writes the line before when the count is 0.
fn start(line: usize, count: usize) -> usize {
    if count == 0 {
        line.saturating_sub(1)
    } else {
        line
    }
}

/// The lines of one hunk of the patch, with the old and new line counts.
fn hunk_body(
    hunk: &Hunk,
    selected: &dyn Fn(usize) -> bool,
    reverse: bool,
) -> (String, usize, usize) {
    let mut body: String = String::new();
    let mut old_count: usize = 0;
    let mut new_count: usize = 0;
    let mut previous_kept: bool = false;
    for (index, line) in hunk.lines.iter().enumerate() {
        let kept: Option<char> = match (line.kind, selected(index), reverse) {
            (LineKind::Context, _, _)
            | (LineKind::Removed, false, false)
            | (LineKind::Added, false, true) => Some(' '),
            (LineKind::Added, true, _) => Some('+'),
            (LineKind::Removed, true, _) => Some('-'),
            (LineKind::Added, false, false) | (LineKind::Removed, false, true) => None,
            (LineKind::NoNewline, _, _) => previous_kept.then_some('\\'),
        };
        if let Some(prefix) = kept {
            let _ = writeln!(body, "{prefix}{}", line.text);
            if matches!(prefix, ' ' | '-') {
                old_count += 1;
            }
            if matches!(prefix, ' ' | '+') {
                new_count += 1;
            }
        }
        if line.kind != LineKind::NoNewline {
            previous_kept = kept.is_some();
        }
    }
    (body, old_count, new_count)
}

#[cfg(test)]
mod tests;
