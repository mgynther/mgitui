#!/bin/sh
# Upgrades rustup, the pinned Rust toolchain, the minimum supported Rust
# version, the crate dependencies and cargo-llvm-cov, then runs the checks.
# Nothing is committed: review the changes with git diff afterwards.
set -eu

cd "$(dirname "$0")"

echo "== rustup"
rustup self update
rustup update stable

# "rustc 1.101.0 (hash date)" -> "1.101.0". The +stable selects the stable
# toolchain for this one command, bypassing the pin in rust-toolchain.toml.
new=$(rustc +stable --version | cut -d ' ' -f 2)
case "$new" in
    [0-9]*.[0-9]*.[0-9]*) ;;
    *)
        echo "could not read the stable version: $new"
        exit 1
        ;;
esac
old=$(sed -n 's/^channel = "\(.*\)"$/\1/p' rust-toolchain.toml)
msrv=${new%.*}

if [ "$old" = "$new" ]; then
    echo "== toolchain already at $new"
else
    echo "== toolchain $old -> $new, rust-version $msrv"
fi
# -i.bak works with both GNU and BSD sed; the backup is removed right away.
sed -i.bak "s/^channel = \".*\"$/channel = \"$new\"/" rust-toolchain.toml
sed -i.bak "s/^rust-version = \".*\"$/rust-version = \"$msrv\"/" Cargo.toml
rm rust-toolchain.toml.bak Cargo.toml.bak

# The first cargo command installs the newly pinned toolchain and the
# components listed in rust-toolchain.toml.
echo "== dependencies"
cargo update

echo "== cargo-llvm-cov"
cov_old=$(cargo llvm-cov --version 2>/dev/null || echo "not installed")
cargo install cargo-llvm-cov --locked
cov_new=$(cargo llvm-cov --version)

echo "== checks"
status=0
./check.sh || status=$?

echo "== summary"
if [ "$old" = "$new" ]; then
    echo "toolchain: already at the latest stable $new"
else
    echo "toolchain: $old -> $new (the old one is still installed)"
fi
if [ "$cov_old" = "$cov_new" ]; then
    echo "cargo-llvm-cov: already up to date ($cov_new)"
else
    echo "cargo-llvm-cov: $cov_old -> $cov_new"
fi
if git diff --quiet; then
    echo "files: no changes"
else
    echo "files changed:"
    git diff --stat
fi
if [ "$status" -ne 0 ]; then
    echo "checks failed; fix them and run ./check.sh again"
    exit "$status"
fi
echo "checks passed"
