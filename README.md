# agb-ui

Menus for [agb](https://agbrs.dev) games on the Game Boy Advance.

- **A headless core.** A `Menu` holds a menu's state (the selected row, the window of rows shown, each row's choice) and turns the buttons into `Event`s. It draws nothing and plays nothing: the game does both.
- **An optional layout and redraw helper.** `draw` lays a menu out in a box (a title, rows with a cursor column, ▲ / ▼ when rows are hidden, a description line) and calls the small drawing functions of a `MenuRenderer` the game implements, only for what changed since the last frame.

Every menu follows the same rules: ▲ / ▼ move the selection, wrapping at the ends and scrolling. ◀ / ▶ cycle a row's choices. A picks the row. A disabled row can be selected, but A on it is refused. BACK, if the menu has it, is its last row, and B does what BACK does.

## Use

agb-ui isn't on crates.io. Take it from git, pinned to a tag:

```toml
[dependencies]
agb-ui = { git = "https://github.com/farmisen/agb-ui", tag = "v0.1.0" }
```

It needs agb 0.25.

```rust
use agb_ui::{Event, Fixed, Menu, Row};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Opt {
    Music,
    Sfx,
    Back,
}

static OPTIONS: [Row<Opt>; 3] = [
    Row::choice(Opt::Music, "MUSIC", &["ON", "OFF"]),
    Row::choice(Opt::Sfx, "SFX", &["ON", "OFF"]),
    Row::back(Opt::Back),
];

let mut menu = Menu::new(&OPTIONS);

// Every frame, after `input.update()`:
match menu.update(&input, &Fixed) {
    Some(Event::Moved) => { /* tick */ }
    Some(Event::Changed(Opt::Music)) => { let on = menu.value(Opt::Music) == 0; /* … */ }
    Some(Event::Back) => { /* leave the menu */ }
    _ => {}
}
```

`Fixed` is for rows that never change. For rows that do (disabled, a price, a description that depends on the game), implement `RowState` for a type of the game's.

To draw, implement `MenuRenderer` (draw a box, a row, the ▲ / ▼ marks, the description line, all in 8 px cells) and call `agb_ui::draw` every frame with an `Option<Drawn>` kept for each place a menu shows. It returns where the cursor goes; the game draws the cursor.

## Developing

```sh
make check   # everything CI runs: fmt, host tests, clippy for the GBA and for the host
```

The tests run on the host without agb (`--no-default-features`). The toolchain is the pinned nightly in `rust-toolchain.toml`, the same as the games that use the crate.

To change agb-ui while working on a game, point the game at a local checkout without editing its `Cargo.toml`, in `~/.cargo/config.toml` (a relative path there is relative to your home directory):

```toml
[patch."https://github.com/farmisen/agb-ui"]
agb-ui = { path = "/path/to/agb-ui" }
```

or for one command: `cargo build --config 'patch."https://github.com/farmisen/agb-ui".agb-ui.path="../agb-ui"'`.

A patched build rewrites the game's `Cargo.lock` to the local checkout (agb-ui loses its `source` line). Don't commit that: build once without the patch to put the git source back, or `git checkout Cargo.lock`.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
