# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Cargo workspace with `diffly-core` (diff engine) and `diffly` (CLI) crates.
- `diffly diff LEFT RIGHT --mode line|word|char` command.
- Dev tooling: justfile, bacon, clippy/rustfmt config, cargo-deny, typos, taplo, pre-commit hook, CI.
