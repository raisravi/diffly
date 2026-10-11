use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Write as _};
use std::ops::Range;

use scraper::node::Node as ParsedNode;
use scraper::{ElementRef, Html};
use similar::{Algorithm, DiffOp};

use crate::{DiffBody, DiffResult, DiffStats};

/// What happened at a node path such as `html/body/div#main/li[2]`.
///
/// Paths name each element by tag, with `#id` when it has one and a 1-based
/// `[n]` when siblings share its tag. Text nodes end in `/text()` and
/// attributes in `/@name`. Removed and changed nodes use their left-hand
/// path, added nodes their right-hand one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HtmlChangeKind {
    /// An element or text node only on the right, as compact HTML.
    Added(String),
    /// An element or text node only on the left, as compact HTML.
    Removed(String),
    /// The text of a matched text node, whitespace collapsed (except in `<pre>`).
    TextChanged { left: String, right: String },
    /// An attribute only the right element has, with its value.
    AttributeAdded(String),
    /// An attribute only the left element has, with its value.
    AttributeRemoved(String),
    /// An attribute both elements have, with different values.
    AttributeChanged { left: String, right: String },
}

/// A node that differs between the two documents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlChange {
    pub path: String,
    pub kind: HtmlChangeKind,
}

/// Changes found by an HTML comparison, in document order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlDiff {
    pub changes: Vec<HtmlChange>,
}

/// Compares two documents by element structure, attributes and text. Parsing
/// never fails: malformed markup is repaired the way browsers do.
pub(crate) fn diff_html(left: &str, right: &str) -> DiffResult {
    let (left, right) = (parse(left), parse(right));
    let mut changes = Vec::new();
    compare_elements(&left, &right, "html", &mut changes);

    let mut stats = DiffStats::default();
    for change in &changes {
        match change.kind {
            HtmlChangeKind::Added(_) => stats.inserted += 1,
            HtmlChangeKind::Removed(_) => stats.deleted += 1,
            // Attributes are part of their element, which is matched on both sides.
            HtmlChangeKind::TextChanged { .. }
            | HtmlChangeKind::AttributeAdded(_)
            | HtmlChangeKind::AttributeRemoved(_)
            | HtmlChangeKind::AttributeChanged { .. } => stats.changed += 1,
        }
    }

    DiffResult {
        stats,
        body: DiffBody::Html(HtmlDiff { changes }),
    }
}

/// A document reduced to what the comparison looks at.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Node {
    Element(Element),
    /// Never empty: whitespace-only text is dropped.
    Text(String),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Element {
    name: String,
    /// Sorted by name, so attribute order doesn't matter.
    attrs: BTreeMap<String, String>,
    children: Vec<Node>,
}

/// Parses `input` as a whole document and returns its `<html>` element.
fn parse(input: &str) -> Element {
    reduce_element(Html::parse_document(input).root_element(), false)
}

/// `keep_whitespace` is set inside `<pre>` and `<textarea>`, where whitespace is content.
fn reduce_element(element: ElementRef<'_>, keep_whitespace: bool) -> Element {
    let name = element.value().name().to_owned();
    let keep_whitespace = keep_whitespace || matches!(name.as_str(), "pre" | "textarea");
    // A `<template>`'s contents sit under a document fragment; treat them as children.
    let nodes = element.children().flat_map(|child| {
        if matches!(child.value(), ParsedNode::Fragment) {
            child.children().collect()
        } else {
            vec![child]
        }
    });

    let mut children = Vec::new();
    // Text since the last element, so text split by a comment stays one node.
    let mut text = String::new();
    for node in nodes {
        match node.value() {
            ParsedNode::Text(part) => text.push_str(part),
            ParsedNode::Element(_) => {
                push_text(&mut text, keep_whitespace, &mut children);
                if let Some(child) = ElementRef::wrap(node) {
                    children.push(Node::Element(reduce_element(child, keep_whitespace)));
                }
            }
            // Comments, doctypes and processing instructions aren't content.
            _ => {}
        }
    }
    push_text(&mut text, keep_whitespace, &mut children);

    Element {
        name,
        attrs: element
            .value()
            .attrs()
            .map(|(name, value)| (name.to_owned(), value.to_owned()))
            .collect(),
        children,
    }
}

/// Moves the pending `text` into `children`, whitespace collapsed unless kept;
/// whitespace-only text is dropped.
fn push_text(text: &mut String, keep_whitespace: bool, children: &mut Vec<Node>) {
    let text = std::mem::take(text);
    let text = if keep_whitespace {
        text
    } else {
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    };
    if !text.is_empty() {
        children.push(Node::Text(text));
    }
}

/// Compares two matched elements: their attributes, then their children.
fn compare_elements(left: &Element, right: &Element, path: &str, changes: &mut Vec<HtmlChange>) {
    let names: BTreeSet<&String> = left.attrs.keys().chain(right.attrs.keys()).collect();
    for name in names {
        let kind = match (left.attrs.get(name), right.attrs.get(name)) {
            (Some(left), Some(right)) if left == right => continue,
            (Some(left), Some(right)) => HtmlChangeKind::AttributeChanged {
                left: left.clone(),
                right: right.clone(),
            },
            (Some(left), None) => HtmlChangeKind::AttributeRemoved(left.clone()),
            (None, Some(right)) => HtmlChangeKind::AttributeAdded(right.clone()),
            (None, None) => unreachable!("{name} comes from one of the two elements"),
        };
        changes.push(HtmlChange {
            path: format!("{path}/@{name}"),
            kind,
        });
    }
    compare_children(&left.children, &right.children, path, changes);
}

/// One side's children of a matched element, with the path of each.
struct Siblings<'a> {
    nodes: &'a [Node],
    paths: Vec<String>,
}

impl<'a> Siblings<'a> {
    fn new(nodes: &'a [Node], parent: &str) -> Self {
        Self {
            nodes,
            paths: child_paths(nodes, parent),
        }
    }
}

/// Matches identical subtrees first, so an inserted sibling doesn't shift the
/// rest, then compares each differing run with [`compare_run`].
fn compare_children(left: &[Node], right: &[Node], path: &str, changes: &mut Vec<HtmlChange>) {
    let (left, right) = (Siblings::new(left, path), Siblings::new(right, path));
    for op in similar::capture_diff_slices(Algorithm::Myers, left.nodes, right.nodes) {
        if !matches!(op, DiffOp::Equal { .. }) {
            compare_run(&left, &right, op.old_range(), op.new_range(), changes);
        }
    }
}

/// Within a run of differing siblings, compares nodes with the same
/// [`Node::key`] in place and reports the rest as removed or added.
fn compare_run(
    left: &Siblings<'_>,
    right: &Siblings<'_>,
    old: Range<usize>,
    new: Range<usize>,
    changes: &mut Vec<HtmlChange>,
) {
    let left_keys: Vec<_> = left.nodes[old.clone()].iter().map(Node::key).collect();
    let right_keys: Vec<_> = right.nodes[new.clone()].iter().map(Node::key).collect();
    for op in similar::capture_diff_slices(Algorithm::Myers, &left_keys, &right_keys) {
        let lefts = shift(op.old_range(), old.start);
        let rights = shift(op.new_range(), new.start);
        if matches!(op, DiffOp::Equal { .. }) {
            for (l, r) in lefts.zip(rights) {
                compare_nodes(&left.nodes[l], &right.nodes[r], &left.paths[l], changes);
            }
            continue;
        }
        for l in lefts {
            changes.push(HtmlChange {
                path: left.paths[l].clone(),
                kind: HtmlChangeKind::Removed(left.nodes[l].snippet()),
            });
        }
        for r in rights {
            changes.push(HtmlChange {
                path: right.paths[r].clone(),
                kind: HtmlChangeKind::Added(right.nodes[r].snippet()),
            });
        }
    }
}

fn shift(range: Range<usize>, by: usize) -> Range<usize> {
    range.start + by..range.end + by
}

/// Compares two nodes with the same [`Node::key`].
fn compare_nodes(left: &Node, right: &Node, path: &str, changes: &mut Vec<HtmlChange>) {
    match (left, right) {
        (Node::Element(left), Node::Element(right)) => {
            compare_elements(left, right, path, changes);
        }
        (Node::Text(left), Node::Text(right)) if left != right => changes.push(HtmlChange {
            path: path.to_owned(),
            kind: HtmlChangeKind::TextChanged {
                left: left.clone(),
                right: right.clone(),
            },
        }),
        _ => {}
    }
}

impl Node {
    /// What makes two nodes the "same" node with possibly different content:
    /// the tag and id for an element, and being text for a text node.
    fn key(&self) -> (Option<&str>, Option<&str>) {
        match self {
            Self::Element(element) => (Some(element.name.as_str()), element.id()),
            Self::Text(_) => (None, None),
        }
    }

    /// The node's name in a path: its tag, or `text()`.
    fn tag(&self) -> &str {
        match self {
            Self::Element(element) => &element.name,
            Self::Text(_) => "text()",
        }
    }

    /// How an added or removed node is shown: text quoted, an element as compact HTML.
    fn snippet(&self) -> String {
        match self {
            Self::Text(text) => format!("{text:?}"),
            Self::Element(_) => self.to_string(),
        }
    }
}

impl Element {
    fn id(&self) -> Option<&str> {
        self.attrs.get("id").map(String::as_str)
    }
}

/// The path of each of `nodes`, the children of the element at `parent`: the
/// tag with `#id` if it has one, else with `[n]` among siblings of that tag.
fn child_paths(nodes: &[Node], parent: &str) -> Vec<String> {
    let mut totals: BTreeMap<&str, usize> = BTreeMap::new();
    for node in nodes {
        *totals.entry(node.tag()).or_default() += 1;
    }
    let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
    nodes
        .iter()
        .map(|node| {
            let tag = node.tag();
            let nth = seen.entry(tag).or_default();
            *nth += 1;
            if let Node::Element(element) = node
                && let Some(id) = element.id()
            {
                format!("{parent}/{tag}#{id}")
            } else if totals[tag] > 1 {
                format!("{parent}/{tag}[{nth}]")
            } else {
                format!("{parent}/{tag}")
            }
        })
        .collect()
}

/// Elements with no content and no closing tag.
const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track",
    "wbr",
];

/// Compact HTML: `<p class="n"><b>bold</b></p>`.
impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text(text) => f.write_str(&escape(text, false)),
            Self::Element(element) => {
                write!(f, "<{}", element.name)?;
                for (name, value) in &element.attrs {
                    write!(f, " {name}=\"{}\"", escape(value, true))?;
                }
                f.write_char('>')?;
                if VOID_ELEMENTS.contains(&element.name.as_str()) {
                    return Ok(());
                }
                for child in &element.children {
                    write!(f, "{child}")?;
                }
                write!(f, "</{}>", element.name)
            }
        }
    }
}

/// Escapes `&`, `<` and `>`, plus `"` inside an attribute value.
fn escape(text: &str, in_attribute: bool) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' if in_attribute => escaped.push_str("&quot;"),
            c => escaped.push(c),
        }
    }
    escaped
}
