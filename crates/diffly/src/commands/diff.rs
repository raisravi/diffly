use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, bail};
use diffly_core::{
    DiffBody, DiffMode, DiffOptions, DiffResult, DiffStats, InputKind, JsonChangeKind, JsonDiff,
    TextChangeKind, TextDiff, diff, load_source,
};

use crate::cli::KindArg;

/// Exit codes follow `diff(1)`: 0 identical, 1 different, 2 error.
pub(crate) fn run(
    left: &Path,
    right: &Path,
    kind: Option<KindArg>,
    mode: Option<DiffMode>,
) -> anyhow::Result<ExitCode> {
    let kind = resolve_kind(left, right, kind, mode)?;
    let left_text =
        load_source(left).with_context(|| format!("loading left input {}", left.display()))?;
    let right_text =
        load_source(right).with_context(|| format!("loading right input {}", right.display()))?;

    tracing::debug!(?kind, left = %left.display(), right = %right.display(), "diffing");
    let result = diff(&left_text, &right_text, &DiffOptions::from(kind))
        .with_context(|| format!("comparing {} with {}", left.display(), right.display()))?;

    let mut out = io::stdout().lock();
    render(&mut out, &result).context("writing diff output")?;

    Ok(if result.is_identical() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

/// An explicit `--kind` wins over detection; `--mode` is only meaningful for text.
fn resolve_kind(
    left: &Path,
    right: &Path,
    kind: Option<KindArg>,
    mode: Option<DiffMode>,
) -> anyhow::Result<InputKind> {
    let text = InputKind::Text(mode.unwrap_or_default());
    let kind = match kind {
        Some(KindArg::Text) => text,
        Some(KindArg::Json) => InputKind::Json,
        None => InputKind::detect_pair(left, right).unwrap_or(text),
    };
    if mode.is_some() && !matches!(kind, InputKind::Text(_)) {
        bail!("--mode only applies to text diffs; add `--kind text` to compare these as text");
    }
    Ok(kind)
}

fn render(out: &mut impl Write, result: &DiffResult) -> io::Result<()> {
    match &result.body {
        DiffBody::Text(text) => render_text(out, text, result.stats),
        DiffBody::Json(json) => render_json(out, json, result.stats),
    }
}

fn render_json(out: &mut impl Write, json: &JsonDiff, stats: DiffStats) -> io::Result<()> {
    for change in &json.changes {
        let path = &change.path;
        match &change.kind {
            JsonChangeKind::Changed { left, right } => {
                writeln!(out, "{path}: {left} \u{2192} {right}")?;
            }
            JsonChangeKind::Removed(left) => writeln!(out, "{path}: removed {left}")?,
            JsonChangeKind::Added(right) => writeln!(out, "{path}: added {right}")?,
        }
    }
    writeln!(
        out,
        "{} added, {} removed, {} changed (json)",
        stats.inserted, stats.deleted, stats.changed
    )
}

fn render_text(out: &mut impl Write, text: &TextDiff, stats: DiffStats) -> io::Result<()> {
    if text.mode == DiffMode::Line {
        for change in &text.changes {
            let sign = match change.kind {
                TextChangeKind::Equal => ' ',
                TextChangeKind::Insert => '+',
                TextChangeKind::Delete => '-',
            };
            write!(out, "{sign}{}", change.value)?;
            if !change.value.ends_with('\n') {
                writeln!(out)?;
            }
        }
    } else {
        // Inline markers for sub-line granularity: [-deleted-]{+inserted+}
        for change in &text.changes {
            match change.kind {
                TextChangeKind::Equal => write!(out, "{}", change.value)?,
                TextChangeKind::Insert => write!(out, "{{+{}+}}", change.value)?,
                TextChangeKind::Delete => write!(out, "[-{}-]", change.value)?,
            }
        }
        if !text.changes.last().is_some_and(|c| c.value.ends_with('\n')) {
            writeln!(out)?;
        }
    }
    writeln!(
        out,
        "{} insertion(s), {} deletion(s) ({} mode)",
        stats.inserted, stats.deleted, text.mode
    )
}
