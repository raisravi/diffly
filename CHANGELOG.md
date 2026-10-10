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
- Desktop app (GPUI Kit): `diffly` or `diffly gui [LEFT] [RIGHT]` compares two files side by side, with deletions on the left and insertions on the right; load errors are shown in the window.
- In the app, choose files with a picker or by drag and drop, and pick the kind (auto/text/json) and granularity (line/word/char) from a toolbar; the diff re-runs on every change. Word and char modes highlight the changed parts of each changed line; JSON is listed by path.
- Dev tooling: justfile, bacon, clippy/rustfmt config, cargo-deny, typos, taplo, pre-commit hook, CI.

### Changed

- Minimum supported Rust version is now 1.88 (required by GPUI Kit).
