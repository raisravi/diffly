//! Text diffs through the public entry point, driven with in-memory inputs.

use diffly_core::{
    DiffBody, DiffMode, DiffOptions, DiffResult, DiffStats, InputKind, TextChangeKind, diff,
};
use pretty_assertions::assert_eq;

fn changes(result: &DiffResult) -> Vec<(TextChangeKind, &str)> {
    match &result.body {
        DiffBody::Text(text) => text
            .changes
            .iter()
            .map(|c| (c.kind, c.value.as_str()))
            .collect(),
        DiffBody::Json(_) => panic!("expected a text diff"),
    }
}

#[test]
fn line_diff_reports_changed_lines() {
    let result = diff("a\nb\n", "a\nc\n", &DiffOptions::text(DiffMode::Line)).unwrap();

    assert_eq!(
        changes(&result),
        vec![
            (TextChangeKind::Equal, "a\n"),
            (TextChangeKind::Delete, "b\n"),
            (TextChangeKind::Insert, "c\n"),
        ]
    );
    assert_eq!(
        result.stats,
        DiffStats {
            inserted: 1,
            deleted: 1,
            changed: 0,
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
            (TextChangeKind::Equal, "hello"),
            (TextChangeKind::Equal, " "),
            (TextChangeKind::Delete, "world"),
            (TextChangeKind::Insert, "there"),
        ]
    );
    assert_eq!(
        result.stats,
        DiffStats {
            inserted: 1,
            deleted: 1,
            changed: 0,
        }
    );
}

#[test]
fn char_diff_reports_changed_chars() {
    let result = diff("cat", "cut", &DiffOptions::text(DiffMode::Char)).unwrap();

    assert_eq!(
        changes(&result),
        vec![
            (TextChangeKind::Equal, "c"),
            (TextChangeKind::Delete, "a"),
            (TextChangeKind::Insert, "u"),
            (TextChangeKind::Equal, "t"),
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
    assert_eq!(changes(&result), Vec::<(TextChangeKind, &str)>::new());
}

#[test]
fn default_options_are_a_line_diff_of_text() {
    let options = DiffOptions::default();

    assert_eq!(options.kind, InputKind::Text(DiffMode::Line));
}
