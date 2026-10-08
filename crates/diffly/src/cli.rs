use std::path::PathBuf;

use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use diffly_core::DiffMode;

/// Unified local diff tool for JSON, HTML, source code and files.
#[derive(Debug, Parser)]
#[command(version, about, propagate_version = true)]
pub(crate) struct Cli {
    /// Increase log verbosity (-v, -vv, -vvv). `RUST_LOG` takes precedence.
    #[arg(short, long, action = ArgAction::Count, global = true)]
    pub(crate) verbose: u8,

    /// When to color output. `auto` colors only on a terminal and honours `NO_COLOR`.
    #[arg(long, value_enum, default_value_t = ColorArg::Auto, global = true)]
    pub(crate) color: ColorArg,

    #[command(subcommand)]
    pub(crate) command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Print the difference between two files to the terminal.
    Diff {
        /// Original file.
        left: PathBuf,
        /// Modified file.
        right: PathBuf,
        /// How to compare the inputs. By default both files must end in `.json`
        /// to be compared as JSON; anything else is compared as text.
        #[arg(short, long, value_enum)]
        kind: Option<KindArg>,
        /// Text diff granularity [default: line]. Only valid for text diffs.
        #[arg(short, long, value_enum)]
        mode: Option<ModeArg>,
    },
    /// Open the desktop app (not implemented yet).
    Gui {
        left: Option<PathBuf>,
        right: Option<PathBuf>,
    },
}

/// When to emit ANSI colors.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum ColorArg {
    Auto,
    Always,
    Never,
}

impl From<ColorArg> for anstream::ColorChoice {
    fn from(color: ColorArg) -> Self {
        match color {
            ColorArg::Auto => Self::Auto,
            ColorArg::Always => Self::Always,
            ColorArg::Never => Self::Never,
        }
    }
}

/// How to compare the two inputs.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum KindArg {
    Text,
    Json,
}

/// CLI mirror of [`DiffMode`] so the core crate stays free of clap.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum ModeArg {
    Line,
    Word,
    Char,
}

impl From<ModeArg> for DiffMode {
    fn from(mode: ModeArg) -> Self {
        match mode {
            ModeArg::Line => Self::Line,
            ModeArg::Word => Self::Word,
            ModeArg::Char => Self::Char,
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }
}
