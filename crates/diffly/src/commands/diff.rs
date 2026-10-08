use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use anyhow::Context;
use diffly_core::{
    DiffBody, DiffMode, DiffOptions, DiffResult, DiffStats, TextChangeKind, TextDiff, diff,
    load_source,
};

/// Exit codes follow `diff(1)`: 0 identical, 1 different, 2 error.
pub(crate) fn run(left: &Path, right: &Path, mode: DiffMode) -> anyhow::Result<ExitCode> {
    let left_text =
        load_source(left).with_context(|| format!("loading left input {}", left.display()))?;
    let right_text =
        load_source(right).with_context(|| format!("loading right input {}", right.display()))?;

    tracing::debug!(%mode, left = %left.display(), right = %right.display(), "diffing");
    let result =
        diff(&left_text, &right_text, &DiffOptions::text(mode)).context("comparing inputs")?;

    let mut out = io::stdout().lock();
    render(&mut out, &result).context("writing diff output")?;

    Ok(if result.is_identical() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

fn render(out: &mut impl Write, result: &DiffResult) -> io::Result<()> {
    match &result.body {
        DiffBody::Text(text) => render_text(out, text, result.stats),
    }
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
