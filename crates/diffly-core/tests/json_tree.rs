//! The merged JSON tree behind the app's tree view, driven with in-memory inputs.

#![allow(clippy::unwrap_used)]

use diffly_core::{DiffBody, DiffOptions, JsonChangeKind, JsonNode, JsonNodeKind, diff};
use pretty_assertions::assert_eq;

fn tree(left: &str, right: &str) -> JsonNode {
    match diff(left, right, &DiffOptions::json()).unwrap().body {
        DiffBody::Json(json) => json.tree,
        _ => panic!("expected a JSON diff"),
    }
}

/// One line per node, depth-first: `"$.a changed: 1 → 2"`, with `*` after the
/// path of a branch that has changes somewhere below it.
fn outline(node: &JsonNode) -> Vec<String> {
    let mut lines = Vec::new();
    describe(node, &mut lines);
    lines
}

fn describe(node: &JsonNode, lines: &mut Vec<String>) {
    let star = if node.changed { "*" } else { "" };
    let (what, children) = match &node.kind {
        JsonNodeKind::Object(children) => (format!("{star} object"), children.as_slice()),
        JsonNodeKind::Array(children) => (format!("{star} array"), children.as_slice()),
        JsonNodeKind::Unchanged(value) => (format!(" same: {value}"), &[][..]),
        JsonNodeKind::Change(JsonChangeKind::Added(value)) => (format!(" added: {value}"), &[][..]),
        JsonNodeKind::Change(JsonChangeKind::Removed(value)) => {
            (format!(" removed: {value}"), &[][..])
        }
        JsonNodeKind::Change(JsonChangeKind::Changed { left, right }) => {
            (format!(" changed: {left} → {right}"), &[][..])
        }
    };
    lines.push(format!("{}{what}", node.path));
    for child in children {
        describe(child, lines);
    }
}

#[test]
fn unchanged_values_are_part_of_the_tree() {
    assert_eq!(
        outline(&tree(
            r#"{"a": 1, "b": [true]}"#,
            r#"{"a": 1, "b": [true]}"#
        )),
        vec!["$ object", "$.a same: 1", "$.b array", "$.b[0] same: true"]
    );
}

#[test]
fn a_nested_change_marks_every_branch_above_it() {
    assert_eq!(
        outline(&tree(
            r#"{"user": {"age": 30, "name": "ann"}, "meta": {"v": 1}}"#,
            r#"{"user": {"age": 31, "name": "ann"}, "meta": {"v": 1}}"#,
        )),
        vec![
            "$* object",
            "$.user* object",
            "$.user.age changed: 30 → 31",
            "$.user.name same: \"ann\"",
            "$.meta object",
            "$.meta.v same: 1",
        ]
    );
}

#[test]
fn added_keys_follow_the_left_document_order_like_changes() {
    let left = r#"{"a": 1, "gone": {"x": 1}, "c": 3}"#;
    let right = r#"{"new": 0, "c": 3, "a": 1}"#;

    assert_eq!(
        outline(&tree(left, right)),
        vec![
            "$* object",
            "$.a same: 1",
            "$.gone removed: {\"x\":1}",
            "$.c same: 3",
            "$.new added: 0",
        ]
    );
}

#[test]
fn arrays_are_matched_by_index() {
    assert_eq!(
        outline(&tree("[1, 2, 3]", "[1, 5]")),
        vec![
            "$* array",
            "$[0] same: 1",
            "$[1] changed: 2 → 5",
            "$[2] removed: 3",
        ]
    );
}

#[test]
fn a_type_change_is_one_changed_node() {
    assert_eq!(
        outline(&tree(r#"{"a": {"x": 1}}"#, r#"{"a": [1]}"#)),
        vec!["$* object", "$.a changed: {\"x\":1} → [1]"]
    );
}

#[test]
fn a_scalar_root_is_a_single_node() {
    assert_eq!(outline(&tree("1", "1")), vec!["$ same: 1"]);
    assert_eq!(outline(&tree("1", "2")), vec!["$ changed: 1 → 2"]);
}

#[test]
fn numbers_spelled_differently_are_unchanged() {
    assert_eq!(
        outline(&tree("[1.0]", "[1]")),
        vec!["$ array", "$[0] same: 1.0"]
    );
}
