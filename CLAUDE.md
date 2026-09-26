# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

**diffly** is a local desktop app written in Rust that shows the differences between two data sources. It is meant to be one app for many kinds of input: JSON, HTML, source code in various languages, whole files, and more. Each input type can be compared with more than one diff technique, such as line, word or character diffs, structural/semantic diffs for JSON and HTML, and syntax-aware diffs for code. Everything runs locally, with no network calls.

The UI framework is **GPUI Kit** (https://gpui-kit.com/, crate `gpui-kit`). It is built on Zed's GPUI and comes from Longbridge's `gpui-component`. It provides components, data tables, docking panels, theming, and a Rope-backed code editor with Tree-sitter highlighting. Use those built-in pieces for side-by-side views, virtualized lists, and syntax highlighting before writing custom ones.

## Workspace

- `crates/diffly-core`: the diff engine. It uses `thiserror` (`DiffError`) and produces a front-end-neutral `DiffResult` (`Vec<Change>` + `DiffStats`). It **must not depend on gpui/gpui-kit, clap or anyhow**, so it stays testable without a window and reusable by any front-end.
- `crates/diffly`: the `diffly` binary. It uses `anyhow` with `.context(...)` at IO boundaries and parses args with clap derive (`src/cli.rs`). Clap-only types such as `ModeArg` mirror core types and convert with `From`, which keeps `ValueEnum` out of core. The GPUI Kit app will live here too; the `gui` subcommand is currently a placeholder.
- `diffly diff` exit codes follow `diff(1)`: 0 identical, 1 different, 2 error.

To add a new data type or diff technique, plug it into the core layer through a shared abstraction, for example "input kind → parser → diff strategy → `DiffResult`". Don't add special cases in view or CLI code.

## Commands

The `justfile` is the entry point (`just` lists recipes). One-time setup: `just setup` installs bacon, cargo-nextest, cargo-deny, typos-cli and taplo-cli, and points `core.hooksPath` at `.githooks/`.

```sh
just run diff a.txt b.txt --mode word   # run the CLI
just watch [job]        # bacon; jobs: clippy (default), check, test, doc, run
just test               # nextest (falls back to cargo test) + doctests
just test-one <name>    # or: cargo test -p diffly-core <name>
just lint               # clippy --workspace --all-targets -D warnings
just fmt / fmt-check    # rustfmt + taplo (TOML formatting)
just ci                 # fmt-check, lint, test, deny, typos (mirrors .github/workflows/ci.yml)
```

The pre-commit hook runs `just pre-commit` (fmt-check + lint).

## Lint policy

Lints are defined once in `[workspace.lints]` in the root `Cargo.toml`, and every crate opts in with `[lints] workspace = true`. Clippy `pedantic` is on, `unwrap_used` warns, and `unsafe_code` is forbidden. CI treats warnings as errors. `clippy.toml` allows unwrap/expect inside `#[test]` fns. Integration-test files add `#![allow(clippy::unwrap_used)]` because their helper fns aren't covered. Inside the binary crate, use `pub(crate)` rather than `pub`, because `unreachable_pub` is enabled.

Add new dependencies to `[workspace.dependencies]` and reference them with `dep.workspace = true`. `deny.toml` controls the allowed licenses: update it if a new dependency (e.g. gpui-kit's tree) brings a license that isn't on the list.
