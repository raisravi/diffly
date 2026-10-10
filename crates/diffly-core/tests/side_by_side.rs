//! Side-by-side alignment of line diffs, driven with in-memory inputs.

#![allow(clippy::unwrap_used)]

use TextChangeKind::{Delete, Equal, Insert};
use diffly_core::{
    DiffBody, DiffMode, DiffOptions, SideBySideRow, SideLine, TextChange, TextChangeKind, diff,
};
use pretty_assertions::assert_eq;

fn rows(left: &str, right: &str) -> Vec<SideBySideRow> {
    rows_in(DiffMode::Line, left, right)
}

fn rows_in(mode: DiffMode, left: &str, right: &str) -> Vec<SideBySideRow> {
    match diff(left, right, &DiffOptions::text(mode)).unwrap().body {
        DiffBody::Text(text) => text.side_by_side(),
        DiffBody::Json(_) => panic!("expected a text diff"),
    }
}

fn line(number: usize, text: &str, kind: TextChangeKind) -> SideLine {
    SideLine {
        number,
        text: text.to_owned(),
        kind,
        inline: Vec::new(),
    }
}

/// Spans as `(kind, text)` pairs, e.g. `[(Equal, "the "), (Delete, "quick")]`.
fn with_inline(mut line: SideLine, spans: &[(TextChangeKind, &str)]) -> SideLine {
    line.inline = spans
        .iter()
        .map(|&(kind, value)| TextChange {
            kind,
            value: value.to_owned(),
        })
        .collect();
    line
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

#[test]
fn word_mode_marks_changed_words_inside_paired_lines() {
    assert_eq!(
        rows_in(DiffMode::Word, "a\nthe quick fox\n", "a\nthe slow fox\n"),
        vec![
            same(1, 1, "a"),
            row(
                with_inline(
                    deleted(2, "the quick fox"),
                    &[(Equal, "the "), (Delete, "quick"), (Equal, " fox")]
                ),
                with_inline(
                    inserted(2, "the slow fox"),
                    &[(Equal, "the "), (Insert, "slow"), (Equal, " fox")]
                ),
            ),
        ]
    );
}

#[test]
fn char_mode_marks_changed_chars_inside_paired_lines() {
    assert_eq!(
        rows_in(DiffMode::Char, "cat\n", "cut\n"),
        vec![row(
            with_inline(
                deleted(1, "cat"),
                &[(Equal, "c"), (Delete, "a"), (Equal, "t")]
            ),
            with_inline(
                inserted(1, "cut"),
                &[(Equal, "c"), (Insert, "u"), (Equal, "t")]
            ),
        )]
    );
}

#[test]
fn sub_line_modes_align_lines_like_line_mode() {
    let (left, right) = ("a\nb\nc\nz\n", "a\nB\nz\nnew\n");

    let lines_only = |rows: Vec<SideBySideRow>| -> Vec<SideBySideRow> {
        rows.into_iter()
            .map(|mut row| {
                for line in [&mut row.left, &mut row.right].into_iter().flatten() {
                    line.inline.clear();
                }
                row
            })
            .collect()
    };
    assert_eq!(
        lines_only(rows_in(DiffMode::Word, left, right)),
        rows(left, right)
    );
    assert_eq!(
        lines_only(rows_in(DiffMode::Char, left, right)),
        rows(left, right)
    );
}

#[test]
fn unpaired_lines_have_no_inline_spans() {
    let rows = rows_in(DiffMode::Word, "a\n", "a\nnew line\n");

    assert_eq!(rows[1], row(None, inserted(2, "new line")));
}
