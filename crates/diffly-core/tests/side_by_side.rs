//! Side-by-side alignment of line diffs, driven with in-memory inputs.

#![allow(clippy::unwrap_used)]

use diffly_core::{DiffBody, DiffMode, DiffOptions, SideBySideRow, SideLine, TextChangeKind, diff};
use pretty_assertions::assert_eq;

fn rows(left: &str, right: &str) -> Vec<SideBySideRow> {
    match diff(left, right, &DiffOptions::text(DiffMode::Line))
        .unwrap()
        .body
    {
        DiffBody::Text(text) => text.side_by_side(),
        DiffBody::Json(_) => panic!("expected a text diff"),
    }
}

fn line(number: usize, text: &str, kind: TextChangeKind) -> SideLine {
    SideLine {
        number,
        text: text.to_owned(),
        kind,
    }
}

fn same(left: usize, right: usize, text: &str) -> SideBySideRow {
    SideBySideRow {
        left: Some(line(left, text, TextChangeKind::Equal)),
        right: Some(line(right, text, TextChangeKind::Equal)),
    }
}

/// Takes a `SideLine` or `None` for each side.
fn row(left: impl Into<Option<SideLine>>, right: impl Into<Option<SideLine>>) -> SideBySideRow {
    SideBySideRow {
        left: left.into(),
        right: right.into(),
    }
}

fn deleted(number: usize, text: &str) -> SideLine {
    line(number, text, TextChangeKind::Delete)
}

fn inserted(number: usize, text: &str) -> SideLine {
    line(number, text, TextChangeKind::Insert)
}

#[test]
fn unchanged_lines_sit_on_both_sides() {
    assert_eq!(
        rows("a\nb\n", "a\nb\n"),
        vec![same(1, 1, "a"), same(2, 2, "b")]
    );
}

#[test]
fn replaced_lines_are_paired_row_by_row() {
    assert_eq!(
        rows("a\nb\nc\nz\n", "a\nB\nC\nz\n"),
        vec![
            same(1, 1, "a"),
            row(deleted(2, "b"), inserted(2, "B")),
            row(deleted(3, "c"), inserted(3, "C")),
            same(4, 4, "z"),
        ]
    );
}

#[test]
fn unpaired_lines_leave_the_other_side_empty() {
    assert_eq!(
        rows("a\nb\nc\nz\n", "a\nB\nz\n"),
        vec![
            same(1, 1, "a"),
            row(deleted(2, "b"), inserted(2, "B")),
            row(deleted(3, "c"), None),
            same(4, 3, "z"),
        ]
    );
}

#[test]
fn inserted_lines_shift_right_hand_numbers() {
    assert_eq!(
        rows("a\nz\n", "a\nnew\nz\n"),
        vec![
            same(1, 1, "a"),
            row(None, inserted(2, "new")),
            same(2, 3, "z"),
        ]
    );
}

#[test]
fn line_endings_are_not_part_of_the_cell_text() {
    assert_eq!(
        rows("a\r\nb", "a\r\nc"),
        vec![same(1, 1, "a"), row(deleted(2, "b"), inserted(2, "c"))]
    );
}

#[test]
fn empty_inputs_have_no_rows() {
    assert_eq!(rows("", ""), Vec::<SideBySideRow>::new());
}
