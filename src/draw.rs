//! The optional renderer layer: [`draw`] lays a menu out and calls a [`MenuRenderer`]
//! for what changed since the last frame. Built only on the core's public API.

use alloc::vec::Vec;
use core::fmt::Write;

use crate::{Line, Menu, RowState, Value};

/// What a game implements for [`draw`]. Everything is in cells (8 px text tiles on the
/// GBA), one cell a character, (0, 0) top-left; the box takes what's inside it.
pub trait MenuRenderer {
    /// The screen's size.
    const COLS: i32;
    const ROWS: i32;
    /// Cells the cursor takes, left of the labels.
    const CURSOR_COLS: i32;
    /// Cells an icon takes, left of its label.
    const ICON_COLS: i32;

    /// Cells `value` takes. Default: a number's digits, a text's characters.
    fn value_cols(&self, value: Value) -> i32 {
        match value {
            Value::None => 0,
            Value::Number(n) => n.checked_ilog10().map_or(1, |d| d as i32 + 1),
            Value::Text(s) => s.chars().count() as i32,
        }
    }

    /// The box `at` and its title, with nothing else in it: the title on row `at.title_y`
    /// from column [`Layout::title_x`], centred in the box.
    fn draw_box(&mut self, at: &Layout, title: Option<&str>);

    /// Clear what `draw_box` and the rest drew in `at`.
    fn clear_box(&mut self, at: &Layout);

    /// A row on screen, over whatever row was drawn there before (the list scrolls).
    fn draw_row(&mut self, at: &Layout, row: &RowView);

    /// ▲ at (`at.marks_x`, `at.up_y`) if `up`, else blank; ▼ at `at.down_y` likewise.
    fn draw_marks(&mut self, at: &Layout, up: bool, down: bool);

    /// The description line at `at.description_y`, over the last one.
    fn draw_description(&mut self, at: &Layout, text: &str);
}

/// A row as [`MenuRenderer::draw_row`] draws it.
pub struct RowView<'a> {
    /// The screen row it's on.
    pub y: i32,
    pub label: &'a str,
    /// Its choice, for a row with choices (drawn between ◀ and ▶ at `Layout::choice_x`).
    pub choice: Option<&'a str>,
    pub icon: Option<u16>,
    /// Right-aligned, ending before `Layout::right`.
    pub value: Value,
    pub selected: bool,
    pub disabled: bool,
}

/// What goes in a menu's box besides its rows.
#[derive(Clone, Copy)]
pub struct Frame<'a> {
    /// On the first line inside the box.
    pub title: Option<&'a str>,
    /// Lines between the title and the list that the game draws itself (a run summary),
    /// then a blank line; 0 for none.
    pub header_rows: i32,
    /// The header's widest line.
    pub header_cols: i32,
    /// Whether the rows show: false leaves their room empty (a summary counting up).
    pub list: bool,
    /// Cells the value column takes at least: the widest its values get, so the box
    /// holds still as they change (prices, a count). 0: as wide as the values now.
    pub value_cols: i32,
}

impl<'a> Frame<'a> {
    pub const fn titled(title: &'a str) -> Self {
        Self {
            title: Some(title),
            ..Self::UNTITLED
        }
    }

    pub const UNTITLED: Frame<'static> = Frame {
        title: None,
        header_rows: 0,
        header_cols: 0,
        list: true,
        value_cols: 0,
    };
}

/// Where a menu's box and rows go, in cells. Centred on the screen, a titled box one column
/// wider when that centres its title in it (a title and a box of the same parity: the title is
/// what the eye checks; on a screen of even width, an odd box then sits half a cell left).
/// Inside the border, top down: the
/// title, a blank line (▲ when rows are hidden above), the header and a blank line if
/// any, the rows two apart (▼ on the blank line after the last when rows are hidden
/// below), then the description and a blank line if any row has one. Across: a blank
/// column, the cursor, a blank column, the icons and a blank column if any row has
/// one, the labels, then a blank column and ◀ choice ▶ (as wide as the widest) for rows
/// with choices, then a blank column and the values, right-aligned, then a blank column.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Layout {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub title_y: Option<i32>,
    /// The header's first line.
    pub header_y: i32,
    /// The first row shown; the rest follow two apart.
    pub list_y: i32,
    /// How many rows show.
    pub rows: i32,
    /// Whether the rows show (`Frame::list`).
    pub list: bool,
    pub cursor_x: i32,
    pub icon_x: i32,
    pub label_x: i32,
    /// A choice's ◀.
    pub choice_x: i32,
    /// The column values and header lines end before: a blank column from the border.
    pub right: i32,
    pub marks_x: i32,
    pub up_y: i32,
    pub down_y: i32,
    pub description_y: Option<i32>,
}

impl Layout {
    /// The layout of `menu` in `frame` for `renderer`, its values from `state`.
    pub fn new<Id: Copy + PartialEq, R: MenuRenderer>(
        renderer: &R,
        menu: &Menu<Id>,
        state: &impl RowState<Id>,
        frame: &Frame,
    ) -> Self {
        let rows = menu.rows();
        let chars = |s: &str| s.chars().count() as i32;
        let widest = |w: &dyn Fn(usize) -> i32| (0..rows.len()).map(w).max().unwrap_or(0);
        let label_w = widest(&|i| chars(rows[i].label));
        let choice_w = widest(&|i| rows[i].choices.iter().map(|c| chars(c)).max().unwrap_or(0));
        let value_w =
            widest(&|i| renderer.value_cols(state.value(rows[i].id))).max(frame.value_cols);
        let description_w = widest(&|i| chars(rows[i].description));
        let icons = rows.iter().any(|r| r.icon.is_some());

        let cursor_dx = 2;
        let icon_dx = cursor_dx + R::CURSOR_COLS + 1;
        let label_dx = if icons {
            icon_dx + R::ICON_COLS + 1
        } else {
            icon_dx
        };
        let choice_dx = label_dx + label_w + 1;
        let mut content = if choice_w > 0 {
            choice_dx + 1 + choice_w + 1
        } else {
            label_dx + label_w
        };
        if value_w > 0 {
            content += 1 + value_w;
        }
        let title = frame.title.map_or(0, chars);
        // A blank column and the border after the content; room for the rest, a blank
        // column and the border either side.
        let mut w = (content + 2)
            .max(title + 4)
            .max(frame.header_cols + 4)
            .max(description_w + 4);
        if frame.title.is_some() && (w - title) % 2 != 0 && w < R::COLS {
            w += 1;
        }
        let shown = menu.shown() as i32;
        let header = if frame.header_rows > 0 {
            frame.header_rows + 1
        } else {
            0
        };
        let description = description_w > 0;
        let h = 3
            + i32::from(frame.title.is_some())
            + header
            + 2 * shown
            + if description { 2 } else { 0 };
        let x = (R::COLS - w) / 2;
        let y = (R::ROWS - h) / 2;
        let header_y = y + 2 + i32::from(frame.title.is_some());
        let list_y = header_y + header;
        let down_y = list_y + 2 * shown - 1;
        Self {
            x,
            y,
            w,
            h,
            title_y: frame.title.map(|_| y + 1),
            header_y,
            list_y,
            rows: shown,
            list: frame.list,
            cursor_x: x + cursor_dx,
            icon_x: x + icon_dx,
            label_x: x + label_dx,
            choice_x: x + choice_dx,
            right: x + w - 2,
            marks_x: x + w / 2,
            up_y: list_y - 1,
            down_y,
            description_y: description.then_some(down_y + 1),
        }
    }

    /// The screen row of the `slot`th row shown.
    pub fn row_y(&self, slot: usize) -> i32 {
        self.list_y + 2 * slot as i32
    }

    /// The column `title` starts at, centred in the box (exactly, the box's width and the
    /// title's having the same parity: `Layout::new` sees to it).
    pub fn title_x(&self, title: &str) -> i32 {
        self.x + (self.w - title.chars().count() as i32) / 2
    }
}

/// A menu as [`draw`] last drew it, to draw only what changed. Kept by the game, one
/// for every place a menu shows; `None` when none does.
pub struct Drawn {
    /// Which table of rows: its address.
    rows: usize,
    layout: Layout,
    title: Line,
    /// Each row shown as drawn, from the top.
    slots: Vec<SlotDrawn>,
    marks: (bool, bool),
    description: Line,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct SlotDrawn {
    index: usize,
    selected: bool,
    disabled: bool,
    choice: u8,
    value: Value,
}

/// What [`draw`] did this frame.
#[derive(Clone, Copy)]
pub struct Shown {
    pub at: Layout,
    /// It drew the box (anything the game drew in it is gone).
    pub box_drawn: bool,
    /// It drew a row.
    pub rows_drawn: bool,
    /// Where the cursor goes, if the rows show.
    pub cursor: Option<Cursor>,
}

/// The selected row, for drawing the cursor.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Cursor {
    /// Its screen row.
    pub y: i32,
    /// Its index in the menu's rows: a new one is a move, even if the list scrolled
    /// under a cursor that stayed put.
    pub index: usize,
    pub disabled: bool,
}

/// Draw `menu` in `frame` with `renderer`, as much of it as changed since `drawn`
/// (the box and all if it's another menu, or moved), and keep `drawn` up to date.
pub fn draw<Id: Copy + PartialEq, R: MenuRenderer>(
    drawn: &mut Option<Drawn>,
    renderer: &mut R,
    menu: &Menu<Id>,
    state: &impl RowState<Id>,
    frame: &Frame,
) -> Shown {
    let at = Layout::new(renderer, menu, state, frame);
    let rows = menu.rows();
    let address = rows.as_ptr() as usize;
    let mut title = Line::new();
    if let Some(t) = frame.title {
        // A title that didn't fit would compare equal to another with the same start.
        let fits = title.write_str(t).is_ok();
        debug_assert!(fits, "a title longer than a line");
    }
    let same = drawn
        .as_ref()
        .is_some_and(|d| d.rows == address && d.layout == at && d.title == title);
    if !same {
        clear(drawn, renderer);
        renderer.draw_box(&at, frame.title);
    }
    let d = drawn.get_or_insert_with(|| Drawn {
        rows: address,
        layout: at,
        title,
        slots: Vec::with_capacity(menu.shown()),
        marks: (false, false),
        description: Line::new(),
    });
    let mut shown = Shown {
        at,
        box_drawn: !same,
        rows_drawn: false,
        cursor: None,
    };
    if !at.list {
        return shown;
    }
    let selected = menu.selected_index();
    for slot in 0..menu.shown() {
        let index = menu.top() + slot;
        let row = &rows[index];
        let now = SlotDrawn {
            index,
            selected: index == selected,
            disabled: state.disabled(row.id),
            choice: menu.values()[index],
            value: state.value(row.id),
        };
        if now.selected {
            shown.cursor = Some(Cursor {
                y: at.row_y(slot),
                index,
                disabled: now.disabled,
            });
        }
        if d.slots.get(slot) == Some(&now) {
            continue;
        }
        renderer.draw_row(
            &at,
            &RowView {
                y: at.row_y(slot),
                label: row.label,
                choice: row.choices.get(usize::from(now.choice)).copied(),
                icon: row.icon,
                value: now.value,
                selected: now.selected,
                disabled: now.disabled,
            },
        );
        shown.rows_drawn = true;
        if slot < d.slots.len() {
            d.slots[slot] = now;
        } else {
            d.slots.push(now);
        }
    }
    let marks = (menu.more_above(), menu.more_below());
    if marks != d.marks {
        renderer.draw_marks(&at, marks.0, marks.1);
        d.marks = marks;
    }
    if at.description_y.is_some() {
        let mut text = Line::new();
        state.description(&rows[selected], &mut text);
        text.truncate_chars((at.w - 4).max(0) as usize);
        if text != d.description {
            renderer.draw_description(&at, text.as_str());
            d.description = text;
        }
    }
    shown
}

/// Clear the menu `drawn` holds, if any.
pub fn clear<R: MenuRenderer>(drawn: &mut Option<Drawn>, renderer: &mut R) {
    if let Some(old) = drawn.take() {
        renderer.clear_box(&old.layout);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Fixed, Input, Row};
    use alloc::string::{String, ToString};
    use alloc::vec;

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum R {
        Mode,
        Music,
        Play,
        Shop,
        Back,
    }

    static ROWS: [Row<R>; 5] = [
        Row::choice(R::Mode, "MODE", &["CHILL", "CLASSIC"]),
        Row::choice(R::Music, "MUSIC", &["ON", "OFF"]),
        Row::action(R::Play, "PLAY"),
        Row::action(R::Shop, "SHOP")
            .with_icon(7)
            .with_description("BUY THINGS"),
        Row::back(R::Back),
    ];

    /// Records what it was asked to draw.
    #[derive(Default)]
    struct Log(Vec<String>);

    impl MenuRenderer for Log {
        const COLS: i32 = 30;
        const ROWS: i32 = 20;
        const CURSOR_COLS: i32 = 2;
        const ICON_COLS: i32 = 2;

        fn draw_box(&mut self, at: &Layout, title: Option<&str>) {
            self.0.push(alloc::format!(
                "box {} {} {} {} {title:?}",
                at.x,
                at.y,
                at.w,
                at.h
            ));
        }

        fn clear_box(&mut self, _at: &Layout) {
            self.0.push("clear".to_string());
        }

        fn draw_row(&mut self, _at: &Layout, row: &RowView) {
            self.0.push(alloc::format!(
                "row {} {} {:?} {} {}",
                row.y,
                row.label,
                row.choice,
                row.selected,
                row.disabled
            ));
        }

        fn draw_marks(&mut self, _at: &Layout, up: bool, down: bool) {
            self.0.push(alloc::format!("marks {up} {down}"));
        }

        fn draw_description(&mut self, _at: &Layout, text: &str) {
            self.0.push(alloc::format!("description {text}"));
        }
    }

    impl Log {
        fn take(&mut self) -> Vec<String> {
            core::mem::take(&mut self.0)
        }
    }

    #[test]
    fn lays_out_a_titled_menu_with_choices() {
        // An OPTIONS menu: MODE / MUSIC / SFX / BACK, titled (Frogman ∞'s, as it was
        // drawn before agb-ui).
        static OPTIONS: [Row<R>; 4] = [
            Row::choice(R::Mode, "MODE", &["CHILL", "CLASSIC"]),
            Row::choice(R::Music, "MUSIC", &["ON", "OFF"]),
            Row::choice(R::Play, "SFX", &["ON", "OFF"]),
            Row::back(R::Back),
        ];
        let at = Layout::new(
            &Log::default(),
            &Menu::new(&OPTIONS),
            &Fixed,
            &Frame::titled("OPTIONS"),
        );
        // 5 to the labels, 5 of them, a blank, ◀ CLASSIC ▶, a blank and the border: 22, and
        // one more to centre the 7-letter title.
        assert_eq!((at.w, at.h), (23, 12));
        assert_eq!((at.x, at.y), (3, 4));
        assert_eq!((at.label_x, at.choice_x, at.cursor_x), (8, 14, 5));
        assert_eq!(at.title_x("OPTIONS"), 11);
        assert_eq!((at.title_y, at.list_y), (Some(5), 7));
        assert_eq!(at.description_y, None);
    }

    #[test]
    fn lays_out_a_header_above_the_rows() {
        // A GAME OVER box: a 7-line run summary 18 wide, then PLAY AGAIN / QUIT
        // (Frogman ∞'s, as it was drawn before agb-ui).
        static GAME_OVER: [Row<R>; 2] = [
            Row::action(R::Play, "PLAY AGAIN"),
            Row::action(R::Shop, "QUIT"),
        ];
        let frame = Frame {
            header_rows: 7,
            header_cols: 18,
            ..Frame::titled("GAME OVER")
        };
        let at = Layout::new(&Log::default(), &Menu::new(&GAME_OVER), &Fixed, &frame);
        // 22 wide, and one more to centre the 9-letter title.
        assert_eq!((at.x, at.y, at.w, at.h), (3, 2, 23, 16));
        assert_eq!((at.title_y, at.header_y, at.right), (Some(3), 5, 24));
        assert_eq!((at.row_y(0), at.row_y(1)), (13, 15));
    }

    #[test]
    fn centres_the_title_in_the_box() {
        // Every title, odd or even, with as many blank columns either side inside the border;
        // an untitled box keeps its width.
        let menu = Menu::new(&ROWS);
        let untitled = Layout::new(&Log::default(), &menu, &Fixed, &Frame::UNTITLED);
        for title in [
            "T",
            "UP",
            "~ PAUSED ~",
            "~ DEBUG ~",
            "~ THE TINIES ~",
            "~ THE TINIES~",
        ] {
            let at = Layout::new(&Log::default(), &menu, &Fixed, &Frame::titled(title));
            let len = title.chars().count() as i32;
            let (left, right) = (
                at.title_x(title) - at.x,
                at.x + at.w - (at.title_x(title) + len),
            );
            assert_eq!(left, right, "{title}: box {} wide", at.w);
            assert!(at.w - untitled.w <= 1 || len + 4 > untitled.w, "{title}");
        }
        // An even title in an even box, so both centre: the box on the 30-column screen too.
        let at = Layout::new(
            &Log::default(),
            &menu,
            &Fixed,
            &Frame::titled("~ THE TINIES ~"),
        );
        assert_eq!(at.w % 2, 0);
        assert_eq!(at.x, (Log::COLS - at.w) / 2);
        assert_eq!(2 * at.x + at.w, Log::COLS);
    }

    #[test]
    fn value_cols_holds_the_box_still() {
        struct Count(u32);
        impl RowState<R> for Count {
            fn value(&self, id: R) -> Value {
                if id == R::Play {
                    Value::Number(self.0)
                } else {
                    Value::None
                }
            }
        }
        let menu = Menu::new(&ROWS);
        let frame = Frame {
            value_cols: 3,
            ..Frame::UNTITLED
        };
        let small = Layout::new(&Log::default(), &menu, &Count(7), &frame);
        let big = Layout::new(&Log::default(), &menu, &Count(999), &frame);
        assert_eq!(small, big);
        // Without it, a digit more is a wider box.
        let small = Layout::new(&Log::default(), &menu, &Count(7), &Frame::UNTITLED);
        let big = Layout::new(&Log::default(), &menu, &Count(99), &Frame::UNTITLED);
        assert_eq!(big.w, small.w + 1);
    }

    #[test]
    fn cuts_a_description_to_the_box() {
        struct Long;
        impl RowState<R> for Long {
            fn description(&self, _row: &Row<R>, out: &mut Line) {
                let _ = out.write_str("A DESCRIPTION FAR TOO LONG FOR ANY BOX");
            }
        }
        let mut log = Log::default();
        let mut menu = Menu::new(&ROWS);
        menu.select(R::Shop);
        let shown = draw(&mut None, &mut log, &menu, &Long, &Frame::UNTITLED);
        let line = log.take().pop().unwrap_or_default();
        let text = line.trim_start_matches("description ");
        assert_eq!(text.chars().count() as i32, shown.at.w - 4);
    }

    #[test]
    fn makes_room_for_icons_values_and_descriptions() {
        struct Price;
        impl RowState<R> for Price {
            fn value(&self, id: R) -> Value {
                if id == R::Shop {
                    Value::Number(120)
                } else {
                    Value::None
                }
            }
        }
        let menu = Menu::new(&ROWS).with_window(3);
        let at = Layout::new(&Log::default(), &menu, &Price, &Frame::UNTITLED);
        assert_eq!(at.label_x - at.x, 8);
        // Labels, ◀ CLASSIC ▶, a blank, 3 digits, a blank, the border.
        assert_eq!(at.w, 8 + 5 + 1 + 9 + 1 + 3 + 2);
        assert_eq!(at.h, 3 + 2 * 3 + 2);
        assert_eq!((at.up_y, at.down_y), (at.y + 1, at.y + 7));
        assert_eq!(at.description_y, Some(at.y + 8));
    }

    #[test]
    fn draws_only_what_changed() {
        let mut log = Log::default();
        let mut drawn = None;
        let mut menu = Menu::new(&ROWS).with_window(3);
        let frame = Frame::titled("T");
        let shown = draw(&mut drawn, &mut log, &menu, &Fixed, &frame);
        assert!(shown.box_drawn && shown.rows_drawn);
        assert_eq!(
            log.take(),
            vec![
                "box 2 4 25 12 Some(\"T\")",
                "row 7 MODE Some(\"CHILL\") true false",
                "row 9 MUSIC Some(\"ON\") false false",
                "row 11 PLAY None false false",
                "marks false true",
            ]
        );
        let shown = draw(&mut drawn, &mut log, &menu, &Fixed, &frame);
        assert!(!shown.box_drawn && !shown.rows_drawn);
        assert!(log.take().is_empty());

        // A move redraws the two rows; a change, the one.
        menu.update(
            Input {
                down: true,
                ..Input::default()
            },
            &Fixed,
        );
        draw(&mut drawn, &mut log, &menu, &Fixed, &frame);
        assert_eq!(
            log.take(),
            vec![
                "row 7 MODE Some(\"CHILL\") false false",
                "row 9 MUSIC Some(\"ON\") true false",
            ]
        );
        menu.update(
            Input {
                right: true,
                ..Input::default()
            },
            &Fixed,
        );
        draw(&mut drawn, &mut log, &menu, &Fixed, &frame);
        assert_eq!(log.take(), vec!["row 9 MUSIC Some(\"OFF\") true false"]);

        // Scrolling redraws every row, the marks and the description.
        menu.select(R::Shop);
        let shown = draw(&mut drawn, &mut log, &menu, &Fixed, &frame);
        assert_eq!(
            shown.cursor,
            Some(Cursor {
                y: 11,
                index: 3,
                disabled: false
            })
        );
        assert_eq!(
            log.take(),
            vec![
                "row 7 MUSIC Some(\"OFF\") false false",
                "row 9 PLAY None false false",
                "row 11 SHOP None true false",
                "marks true true",
                "description BUY THINGS",
            ]
        );

        // Another title is another box.
        draw(&mut drawn, &mut log, &menu, &Fixed, &Frame::titled("U"));
        assert_eq!(log.take()[..2], ["clear", "box 2 4 25 12 Some(\"U\")"]);
    }

    #[test]
    fn draws_disabled_rows_and_hides_the_list() {
        struct Off;
        impl RowState<R> for Off {
            fn disabled(&self, id: R) -> bool {
                id == R::Mode
            }
        }
        let mut log = Log::default();
        let mut drawn = None;
        let menu = Menu::new(&ROWS).with_window(1);
        let shown = draw(&mut drawn, &mut log, &menu, &Off, &Frame::UNTITLED);
        assert_eq!(shown.cursor.map(|c| c.disabled), Some(true));
        assert_eq!(log.take()[1], "row 8 MODE Some(\"CHILL\") true true");

        let hidden = Frame {
            list: false,
            ..Frame::UNTITLED
        };
        let shown = draw(&mut drawn, &mut log, &menu, &Off, &hidden);
        assert!(shown.box_drawn && shown.cursor.is_none());
        assert_eq!(log.take(), vec!["clear", "box 2 6 25 7 None"]);
    }
}
