# diffly

A unified, local-first desktop app for comparing data: JSON, HTML, source code, plain files and more, with multiple diff techniques per data type. The desktop UI will be built with [GPUI Kit](https://gpui-kit.com/).

> **Status:** early development. The diff engine and a terminal CLI work today; the desktop app is not implemented yet.

## Usage

```sh
diffly diff old.txt new.txt               # line diff
diffly diff old.txt new.txt --mode word   # word diff: [-removed-]{+added+}
diffly diff old.txt new.txt --mode char   # character diff
```

Exit codes follow `diff(1)`: `0` identical, `1` different, `2` error. Use `-v`/`-vv` or `RUST_LOG` for logging.

## Development

Requires stable Rust (pinned via `rust-toolchain.toml`) and [just](https://github.com/casey/just).

```sh
just setup      # install bacon, cargo-nextest, cargo-deny, typos, taplo; enable git hooks
just            # list all recipes
just watch      # bacon: clippy on save (`just watch test` for tests)
just test       # run all tests
just ci         # everything CI runs: fmt-check, lint, test, deny, typos
```

### Layout

| Crate | Purpose |
| --- | --- |
| `crates/diffly-core` | UI-agnostic diff engine (`thiserror` errors, no GPUI/clap) |
| `crates/diffly` | `diffly` binary: CLI now, desktop app later (`anyhow` errors) |

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.
