mod cli;
mod commands;
mod gui;

use std::process::ExitCode;

use anyhow::bail;
use clap::Parser;
use tracing_subscriber::EnvFilter;

use crate::cli::{Cli, Command};

fn main() -> ExitCode {
    let cli = Cli::parse();
    init_tracing(cli.verbose);

    match run(cli) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err:?}");
            ExitCode::from(2)
        }
    }
}

fn run(cli: Cli) -> anyhow::Result<ExitCode> {
    match cli.command {
        Some(Command::Diff {
            left,
            right,
            kind,
            mode,
        }) => commands::diff(&left, &right, kind, mode.map(Into::into), cli.color.into()),
        Some(Command::Gui { left, right }) => {
            gui::run(&left, &right)?;
            Ok(ExitCode::SUCCESS)
        }
        None => {
            bail!("pass two files: `diffly gui <LEFT> <RIGHT>` or `diffly diff <LEFT> <RIGHT>`")
        }
    }
}

fn init_tracing(verbose: u8) {
    let default_level = match verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();
}
