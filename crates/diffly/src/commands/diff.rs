use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use anyhow::Context;
use diffly_core::{ChangeKind, DiffMode, DiffResult, diff_text, load_source};

/// Exit codes follow `diff(1)`: 0 identical, 1 different, 2 error.
pub(crate) fn run(left: &Path, right: &Path, mode: DiffMode) -> anyhow::Result<ExitCode> {
    let old =
        load_source(left).with_context(|| format!("loading left input {}", left.display()))?;
    let new =
        load_source(right).with_context(|| format!("loading right input {}", right.display()))?;

    tracing::debug!(%mode, left = %left.display(), right = %right.display(), "diffing");
    let result = diff_text(&old, &new, mode);

    let mut out = io::stdout().lock();
    render(&mut out, &result).context("writing diff output")?;

    Ok(if result.is_identical() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

fn render(out: &mut impl Write, result: &DiffResult) -> io::Result<()> {
    if result.mode == DiffMode::Line {
        for change in &result.changes {
            let sign = match change.kind {
                ChangeKind::Equal => ' ',
                ChangeKind::Insert => '+',
                ChangeKind::Delete => '-',
            };
            write!(out, "{sign}{}", change.value)?;
            if !change.value.ends_with('\n') {
                writeln!(out)?;
            }
        }
    } else {
        // Inline markers for sub-line granularity: [-deleted-]{+inserted+}
        for change in &result.changes {
            match change.kind {
                ChangeKind::Equal => write!(out, "{}", change.value)?,
                ChangeKind::Insert => write!(out, "{{+{}+}}", change.value)?,
                ChangeKind::Delete => write!(out, "[-{}-]", change.value)?,
            }
        }
        if !result
            .changes
            .last()
            .is_some_and(|c| c.value.ends_with('\n'))
        {
            writeln!(out)?;
        }
    }
    writeln!(
        out,
        "{} insertion(s), {} deletion(s) ({} mode)",
        result.stats.inserted, result.stats.deleted, result.mode
    )
}
