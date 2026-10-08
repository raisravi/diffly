//! JSON structural diffs through the public entry point, driven with in-memory inputs.

#![allow(clippy::unwrap_used)]

use std::error::Error as _;

use diffly_core::{
    DiffBody, DiffError, DiffOptions, DiffResult, DiffStats, JsonChangeKind, Side, diff,
};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};

type Row = (String, JsonChangeKind);

fn json_diff(left: &str, right: &str) -> DiffResult {
    diff(left, right, &DiffOptions::json()).unwrap()
}

fn rows(result: &DiffResult) -> Vec<Row> {
    match &result.body {
        DiffBody::Json(json) => json
            .changes
            .iter()
            .map(|c| (c.path.to_string(), c.kind.clone()))
            .collect(),
        DiffBody::Text(_) => panic!("expected a JSON diff"),
    }
}

fn changed(path: &str, left: Value, right: Value) -> Row {
    (path.into(), JsonChangeKind::Changed { left, right })
}

#[test]
fn changed_value_is_reported_at_its_path() {
    let result = json_diff(r#"{"age": 30}"#, r#"{"age": 31}"#);

    assert_eq!(rows(&result), vec![changed("$.age", json!(30), json!(31))]);
}

fn added(path: &str, right: Value) -> Row {
    (path.into(), JsonChangeKind::Added(right))
}

fn removed(path: &str, left: Value) -> Row {
    (path.into(), JsonChangeKind::Removed(left))
}

#[test]
fn added_and_removed_keys_are_reported() {
    let result = json_diff(
        r#"{"name": "ada", "email": "ada@example.com"}"#,
        r#"{"name": "ada", "phone": "555"}"#,
    );

    assert_eq!(
        rows(&result),
        vec![
            removed("$.email", json!("ada@example.com")),
            added("$.phone", json!("555")),
        ]
    );
}

#[test]
fn arrays_compare_by_index_inside_nested_objects() {
    let result = json_diff(
        r#"{"user": {"tags": ["a", "b"]}}"#,
        r#"{"user": {"tags": ["a", "x", "admin"]}}"#,
    );

    assert_eq!(
        rows(&result),
        vec![
            changed("$.user.tags[1]", json!("b"), json!("x")),
            added("$.user.tags[2]", json!("admin")),
        ]
    );
}

#[test]
fn shorter_array_reports_trailing_elements_as_removed() {
    let result = json_diff("[1, 2, 3]", "[1, 3]");

    assert_eq!(
        rows(&result),
        vec![
            changed("$[1]", json!(2), json!(3)),
            removed("$[2]", json!(3))
        ]
    );
}

#[test]
fn type_change_is_a_change_of_the_whole_value() {
    let result = json_diff(
        r#"{"id": 7, "meta": {"a": 1}}"#,
        r#"{"id": "7", "meta": [1]}"#,
    );

    assert_eq!(
        rows(&result),
        vec![
            changed("$.id", json!(7), json!("7")),
            changed("$.meta", json!({"a": 1}), json!([1])),
        ]
    );
}

#[test]
fn null_is_a_value_distinct_from_a_missing_key() {
    let result = json_diff(r#"{"a": null, "b": 1}"#, r#"{"a": 0, "b": 1, "c": null}"#);

    assert_eq!(
        rows(&result),
        vec![
            changed("$.a", Value::Null, json!(0)),
            added("$.c", Value::Null),
        ]
    );
}

#[test]
fn reordered_and_reformatted_documents_are_identical() {
    let result = json_diff(
        r#"{"a": 1, "b": {"x": [1, 2], "y": null}}"#,
        "{\n  \"b\": {\n    \"y\": null,\n    \"x\": [ 1, 2 ]\n  },\n  \"a\": 1\n}\n",
    );

    assert_eq!(rows(&result), Vec::<Row>::new());
    assert!(result.is_identical());
}

#[test]
fn stats_count_added_removed_and_changed_values() {
    let result = json_diff(
        r#"{"keep": 1, "edit": 1, "drop": 1, "list": [1, 2]}"#,
        r#"{"keep": 1, "edit": 2, "list": [1, 2, 3, 4]}"#,
    );

    assert_eq!(
        result.stats,
        DiffStats {
            inserted: 2,
            deleted: 1,
            changed: 1,
        }
    );
    assert!(!result.is_identical());
}

#[test]
fn invalid_json_names_the_side_and_position() {
    let err = diff("{}", "{\n  \"a\": 1,\n}", &DiffOptions::json()).unwrap_err();

    assert!(matches!(
        err,
        DiffError::InvalidJson {
            side: Side::Right,
            ..
        }
    ));
    assert_eq!(err.to_string(), "right input is not valid JSON");
    let cause = err.source().unwrap().to_string();
    assert!(cause.contains("line 3 column 1"), "{cause}");
}

#[test]
fn invalid_left_input_is_reported_as_left() {
    let err = diff("not json", "{}", &DiffOptions::json()).unwrap_err();

    assert!(matches!(
        err,
        DiffError::InvalidJson {
            side: Side::Left,
            ..
        }
    ));
}

#[test]
fn keys_that_are_not_identifiers_are_quoted_in_paths() {
    let result = json_diff(
        r#"{"a.b": 1, "first name": 1, "": 1, "say \"hi\"": 1, "_ok1": 1}"#,
        r#"{"a.b": 2, "first name": 2, "": 2, "say \"hi\"": 2, "_ok1": 2}"#,
    );

    let paths: Vec<String> = rows(&result).into_iter().map(|row| row.0).collect();
    assert_eq!(
        paths,
        vec![
            r#"$["a.b"]"#,
            r#"$["first name"]"#,
            r#"$[""]"#,
            r#"$["say \"hi\""]"#,
            "$._ok1",
        ]
    );
}

#[test]
fn numbers_compare_by_value_not_spelling() {
    let result = json_diff("[1, 1e2, -0, 2.50]", "[1.0, 100, 0, 2.5]");

    assert_eq!(rows(&result), Vec::<Row>::new());
}

#[test]
fn numbers_are_shown_as_written_and_large_integers_keep_precision() {
    let result = json_diff("[1e2, 18446744073709551616]", "[101, 18446744073709551617]");

    let shown: Vec<(String, String)> = match &result.body {
        DiffBody::Json(json) => json
            .changes
            .iter()
            .map(|c| match &c.kind {
                JsonChangeKind::Changed { left, right } => (left.to_string(), right.to_string()),
                other => panic!("expected a change, got {other:?}"),
            })
            .collect(),
        DiffBody::Text(_) => panic!("expected a JSON diff"),
    };
    assert_eq!(
        shown,
        vec![
            // serde_json keeps the number's digits but normalises the exponent sign.
            ("1e+2".to_owned(), "101".to_owned()),
            (
                "18446744073709551616".to_owned(),
                "18446744073709551617".to_owned()
            ),
        ]
    );
}

#[test]
fn leading_byte_order_mark_is_ignored() {
    let result = json_diff("\u{feff}{\"a\": 1}", "{\"a\": 1}");

    assert!(result.is_identical());
}
