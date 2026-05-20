# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

`aoc-cli` is a small Rust CLI (binary name `aoc`) for fetching Advent of Code puzzles and inputs, submitting answers, and tracking progress.

## Commands

The dev shell (entered automatically via `direnv` / `flake.nix`) provides the Rust toolchain plus `cargo-watch`, `cargo-edit`, and `just`. If a tool is missing on this NixOS host, wrap it in `nix-shell -p <pkg> --run "..."`.

- Build: `cargo build` (release: `cargo build --release`)
- Run: `cargo run -- <subcommand>` (e.g. `cargo run -- puzzle 2023 1`)
- Test: `cargo test` — run a single test with `cargo test extracts_multiple_articles`
- Lint/format: `cargo clippy --all-targets -- -D warnings` and `cargo fmt`
- Nix build: `nix build` (produces `./result/bin/aoc`)

## Architecture

Five modules, each with a narrow responsibility:

- `src/main.rs` — `clap` subcommand dispatch (`Login`, `Where`, `Puzzle`, `Input`, `Submit`, `Next`, `Stars`, `Open`) and input validation (`year >= 2015`, `day ∈ 1..=25`, `part ∈ {1, 2}`). `submit` reads the answer from stdin when the positional arg is omitted, so solver output can be piped in. `input` reads from the on-disk cache when present; `--refresh` forces a re-fetch and overwrites the cached copy. `open` shells out via the `webbrowser` crate.
- `src/auth.rs` — token acquisition. `aoc login` (no arg) first tries to read the `session` cookie from local browsers via `rookie`, augmented with extra Firefox cookie DB paths under `~/.config/{mozilla/firefox,librewolf,zen}/<profile>/cookies.sqlite` (NixOS / Home Manager layout that `rookie` misses by default). Picks the cookie with the latest expiry. Falls back to opening the AoC login page in a browser and reading a pasted token from stdin.
- `src/session.rs` — auth-token persistence. Token is stored at the platform config dir (`directories::ProjectDirs` with qualifier `""`, organization `""`, app `aoc-cli`) as a file named `session`, chmod 0600 on Unix.
- `src/cache.rs` — puzzle-input persistence. Inputs are stored under the platform cache dir at `inputs/<year>/<day>.txt` (zero-padded day). `read_input` returns `Ok(None)` on `NotFound` and bubbles up any other I/O error; `write_input` creates the parent dirs and writes the body. Per AoC's automation guidelines, inputs are stable per user — they're cached indefinitely and only re-fetched on `--refresh`.
- `src/client.rs` — `reqwest` blocking client with `redirect::Policy::none()` so that AoC's "redirect to login" responses surface as `302 FOUND` and are mapped to a clear auth-failure error (alongside `401`). Sends the session cookie as a `Cookie` header. `render_puzzle` extracts `<article class="day-desc">...</article>` blocks (there are two once part 2 is unlocked) and renders each via `html2text` at width 100; if no articles are found it falls back to rendering the whole page. `submit_answer` POSTs `level=<part>&answer=<value>` to `/{year}/day/{day}/answer`, and `render_response` extracts the first `<article>...</article>` (no `day-desc` class) from the feedback page. `next_unsolved` and `star_summary` both parse the calendar page's `aria-label="Day N[, one star|two stars]"` markers — the former finds the lowest day still owing stars, the latter returns a per-day star count vec covering only released days.

## Conventions

- Errors flow through `anyhow::Result`; user-visible messages use `bail!`/`ensure!` with concrete remediation (e.g. "run `aoc login` first").
- Status messages go to stderr; puzzle/input payloads go to stdout so they can be piped/redirected.
- The HTTP `User-Agent` is built from `CARGO_PKG_VERSION` and includes the repo URL — keep it identifying per AoC's automation guidelines.
