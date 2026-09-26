#![allow(clippy::unwrap_used)]

use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn diffly() -> Command {
    Command::cargo_bin("diffly").unwrap()
}

fn write_pair(left: &str, right: &str) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("left.txt"), left).unwrap();
    fs::write(dir.path().join("right.txt"), right).unwrap();
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
