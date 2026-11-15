#!/bin/sh
# Runs the project checks in order: build, tests with coverage, lint, format.
set -eu

echo "== build"
cargo build --workspace --all-targets

echo "== tests"
cargo test --workspace

echo "== coverage"
cargo llvm-cov --workspace \
    --fail-under-lines 100 --fail-under-functions 100 --fail-under-regions 100 \
    --show-missing-lines

echo "== lint"
cargo clippy --workspace --all-targets -- -D warnings

echo "== format"
if ! cargo fmt --all --check > /dev/null; then
    cargo fmt --all
    echo "formatting changed files; run the checks again"
    exit 1
fi

echo "== all checks passed"
