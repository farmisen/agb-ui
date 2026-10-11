# agb-ui Project Guide

Menus for [agb](https://agbrs.dev) (Game Boy Advance, Rust) games: a headless core and an optional layout and redraw helper. Taken by games as a git dependency pinned to a tag; first used by Frogman ∞ (`farmisen/frogman-infinite-gba`).

## Critical rules

- **Never `git commit` or `git push` without explicit approval for this change.** Applies to all branches.
- **Never merge a pull request.** Opening the PR is where the work stops.
- **No attribution trailers.** Commit messages and PR descriptions get no `Co-Authored-By:`, "Generated with …", or similar agent attribution lines.
- **The repo is public.** Nothing from a game's private repo (art, licensed assets, unreleased designs) comes in here, tests included.

## Commands

- Everything CI runs: `make check`
- Tests (host, without agb): `make test`
- Lint for the GBA, with agb: `make clippy`
- Lint the host build and the tests: `make clippy-host`
- Format: `cargo fmt`

No `.cargo/config.toml`: its build-std would clash with the host's std in the tests, so the Makefile passes the GBA target and `-Zbuild-std` to `make clippy` instead.

## Stack

- Rust nightly pinned in `rust-toolchain.toml`, the same as the games that use it; edition 2024.
- `agb` 0.25, optional (feature `agb`, on by default): only for `Input::from(&ButtonController)`. Without it the crate builds for the host.
- `no_std` + `alloc`: allocates when a menu is made or drawn anew, never in a frame that only updates it or redraws rows.

## Layout

```
src/lib.rs   crate docs (the rules every menu follows) and the public API
src/menu.rs  the headless core: `Row` tables keyed by an id enum, `Menu` state, scrolling, BACK, `RowState` (disabled rows, values, descriptions), `Input`, `Event`s
src/draw.rs  the optional layer: `Layout`, the `MenuRenderer` trait, `draw` / `clear` redrawing only what changed
src/line.rs  `Line`, a fixed-capacity line of text (no heap)
```

## Versioning

`version` in `Cargo.toml` follows semver (0.x: a minor bump may break). Every release is a tag `vX.Y.Z` on `main`, the version in `Cargo.toml` matching; games pin to the tag. A change to the public API that breaks a game bumps the minor version.

## Agent failure modes

- Assuming a hosted stdlib: no filesystem, no println in library code.
- `.unwrap()` / `.expect()` outside tests — handle or propagate.
- Silencing clippy with `#[allow]` without a comment stating why.
- Integer `/` or `%` by anything but a power of two, or `*` of two variables, in code a game calls every frame: the GBA has no divide instruction and Thumb code checks overflow with a 64-bit multiply.
- Game-specific behaviour in the crate: it decides what a menu does with its buttons, never what it looks like or sounds like. Drawing goes through `MenuRenderer`, sounds are the game's answer to `Event`s.
