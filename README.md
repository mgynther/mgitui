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

`./upgrade.sh` upgrades the pinned Rust toolchain in `rust-toolchain.toml` to the latest stable, raises `rust-version` in `Cargo.toml` to match, updates `Cargo.lock` and cargo-llvm-cov, and then runs `./check.sh`. It commits nothing. New Clippy lints in a new toolchain can make the checks fail; fix them before committing. The previous toolchain stays installed; remove it with `rustup toolchain uninstall <version>` when no longer needed.

### Testing one layer

The code is split into one crate per layer under `crates/`: `git`, `logic`, `ui`, `validation` and `wiring`. Only `wiring` depends on the other crates, so each layer builds and tests on its own, even while another layer does not compile:

```sh
cargo test -p logic                  # all tests of one layer
cargo test -p logic switching        # only tests whose name contains "switching"
cargo clippy -p logic --all-targets -- -D warnings
cargo llvm-cov -p logic --fail-under-lines 100 --fail-under-functions 100 --fail-under-regions 100
```

For `wiring`, add the `--ignore-filename-regex` from `check.sh` to the coverage command. Run `./check.sh` before committing.

If per-layer coverage reports well-tested code as uncovered, the coverage data from earlier runs is stale. Clear it with `cargo llvm-cov clean --workspace` and run the coverage command again.

## Keys

| Key | Where | Action |
|---|---|---|
| F1 / F2 / F3 | everywhere | Branches / Diff / Log view; F1 and F2 also reload from git |
| q | everywhere | quit (asks `Quit? y/n`) |
| r | everywhere | reload from git |
| j / k / g / G | lists and diffs | down / up / top / bottom |
| / then n / m | lists | search, next / previous match |
| w | Branches, Diff | next pane |
| c | Branches | create a branch at the selected commit |
| h | Branches | check out the commit's local branch |
| p | Branches | cherry-pick the selected commit onto the checked out branch |
| u | Branches | fetch all remotes with `git remote update` |
| d | Branches | delete a local branch of the selected commit that is not checked out, after confirming with y (also when not merged) |
| b | Branches | rebase a branch of the selected commit onto another local branch or remote main/master |
| i | Branches | the same as b, interactively: git opens Vim for the todo list |
| s | Diff file lists | switch between unstaged and staged |
| Enter | Diff file list | stage or unstage the whole file |
| d | Diff unstaged list | discard the file's unstaged changes, or delete it when untracked, after confirming with y |
| space, j / k, Enter | Diff file diff | select lines, stage or unstage them |
| Esc | Diff file diff | clear the line selection |
| e | Diff | edit the commit message in Vim |
| c | Diff | commit |
| a | Diff | toggle amending the last commit |

## Manual checks

The terminal handling and starting Vim are not covered by automated tests. After changing `crates/wiring/src/main.rs` or `crates/wiring/src/terminal.rs`, check by hand:

1. Start `mgitui` in a repository: the Branches view is shown and the terminal is in raw mode (typed keys are not echoed).
2. F1, F2 and F3 switch views, also in a different terminal emulator and in the Linux console if available.
3. Resize the terminal: the view is redrawn; below 80x24 the "terminal too small" message is shown.
4. In the Diff view press e: Vim opens on the commit message, and after `:wq` the message is shown in the box and the screen is redrawn.
5. Quit Vim with `:q!`: the message is unchanged.
6. In the Branches view press i on a branch and choose a branch to rebase onto: git opens Vim with the todo list, and after `:wq` the app comes back with the rebased graph.
7. Press q then y: the terminal is restored, the cursor is visible and typed keys are echoed again.
8. Outside a git repository `mgitui` prints an error and exits with status 1.
