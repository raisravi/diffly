//! Choosing an input kind from a file name.

use std::path::Path;

use diffly_core::InputKind;

#[test]
fn json_extension_is_detected_case_insensitively() {
    assert_eq!(
        InputKind::detect(Path::new("a.json")),
        Some(InputKind::Json)
    );
    assert_eq!(
        InputKind::detect(Path::new("dir/B.JSON")),
        Some(InputKind::Json)
    );
}

#[test]
fn unknown_or_missing_extension_is_not_detected() {
    assert_eq!(InputKind::detect(Path::new("notes.txt")), None);
    assert_eq!(InputKind::detect(Path::new("Makefile")), None);
    assert_eq!(InputKind::detect(Path::new("json")), None);
}

#[test]
fn a_pair_is_detected_only_when_both_sides_agree() {
    let json = Path::new("a.json");
    let text = Path::new("b.txt");

    assert_eq!(
        InputKind::detect_pair(json, Path::new("b.JSON")),
        Some(InputKind::Json)
    );
    assert_eq!(InputKind::detect_pair(json, text), None);
    assert_eq!(InputKind::detect_pair(text, json), None);
    assert_eq!(InputKind::detect_pair(text, text), None);
}
