use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use anstream::{AutoStream, ColorChoice};
use anstyle::{AnsiColor, Style};
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
    color: ColorChoice,
) -> anyhow::Result<ExitCode> {
    let kind = resolve_kind(left, right, kind, mode)?;
    let left_text =
        load_source(left).with_context(|| format!("loading left input {}", left.display()))?;
    let right_text =
        load_source(right).with_context(|| format!("loading right input {}", right.display()))?;

    tracing::debug!(?kind, left = %left.display(), right = %right.display(), "diffing");
    let result = diff(&left_text, &right_text, &DiffOptions::from(kind))
        .with_context(|| format!("comparing {} with {}", left.display(), right.display()))?;

    // Styles are always written; AutoStream strips them when color is off.
    let mut out = AutoStream::new(io::stdout().lock(), color);
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

const INSERT: Style = AnsiColor::Green.on_default();
const DELETE: Style = AnsiColor::Red.on_default();

fn text_style(kind: TextChangeKind) -> Style {
    match kind {
        TextChangeKind::Equal => Style::new(),
        TextChangeKind::Insert => INSERT,
        TextChangeKind::Delete => DELETE,
    }
}

/// Writes `text` in `style`, closing the style before every line ending so a
/// color never carries over into the next line (or into `less`, `grep`, ...).
fn paint(out: &mut impl Write, style: Style, text: &str) -> io::Result<()> {
    for line in text.split_inclusive('\n') {
        let (body, ending) = line
            .strip_suffix("\r\n")
            .map(|body| (body, "\r\n"))
            .or_else(|| line.strip_suffix('\n').map(|body| (body, "\n")))
            .unwrap_or((line, ""));
        if !body.is_empty() {
            write!(out, "{style}{body}{style:#}")?;
        }
        out.write_all(ending.as_bytes())?;
    }
    Ok(())
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
                write!(out, "{path}: ")?;
                paint(out, DELETE, &left.to_string())?;
                write!(out, " \u{2192} ")?;
                paint(out, INSERT, &right.to_string())?;
            }
            JsonChangeKind::Removed(left) => {
                paint(out, DELETE, &format!("{path}: removed {left}"))?;
            }
            JsonChangeKind::Added(right) => {
                paint(out, INSERT, &format!("{path}: added {right}"))?;
            }
        }
        writeln!(out)?;
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
            paint(
                out,
                text_style(change.kind),
                &format!("{sign}{}", change.value),
            )?;
            if !change.value.ends_with('\n') {
                writeln!(out)?;
            }
        }
    } else {
        // Inline markers for sub-line granularity: [-deleted-]{+inserted+}
        for change in &text.changes {
            let marked = match change.kind {
                TextChangeKind::Equal => change.value.clone(),
                TextChangeKind::Insert => format!("{{+{}+}}", change.value),
                TextChangeKind::Delete => format!("[-{}-]", change.value),
            };
            paint(out, text_style(change.kind), &marked)?;
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
