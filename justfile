# List available recipes
default:
    @just --list

# Install dev tools and enable git hooks
setup: hooks
    cargo install --locked bacon cargo-nextest cargo-deny typos-cli taplo-cli

# Use the repo's .githooks directory
hooks:
    git config core.hooksPath .githooks

# Run the CLI, e.g. `just run diff a.txt b.txt --mode word`
run *args:
    cargo run -p diffly -- {{ args }}

build:
    cargo build --workspace

check:
    cargo check --workspace --all-targets

fmt:
    cargo fmt --all
    taplo fmt

fmt-check:
    cargo fmt --all --check
    taplo fmt --check

lint:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

# Run all tests (nextest if installed, else cargo test)
test:
    #!/usr/bin/env sh
    if command -v cargo-nextest >/dev/null; then
        cargo nextest run --workspace && cargo test --workspace --doc
    else
        cargo test --workspace
    fi

# Run tests whose name contains NAME
test-one name:
    cargo test --workspace {{ name }} -- --nocapture

doc-test:
    cargo test --workspace --doc

# Watch with bacon (jobs: check, clippy, test, doc, run)
watch job="clippy":
    bacon {{ job }}

deny:
    cargo deny check

typos:
    typos

# Everything CI runs
ci: fmt-check lint test deny typos

# Fast checks run by the git pre-commit hook
pre-commit: fmt-check lint

clean:
    cargo clean
