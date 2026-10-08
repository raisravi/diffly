#![allow(clippy::unwrap_used)]

use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn diffly() -> Command {
    Command::cargo_bin("diffly").unwrap()
}

fn write_pair(left: &str, right: &str) -> TempDir {
    write_files(&[("left.txt", left), ("right.txt", right)])
}

fn write_files(files: &[(&str, &str)]) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (name, contents) in files {
        fs::write(dir.path().join(name), contents).unwrap();
    }
    dir
}

#[test]
fn help_lists_subcommands() {
    diffly()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("diff").and(predicate::str::contains("gui")));
}

#[test]
fn different_files_exit_1() {
    let dir = write_pair("a\nb\n", "a\nc\n");
    diffly()
        .current_dir(dir.path())
        .args(["diff", "left.txt", "right.txt"])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("-b\n+c\n"))
        .stdout(predicate::str::contains("1 insertion(s), 1 deletion(s)"));
}

#[test]
fn word_mode_uses_inline_markers() {
    let dir = write_pair("hello world\n", "hello there\n");
    diffly()
        .current_dir(dir.path())
        .args(["diff", "left.txt", "right.txt", "--mode", "word"])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("[-world-]{+there+}"));
}

#[test]
fn identical_files_exit_0() {
    let dir = write_pair("same\n", "same\n");
    diffly()
        .current_dir(dir.path())
        .args(["diff", "left.txt", "right.txt"])
        .assert()
        .success();
}

#[test]
fn missing_file_reports_context() {
    let dir = write_pair("x\n", "x\n");
    diffly()
        .current_dir(dir.path())
        .args(["diff", "nope.txt", "right.txt"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("loading left input nope.txt"));
}

#[test]
fn json_files_are_diffed_structurally() {
    let dir = write_files(&[
        ("a.json", r#"{"age": 30, "email": "a@x", "tags": ["a"]}"#),
        ("b.json", r#"{"age": 31, "tags": ["a", "admin"]}"#),
    ]);
    diffly()
        .current_dir(dir.path())
        .args(["diff", "a.json", "b.json"])
        .assert()
        .code(1)
        .stdout(
            "$.age: 30 \u{2192} 31\n\
             $.email: removed \"a@x\"\n\
             $.tags[1]: added \"admin\"\n\
             1 added, 1 removed, 1 changed (json)\n",
        );
}

#[test]
fn reformatted_equal_json_is_identical() {
    let dir = write_files(&[
        ("a.json", r#"{"a": 1, "b": [1, 2]}"#),
        ("b.json", "{\n  \"b\": [1, 2],\n  \"a\": 1\n}\n"),
    ]);
    diffly()
        .current_dir(dir.path())
        .args(["diff", "a.json", "b.json"])
        .assert()
        .success()
        .stdout("0 added, 0 removed, 0 changed (json)\n");
}

#[test]
fn kind_text_overrides_json_detection() {
    let dir = write_files(&[("a.json", "{\"a\": 1}\n"), ("b.json", "{\"a\": 2}\n")]);
    diffly()
        .current_dir(dir.path())
        .args(["diff", "a.json", "b.json", "--kind", "text"])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("-{\"a\": 1}\n+{\"a\": 2}\n"));
}

#[test]
fn kind_json_applies_to_files_without_json_extension() {
    let dir = write_files(&[("a.txt", r#"{"a": 1}"#), ("b.txt", r#"{"a": 2}"#)]);
    diffly()
        .current_dir(dir.path())
        .args(["diff", "a.txt", "b.txt", "--kind", "json"])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("$.a: 1 \u{2192} 2"));
}

#[test]
fn json_is_only_detected_when_both_files_are_json() {
    let dir = write_files(&[("a.json", r#"{"a": 1}"#), ("b.txt", r#"{"a": 1}"#)]);
    diffly()
        .current_dir(dir.path())
        .args(["diff", "a.json", "b.txt"])
        .assert()
        .success()
        .stdout(predicate::str::contains("(line mode)"));
}

#[test]
fn invalid_json_reports_side_file_and_position() {
    let dir = write_files(&[("a.json", "{}"), ("b.json", "{\n  \"a\": 1,\n}")]);
    diffly()
        .current_dir(dir.path())
        .args(["diff", "a.json", "b.json"])
        .assert()
        .code(2)
        .stdout("")
        .stderr(
            predicate::str::contains("comparing a.json with b.json")
                .and(predicate::str::contains("right input is not valid JSON"))
                .and(predicate::str::contains("line 3 column 1")),
        );
}

#[test]
fn mode_is_rejected_for_json_diffs() {
    let dir = write_files(&[("a.json", "{}"), ("b.json", "{}")]);
    diffly()
        .current_dir(dir.path())
        .args(["diff", "a.json", "b.json", "--mode", "word"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains(
            "--mode only applies to text diffs",
        ));
}
