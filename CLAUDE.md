# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

**diffly** is a local desktop app written in Rust that shows the differences between two data sources. It is meant to be one app for many kinds of input: JSON, HTML, source code in various languages, whole files, and more. Each input type can be compared with more than one diff technique, such as line, word or character diffs, structural/semantic diffs for JSON and HTML, and syntax-aware diffs for code. Everything runs locally, with no network calls.

The UI framework is **GPUI Kit** (https://gpui-kit.com/, crate `gpui-kit`). It is built on Zed's GPUI and comes from Longbridge's `gpui-component`. It provides components, data tables, docking panels, theming, and a Rope-backed code editor with Tree-sitter highlighting. Use those built-in pieces for side-by-side views, virtualized lists, and syntax highlighting before writing custom ones.

## Workspace

- `crates/diffly-core`: the diff engine. It uses `thiserror` (`DiffError`) and produces a front-end-neutral `DiffResult` (shared `DiffStats` + a per-kind `DiffBody`). It **must not depend on gpui/gpui-kit, clap or anyhow**, so it stays testable without a window and reusable by any front-end.
- `crates/diffly`: the `diffly` binary. It uses `anyhow` with `.context(...)` at IO boundaries and parses args with clap derive (`src/cli.rs`). Clap-only types such as `ModeArg` mirror core types and convert with `From`, which keeps `ValueEnum` out of core. The GPUI Kit app lives in `src/gui/`: `diffly gui LEFT RIGHT` opens a `DiffView` that renders `TextDiff::side_by_side()` rows (a core function, so alignment is tested without a window) in a virtualized `uniform_list`. Load and diff errors become an on-screen error panel, never a panic.
- CLI color: renderers always write `anstyle` styles through the `paint` helper (in `commands/diff.rs`), which closes the style before each line ending. Stdout is wrapped in `anstream::AutoStream`, which strips the styles for `--color never` and for `auto` off a terminal, so uncolored output can't drift from colored output.
- GUI tests are headless `#[gpui_kit::test]` unit tests inside the binary (`src/gui/view.rs`), via gpui-kit's `test-support` dev feature. Give each element worth asserting `.id(...).test_support().aria_label(...)` and query it with `window.find(id).label()`; this needs no display, so it runs on all CI OSes. Pixels aren't checked: look at the real window for visual changes.
- `diffly diff` exit codes follow `diff(1)`: 0 identical, 1 different, 2 error.

Front-ends only call `diffly_core::diff(left, right, &DiffOptions)`. It takes two in-memory strings, dispatches on `InputKind`, and returns `Result<DiffResult>`; loading files is a separate step. To add a new data type:
- Add an `InputKind` variant (text granularity lives inside `InputKind::Text`).
- Add its own module returning a new `DiffBody` variant.
- Teach `InputKind::detect` its file extensions if it has any.
- Render the new `DiffBody` variant in each front-end, and expose the kind in the CLI's `--kind` (`KindArg`).

`DiffBody` is deliberately not `#[non_exhaustive]`, so every front-end fails to compile until it renders the new kind. Don't add special cases in view or CLI code. Core behaviour is tested at that seam: `crates/diffly-core/tests/` drives the public API with strings.

## Commands

The `justfile` is the entry point (`just` lists recipes). One-time setup: `just setup` installs bacon, cargo-nextest, cargo-deny, typos-cli and taplo-cli, and points `core.hooksPath` at `.githooks/`.

```sh
just run diff a.txt b.txt --mode word   # run the CLI
just run gui a.txt b.txt                # open the desktop window
just watch [job]        # bacon; jobs: clippy (default), check, test, doc, run
just test               # nextest (falls back to cargo test) + doctests
just test-one <name>    # or: cargo test -p diffly-core <name>
just lint               # clippy --workspace --all-targets -D warnings
just fmt / fmt-check    # rustfmt + taplo (TOML formatting)
just ci                 # fmt-check, lint, test, deny, typos (mirrors .github/workflows/ci.yml)
```

The pre-commit hook runs `just pre-commit` (fmt-check + lint).

## Lint policy

The toolchain is pinned in `rust-toolchain.toml`, and CI installs it from that file (`rustup toolchain install`), so local and CI clippy match. A new Rust release can bring new lints: bump the pin deliberately and fix the fallout in the same commit.


Lints are defined once in `[workspace.lints]` in the root `Cargo.toml`, and every crate opts in with `[lints] workspace = true`. Clippy `pedantic` is on, `unwrap_used` warns, and `unsafe_code` is forbidden. CI treats warnings as errors. `clippy.toml` allows unwrap/expect inside `#[test]` fns. Integration-test files add `#![allow(clippy::unwrap_used)]` because their helper fns aren't covered. Inside the binary crate, use `pub(crate)` rather than `pub`, because `unreachable_pub` is enabled.

Add new dependencies to `[workspace.dependencies]` and reference them with `dep.workspace = true`. `deny.toml` controls the allowed licenses: update it if a new dependency brings a license that isn't on the list. Advisories only fail for unmaintained crates we depend on directly (`unmaintained = "workspace"`), because gpui-kit's tree has several; any ignored vulnerability needs a reason and a removal condition. gpui-kit sets the MSRV (`rust-version` 1.88). On Linux, building needs the system headers listed in `GPUI_LINUX_DEPS` in `.github/workflows/ci.yml`.
