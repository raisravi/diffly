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
        /// Diff granularity.
        #[arg(short, long, value_enum, default_value_t)]
        mode: ModeArg,
    },
    /// Open the desktop app (not implemented yet).
    Gui {
        left: Option<PathBuf>,
        right: Option<PathBuf>,
    },
}

/// CLI mirror of [`DiffMode`] so the core crate stays free of clap.
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub(crate) enum ModeArg {
    #[default]
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
