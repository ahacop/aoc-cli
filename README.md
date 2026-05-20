# aoc

A small CLI for fetching [Advent of Code](https://adventofcode.com) puzzles and inputs.

## Install

### Homebrew (macOS, Linux)

```sh
brew install ahacop/tap/aoc-cli
```

### From source

```sh
cargo install --git https://github.com/ahacop/aoc-cli
```

### Shell installer

Pre-built binaries for `aarch64-apple-darwin`, `aarch64-unknown-linux-gnu`, and `x86_64-unknown-linux-gnu` are attached to each [GitHub Release](https://github.com/ahacop/aoc-cli/releases), along with a `curl | sh` installer.

## Usage

```sh
aoc login                 # acquire and store an AoC session cookie
aoc where                 # print where the session file lives
aoc puzzle 2023 1         # render the puzzle text for day 1 of 2023
aoc input  2023 1         # download the puzzle input
```

`aoc login` with no argument first tries to read the `session` cookie out of local browsers (Firefox, LibreWolf, Zen, plus whatever `rookie` supports). If that fails it opens the AoC login page so you can paste a token from `document.cookie`.

Status messages go to stderr; puzzle text and input go to stdout, so:

```sh
aoc input 2023 1 > input.txt
```

## Development

The dev shell (entered automatically via `direnv` / `flake.nix`) provides the Rust toolchain plus `cargo-watch`, `cargo-edit`, `just`, and `cargo-dist`. If a tool is missing on a NixOS host, wrap it in `nix-shell -p <pkg> --run "..."`.

```sh
cargo build                              # debug build
cargo build --release                    # release build
cargo run -- puzzle 2023 1               # run subcommand
cargo test                               # tests
cargo clippy --all-targets -- -D warnings
cargo fmt
nix build                                # produces ./result/bin/aoc
```

## Releases

Releases are automated by [`cargo-dist`](https://github.com/axodotdev/cargo-dist) (invoked as `dist`). A tag push to GitHub triggers `.github/workflows/release.yml`, which:

1. Builds binaries for all three target triples in parallel.
2. Creates a GitHub Release and uploads tarballs + checksums + a shell installer.
3. Generates a Homebrew formula and pushes it to [`ahacop/homebrew-tap`](https://github.com/ahacop/homebrew-tap).

### One-time setup

A GitHub PAT with `contents: write` on `ahacop/homebrew-tap` must be available to this repo as a secret named `HOMEBREW_TAP_TOKEN`:

```sh
gh secret set HOMEBREW_TAP_TOKEN --repo ahacop/aoc-cli
```

### Cutting a release

```sh
cargo set-version 0.2.0          # whatever bump
git commit -am "Release v0.2.0"
git tag v0.2.0
git push && git push --tags
```

The tag push fires the workflow; binaries and the updated Homebrew formula land within a few minutes.

### Regenerating the workflow

The workflow pins itself to the `dist` version that generated it (currently `0.30.4`, from `nixpkgs`). When the flake bumps `cargo-dist`, regenerate so CI matches:

```sh
dist init --yes
```

Then commit `dist-workspace.toml` and `.github/workflows/release.yml`.
