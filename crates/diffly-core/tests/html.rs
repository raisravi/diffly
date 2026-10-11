//! HTML structural diffs through the public entry point, driven with in-memory inputs.

#![allow(clippy::unwrap_used)]

use diffly_core::{DiffBody, DiffOptions, DiffResult, DiffStats, HtmlChangeKind, diff};
use pretty_assertions::assert_eq;

fn html_diff(left: &str, right: &str) -> DiffResult {
    diff(left, right, &DiffOptions::html()).unwrap()
}

/// One line per change: `"html/body/p/text(): \"a\" → \"b\""`.
fn changes(left: &str, right: &str) -> Vec<String> {
    let DiffBody::Html(html) = html_diff(left, right).body else {
        panic!("expected an HTML diff");
    };
    html.changes
        .iter()
        .map(|change| {
            let what = match &change.kind {
                HtmlChangeKind::Added(node) => format!("added {node}"),
                HtmlChangeKind::Removed(node) => format!("removed {node}"),
                HtmlChangeKind::TextChanged { left, right }
                | HtmlChangeKind::AttributeChanged { left, right } => {
                    format!("{left:?} → {right:?}")
                }
                HtmlChangeKind::AttributeAdded(value) => format!("added {value:?}"),
                HtmlChangeKind::AttributeRemoved(value) => format!("removed {value:?}"),
            };
            format!("{}: {what}", change.path)
        })
        .collect()
}

#[test]
fn reformatting_and_attribute_order_are_ignored() {
    let left = r#"<html><body><div class="a" id="m"><p>Hello   world</p></div></body></html>"#;
    let right = r#"
        <html>
          <body>
            <div id="m"
                 class="a">
              <p>
                Hello
                world
              </p>
            </div>
          </body>
        </html>
    "#;

    assert_eq!(changes(left, right), Vec::<String>::new());
    assert!(html_diff(left, right).is_identical());
}

#[test]
fn comments_and_doctype_are_ignored() {
    assert_eq!(
        changes("<!DOCTYPE html><p>x</p><!-- note -->", "<p>x</p>"),
        Vec::<String>::new()
    );
}

#[test]
fn a_text_change_is_reported_at_its_element() {
    assert_eq!(
        changes("<p>Hello</p>", "<p>Hi</p>"),
        vec![r#"html/body/p/text(): "Hello" → "Hi""#]
    );
}

#[test]
fn attributes_are_compared_by_name() {
    assert_eq!(
        changes(
            r#"<a href="a" title="t">x</a>"#,
            r#"<a target="_blank" href="b">x</a>"#
        ),
        vec![
            r#"html/body/a/@href: "a" → "b""#,
            r#"html/body/a/@target: added "_blank""#,
            r#"html/body/a/@title: removed "t""#,
        ]
    );
}

#[test]
fn nested_elements_are_added_and_removed_by_path() {
    assert_eq!(
        changes(
            r#"<div id="main"><ul><li>a</li><li>b</li></ul></div>"#,
            r#"<div id="main"><ul><li>a</li></ul><p class="n"><b>bold</b></p></div>"#,
        ),
        vec![
            "html/body/div#main/ul/li[2]: removed <li>b</li>",
            r#"html/body/div#main/p: added <p class="n"><b>bold</b></p>"#,
        ]
    );
}

#[test]
fn inserting_a_sibling_reports_only_that_sibling() {
    assert_eq!(
        changes(
            "<ul><li>a</li><li>b</li></ul>",
            "<ul><li>new</li><li>a</li><li>b</li></ul>"
        ),
        vec!["html/body/ul/li[1]: added <li>new</li>"]
    );
}

#[test]
fn malformed_html_is_parsed_like_a_browser() {
    assert_eq!(
        changes("<p>one<p>two", "<p>one</p><p>two</p>"),
        Vec::<String>::new()
    );
    assert_eq!(
        changes("<div><span>x</div>", "<div><span>x</span></div>"),
        Vec::<String>::new()
    );
}

#[test]
fn whitespace_inside_pre_is_content() {
    assert_eq!(
        changes("<pre>a  b</pre>", "<pre>a b</pre>"),
        vec![r#"html/body/pre/text(): "a  b" → "a b""#]
    );
}

#[test]
fn void_elements_are_shown_without_a_closing_tag() {
    assert_eq!(
        changes("<p>x</p>", r#"<p>x</p><img src="a.png">"#),
        vec![r#"html/body/img: added <img src="a.png">"#]
    );
}

#[test]
fn stats_count_added_removed_and_changed_nodes() {
    let result = html_diff(
        r#"<p class="a">one</p><p>two</p>"#,
        r#"<p class="b">uno</p><h1>new</h1>"#,
    );

    assert_eq!(
        result.stats,
        DiffStats {
            inserted: 1,
            deleted: 1,
            changed: 2,
        }
    );
}

#[test]
fn template_contents_are_compared() {
    assert_eq!(
        changes(
            "<template><p>a</p></template>",
            "<template><p>b</p></template>"
        ),
        vec![r#"html/head/template/p/text(): "a" → "b""#]
    );
}

#[test]
fn text_split_by_a_comment_is_one_text() {
    assert_eq!(
        changes("<p>a<!-- x -->b</p>", "<p>ab</p>"),
        Vec::<String>::new()
    );
}

#[test]
fn indexes_count_every_sibling_with_the_tag() {
    assert_eq!(
        changes(
            r#"<div id="m">0</div><div>1</div><div>2</div>"#,
            r#"<div id="m">0</div><div>1</div><div>two</div>"#,
        ),
        vec![r#"html/body/div[3]/text(): "2" → "two""#]
    );
}

#[test]
fn added_text_is_quoted_and_markup_in_snippets_is_escaped() {
    assert_eq!(
        changes("<p>x</p>", "<p>x</p>a &lt; b<i>&amp;</i>"),
        vec![
            r#"html/body/text(): added "a < b""#,
            "html/body/i: added <i>&amp;</i>",
        ]
    );
}
