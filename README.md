# diffly

A unified, local-first desktop app for comparing data: JSON, HTML, source code, plain files and more, with multiple diff techniques per data type. The desktop UI is built with [GPUI Kit](https://gpui-kit.com/).

> **Status:** early development. The diff engine and a terminal CLI work today; the desktop app shows a side-by-side line diff of two files.

## Usage

```sh
diffly gui old.txt new.txt                # desktop window: side-by-side line diff
diffly diff old.txt new.txt               # line diff
diffly diff old.txt new.txt --mode word   # word diff: [-removed-]{+added+}
diffly diff old.txt new.txt --mode char   # character diff
diffly diff old.json new.json             # structural JSON diff: $.user.tags[2]: added "admin"
diffly diff a.txt b.txt --kind json       # force JSON (or --kind text to force a line diff)
```

JSON is detected when both files end in `.json`; key order and formatting are ignored, arrays are compared by index.

`diffly gui` always compares as lines for now (choosing the kind and mode in the app is planned). If a file can't be read, the window shows the error instead of the diff.

Exit codes follow `diff(1)`: `0` identical, `1` different, `2` error. Output is colored on a terminal; `--color auto|always|never` overrides that, and `NO_COLOR` is honoured. Use `-v`/`-vv` or `RUST_LOG` for logging.

## Development

Requires stable Rust (pinned via `rust-toolchain.toml`) and [just](https://github.com/casey/just). On Linux, GPUI also needs system headers, e.g. on Debian/Ubuntu:

```sh
sudo apt-get install libfontconfig-dev libfreetype-dev libwayland-dev libx11-xcb-dev \
  libxcb1-dev libxkbcommon-dev libxkbcommon-x11-dev
```

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
| `crates/diffly` | `diffly` binary: CLI and GPUI Kit desktop app (`anyhow` errors) |

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.
