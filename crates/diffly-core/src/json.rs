use std::fmt;

use serde_json::{Number, Value};

use crate::{DiffBody, DiffError, DiffResult, DiffStats, Result, Side};

/// One step from a JSON value into a child.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathSegment {
    Key(String),
    Index(usize),
}

/// Location of a value inside a JSON document, displayed as `$.user.tags[2]`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct JsonPath(pub Vec<PathSegment>);

impl fmt::Display for JsonPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("$")?;
        for segment in &self.0 {
            match segment {
                PathSegment::Key(key) if is_identifier(key) => write!(f, ".{key}")?,
                // A JSON string literal is a valid, unambiguous bracket key.
                PathSegment::Key(key) => write!(f, "[{}]", Value::from(key.as_str()))?,
                PathSegment::Index(index) => write!(f, "[{index}]")?,
            }
        }
        Ok(())
    }
}

fn is_identifier(key: &str) -> bool {
    let mut chars = key.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// What happened to the value at a path. Numbers keep the digits they had in
/// the input (`1e2` displays as `1e+2`, not `100.0`), and large integers stay exact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsonChangeKind {
    Added(Value),
    Removed(Value),
    Changed { left: Value, right: Value },
}

/// A value that differs between the two documents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonChange {
    pub path: JsonPath,
    pub kind: JsonChangeKind,
}

/// Changes found by a JSON comparison, in document order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonDiff {
    pub changes: Vec<JsonChange>,
}

pub(crate) fn diff_json(left: &str, right: &str) -> Result<DiffResult> {
    let left = parse(left, Side::Left)?;
    let right = parse(right, Side::Right)?;

    let mut walker = Walker::default();
    walker.compare(&left, &right);

    let mut stats = DiffStats::default();
    for change in &walker.changes {
        match change.kind {
            JsonChangeKind::Added(_) => stats.inserted += 1,
            JsonChangeKind::Removed(_) => stats.deleted += 1,
            JsonChangeKind::Changed { .. } => stats.changed += 1,
        }
    }

    Ok(DiffResult {
        stats,
        body: DiffBody::Json(JsonDiff {
            changes: walker.changes,
        }),
    })
}

fn parse(input: &str, side: Side) -> Result<Value> {
    // Editors on some platforms prefix files with a byte order mark.
    let input = input.strip_prefix('\u{feff}').unwrap_or(input);
    serde_json::from_str(input).map_err(|source| DiffError::InvalidJson { side, source })
}

/// Numbers are equal when they denote the same value, however they are spelled
/// (`1`, `1.0`, `1e0`). Integers compare exactly up to `i128`; anything else as `f64`.
fn numbers_equal(left: &Number, right: &Number) -> bool {
    let (left, right) = (left.to_string(), right.to_string());
    if let (Ok(left), Ok(right)) = (left.parse::<i128>(), right.parse::<i128>()) {
        return left == right;
    }
    match (left.parse::<f64>(), right.parse::<f64>()) {
        #[allow(clippy::float_cmp)] // exact equality is the point: same denoted value
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

#[derive(Default)]
struct Walker {
    path: Vec<PathSegment>,
    changes: Vec<JsonChange>,
}

impl Walker {
    fn compare(&mut self, left: &Value, right: &Value) {
        match (left, right) {
            (Value::Object(left), Value::Object(right)) => {
                for (key, left_value) in left {
                    self.at(PathSegment::Key(key.clone()), |walker| {
                        match right.get(key) {
                            Some(right_value) => walker.compare(left_value, right_value),
                            None => walker.record(JsonChangeKind::Removed(left_value.clone())),
                        }
                    });
                }
                for (key, right_value) in right {
                    if !left.contains_key(key) {
                        self.at(PathSegment::Key(key.clone()), |walker| {
                            walker.record(JsonChangeKind::Added(right_value.clone()));
                        });
                    }
                }
            }
            (Value::Array(left), Value::Array(right)) => {
                for index in 0..left.len().max(right.len()) {
                    self.at(PathSegment::Index(index), |walker| {
                        match (left.get(index), right.get(index)) {
                            (Some(left), Some(right)) => walker.compare(left, right),
                            (Some(left), None) => {
                                walker.record(JsonChangeKind::Removed(left.clone()));
                            }
                            (None, Some(right)) => {
                                walker.record(JsonChangeKind::Added(right.clone()));
                            }
                            (None, None) => {
                                unreachable!("index is below the longer array's length")
                            }
                        }
                    });
                }
            }
            // `Value`'s own equality compares numbers by spelling.
            (Value::Number(l), Value::Number(r)) if numbers_equal(l, r) => {}
            (Value::Number(_), Value::Number(_)) => self.record(JsonChangeKind::Changed {
                left: left.clone(),
                right: right.clone(),
            }),
            _ if left == right => {}
            _ => self.record(JsonChangeKind::Changed {
                left: left.clone(),
                right: right.clone(),
            }),
        }
    }

    /// Runs `f` with `segment` appended to the current path.
    fn at(&mut self, segment: PathSegment, f: impl FnOnce(&mut Self)) {
        self.path.push(segment);
        f(self);
        self.path.pop();
    }

    fn record(&mut self, kind: JsonChangeKind) {
        self.changes.push(JsonChange {
            path: JsonPath(self.path.clone()),
            kind,
        });
    }
}
