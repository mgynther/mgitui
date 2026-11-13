# mgitui

mgitui is a terminal user interface for git. It makes a few opinionated git workflows smooth and deliberately leaves out most of git's features. It offers a gitk-style branch graph, a git gui-style view for staging lines and committing, and a log of everything it runs.

mgitui runs on Linux.

## Requirements

### Runtime

mgitui runs `git` and `vim` from your `PATH`.

Debian / Ubuntu:

```sh
sudo apt install git vim
```

Fedora:

```sh
sudo dnf install git vim-enhanced
```

Arch:

```sh
sudo pacman -S git vim
```

Verify:

```sh
git --version
vim --version
```

### Building

Rust is installed with [rustup](https://rustup.rs):

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

The installer adds `~/.cargo/bin` to `PATH` in your shell's startup files, so it takes effect in new terminals. To use it in the current terminal right away:

```sh
. "$HOME/.cargo/env"            # bash, zsh, sh
source "$HOME/.cargo/env.fish"  # fish
```

Verify:

```sh
rustc --version
cargo --version
```

Rust needs a C linker. Debian / Ubuntu: `sudo apt install build-essential`. Fedora: `sudo dnf install gcc`. Arch: `sudo pacman -S base-devel`.

### Development

The linter and formatter are rustup components (included in the default profile):

```sh
rustup component add clippy rustfmt
```

Test coverage uses cargo-llvm-cov:

```sh
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov
```

Verify:

```sh
cargo clippy --version
cargo fmt --version
cargo llvm-cov --version
```

## Building and running

```sh
cargo build --release
./target/release/mgitui
```

Run mgitui inside a git repository. To install it into `~/.cargo/bin`:

```sh
cargo install --path crates/wiring
```

## Development checks

`./check.sh` runs all checks in order: build, tests with 100% coverage, lint and format.

The code is split into one crate per layer under `crates/`: `git`, `logic`, `ui`, `validation` and `wiring`. Only `wiring` depends on the other crates, so each layer can be built and tested on its own, for example `cargo test -p logic`.
