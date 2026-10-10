# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Cargo workspace with `diffly-core` (diff engine) and `diffly` (CLI) crates.
- `diffly diff LEFT RIGHT --mode line|word|char` command.
- Structural JSON diff: changes reported by JSON path, ignoring key order and formatting. Auto-detected for `.json` files, or chosen with `--kind json|text`.
- Colored diff output (insertions green, deletions red) with `--color auto|always|never`; `auto` honours `NO_COLOR` and `CLICOLOR_FORCE`.
- Desktop app (GPUI Kit): `diffly gui LEFT RIGHT` shows a side-by-side line diff with deletions on the left and insertions on the right; load errors are shown in the window.
- Dev tooling: justfile, bacon, clippy/rustfmt config, cargo-deny, typos, taplo, pre-commit hook, CI.

### Changed

- Minimum supported Rust version is now 1.88 (required by GPUI Kit).
