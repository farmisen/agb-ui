//! Menus for agb games.
//!
//! The core is headless: a [`Menu`] holds a menu's state (the selected row, the rows
//! scrolled past, each row's choice) and turns the buttons into [`Event`]s, and draws
//! nothing. Its rows are a fixed table of [`Row`]s named by an id enum of the game's;
//! what changes (a row disabled, a price, the description line) the game answers
//! through [`RowState`]. The game plays the sounds and acts on the events.
//!
//! On top, optionally, [`draw`] lays a menu out the usual way (a box, a title, rows top
//! to bottom with a cursor column, ▲ / ▼ when rows are hidden, a description line) and
//! calls the small drawing functions of a [`MenuRenderer`] the game implements, only
//! for what changed since the last frame. It uses nothing but the core's public API,
//! so a game can draw some menus with it and others itself.
//!
//! The rules every menu follows:
//! - ▲ / ▼ move the selection, wrapping at the ends, scrolling the window of rows
//!   shown. ◀ / ▶ cycle a row's choices, if it has any. A picks the row.
//! - A disabled row can be selected, but A (and ◀ / ▶) on it is refused.
//! - BACK, if the menu has it, is its last row. B does what BACK does when the menu
//!   has a way back (a BACK row, or [`Menu::with_back_button`]), and nothing
//!   otherwise.

#![cfg_attr(not(test), no_std)]

extern crate alloc;

mod draw;
mod line;
mod menu;

pub use draw::{Cursor, Drawn, Frame, Layout, MenuRenderer, RowView, Shown, clear, draw};
pub use line::Line;
pub use menu::{BACK, Event, Fixed, Input, Menu, Row, RowState, Value};
