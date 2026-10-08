//! Behaviour of the public diff entry point, driven with in-memory inputs.

use diffly_core::{ChangeKind, DiffMode, DiffOptions, DiffResult, DiffStats, InputKind, diff};
use pretty_assertions::assert_eq;

fn changes(result: &DiffResult) -> Vec<(ChangeKind, &str)> {
    result
        .changes
        .iter()
        .map(|c| (c.kind, c.value.as_str()))
        .collect()
}

#[test]
fn line_diff_reports_changed_lines() {
    let result = diff("a\nb\n", "a\nc\n", &DiffOptions::text(DiffMode::Line)).unwrap();

    assert_eq!(
        changes(&result),
        vec![
            (ChangeKind::Equal, "a\n"),
            (ChangeKind::Delete, "b\n"),
            (ChangeKind::Insert, "c\n"),
        ]
    );
    assert_eq!(
        result.stats,
        DiffStats {
            inserted: 1,
            deleted: 1
        }
    );
}

#[test]
fn word_diff_reports_changed_words() {
    let result = diff(
        "hello world",
        "hello there",
        &DiffOptions::text(DiffMode::Word),
    )
    .unwrap();

    assert_eq!(
        changes(&result),
        vec![
            (ChangeKind::Equal, "hello"),
            (ChangeKind::Equal, " "),
            (ChangeKind::Delete, "world"),
            (ChangeKind::Insert, "there"),
        ]
    );
    assert_eq!(
        result.stats,
        DiffStats {
            inserted: 1,
            deleted: 1
        }
    );
}

#[test]
fn char_diff_reports_changed_chars() {
    let result = diff("cat", "cut", &DiffOptions::text(DiffMode::Char)).unwrap();

    assert_eq!(
        changes(&result),
        vec![
            (ChangeKind::Equal, "c"),
            (ChangeKind::Delete, "a"),
            (ChangeKind::Insert, "u"),
            (ChangeKind::Equal, "t"),
        ]
    );
}

#[test]
fn identical_inputs_are_identical() {
    let result = diff("a\nb\n", "a\nb\n", &DiffOptions::default()).unwrap();

    assert!(result.is_identical());
    assert_eq!(result.stats, DiffStats::default());
}

#[test]
fn empty_inputs_produce_no_changes() {
    let result = diff("", "", &DiffOptions::text(DiffMode::Char)).unwrap();

    assert!(result.is_identical());
    assert!(result.changes.is_empty());
}

#[test]
fn default_options_are_a_line_diff_of_text() {
    let options = DiffOptions::default();

    assert_eq!(options.kind, InputKind::Text);
    assert_eq!(options.mode, DiffMode::Line);
}
