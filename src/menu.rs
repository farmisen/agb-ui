use alloc::vec;
use alloc::vec::Vec;
use core::fmt::Write;

use crate::Line;

/// The label of a [`Row::back`].
pub const BACK: &str = "BACK";

/// A row of a menu, in a fixed table: [`Row::action`], [`Row::choice`] or [`Row::back`],
/// with [`Row::with_icon`] and [`Row::with_description`] to add to it. `Id` names it:
/// an enum of the menu's rows.
#[derive(Clone, Copy)]
pub struct Row<Id: Copy + 'static> {
    pub id: Id,
    pub label: &'static str,
    /// What ◀ / ▶ cycle through; empty for a row that is only picked.
    pub choices: &'static [&'static str],
    /// The game's icon, drawn left of the label (a sprite or tile number of its own).
    pub icon: Option<u16>,
    /// The line under the list while the row is selected; empty for none. A menu has
    /// the line if any row has one, as wide as the widest; [`RowState::description`] can
    /// say something else there.
    pub description: &'static str,
    back: bool,
}

impl<Id: Copy> Row<Id> {
    /// A row that is only picked.
    pub const fn action(id: Id, label: &'static str) -> Self {
        Self {
            id,
            label,
            choices: &[],
            icon: None,
            description: "",
            back: false,
        }
    }

    /// A row whose choice ◀ / ▶ cycle; A picks it too.
    pub const fn choice(id: Id, label: &'static str, choices: &'static [&'static str]) -> Self {
        Self {
            choices,
            ..Self::action(id, label)
        }
    }

    /// BACK: the menu's way back, its last row. Picking it is [`Event::Back`], as B is.
    pub const fn back(id: Id) -> Self {
        Self {
            back: true,
            ..Self::action(id, BACK)
        }
    }

    pub const fn with_icon(self, icon: u16) -> Self {
        Self {
            icon: Some(icon),
            ..self
        }
    }

    pub const fn with_description(self, description: &'static str) -> Self {
        Self {
            description,
            ..self
        }
    }

    pub const fn is_back(&self) -> bool {
        self.back
    }
}

/// What a row shows right-aligned, if anything: a price, a count. Asked of
/// [`RowState::value`].
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum Value {
    #[default]
    None,
    Number(u32),
    Text(&'static str),
}

/// What the game says about rows that change, asked when a menu takes input and when
/// it is drawn. Each default is a row that never changes.
pub trait RowState<Id: Copy> {
    /// Drawn grey; A and ◀ / ▶ on it are refused.
    fn disabled(&self, _id: Id) -> bool {
        false
    }

    /// Shown right-aligned on the row.
    fn value(&self, _id: Id) -> Value {
        Value::None
    }

    /// The description line while `row` is selected, in a menu that has the line (a row
    /// with a [`Row::description`]). What doesn't fit the box is dropped.
    fn description(&self, row: &Row<Id>, out: &mut Line) {
        let _ = out.write_str(row.description);
    }
}

/// Rows that never change: none disabled, no values, their own descriptions.
pub struct Fixed;

impl<Id: Copy> RowState<Id> for Fixed {}

/// The buttons pressed this frame, as a menu reads them.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct Input {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub a: bool,
    pub b: bool,
}

#[cfg(feature = "agb")]
impl From<&agb::input::ButtonController> for Input {
    fn from(input: &agb::input::ButtonController) -> Self {
        use agb::input::Button;
        Self {
            up: input.is_just_pressed(Button::Up),
            down: input.is_just_pressed(Button::Down),
            left: input.is_just_pressed(Button::Left),
            right: input.is_just_pressed(Button::Right),
            a: input.is_just_pressed(Button::A),
            b: input.is_just_pressed(Button::B),
        }
    }
}

/// What a frame's input did to a menu. The game plays its sound: a move or a change
/// ticks, a pick confirms, back goes back, a refusal buzzes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Event<Id> {
    /// ▲ / ▼ moved the selection.
    Moved,
    /// A picked the row.
    Picked(Id),
    /// ◀ / ▶ changed the row's choice.
    Changed(Id),
    /// B, or A on the BACK row.
    Back,
    /// A, or ◀ / ▶ on a row with choices, on a disabled row.
    Refused(Id),
}

/// A menu's state: its rows, the one selected, the window of rows shown and each row's
/// choice.
pub struct Menu<Id: Copy + 'static> {
    rows: &'static [Row<Id>],
    selected: usize,
    /// The first row shown.
    top: usize,
    /// Most rows shown at once.
    window: usize,
    /// B goes back though there's no BACK row.
    back_button: bool,
    /// Each row's choice, an index into its `choices`.
    values: Vec<u8>,
}

impl<Id: Copy + PartialEq> Menu<Id> {
    /// Every row shown, the first selected, each choice its first. B goes back if the
    /// last row is BACK.
    ///
    /// Panics if `rows` is empty, has BACK anywhere but last, or a row with more than
    /// 256 choices (each row's choice is a byte).
    pub fn new(rows: &'static [Row<Id>]) -> Self {
        assert!(!rows.is_empty(), "a menu needs a row");
        assert!(
            rows.iter().all(|r| r.choices.len() <= 256),
            "a row has more than 256 choices"
        );
        assert!(
            rows[..rows.len() - 1].iter().all(|r| !r.back),
            "BACK goes last"
        );
        Self {
            rows,
            selected: 0,
            top: 0,
            window: rows.len(),
            back_button: false,
            values: vec![0; rows.len()],
        }
    }

    /// B goes back without a BACK row: a pause menu's RESUME is its way back.
    pub fn with_back_button(mut self) -> Self {
        self.back_button = true;
        self
    }

    /// Show at most `n` rows at once, scrolling to keep the selected one in view.
    pub fn with_window(mut self, n: usize) -> Self {
        self.window = n.clamp(1, self.rows.len());
        self.scroll_to_selected();
        self
    }

    pub fn rows(&self) -> &'static [Row<Id>] {
        self.rows
    }

    /// Whether B goes back.
    pub fn has_back(&self) -> bool {
        self.back_button || self.rows[self.rows.len() - 1].back
    }

    pub fn selected(&self) -> Id {
        self.rows[self.selected].id
    }

    pub fn selected_index(&self) -> usize {
        self.selected
    }

    pub fn select(&mut self, id: Id) {
        self.selected = self.index(id);
        self.scroll_to_selected();
    }

    /// The index of row `id` in the table.
    pub fn index(&self, id: Id) -> usize {
        match self.rows.iter().position(|r| r.id == id) {
            Some(i) => i,
            None => {
                debug_assert!(false, "no such row in this menu");
                0
            }
        }
    }

    /// Row `id`'s choice, an index into its `choices`.
    pub fn value(&self, id: Id) -> usize {
        self.value_at(self.index(id))
    }

    /// Row `i`'s choice, an index into its `choices`.
    pub fn value_at(&self, i: usize) -> usize {
        usize::from(self.values[i])
    }

    /// Set row `id`'s choice, an index into its `choices`.
    pub fn set_value(&mut self, id: Id, v: usize) {
        let i = self.index(id);
        debug_assert!(v < self.rows[i].choices.len(), "no such choice");
        self.values[i] = v as u8;
    }

    /// Every row's choice, for telling whether any changed.
    pub fn values(&self) -> &[u8] {
        &self.values
    }

    /// The first row shown.
    pub fn top(&self) -> usize {
        self.top
    }

    /// How many rows show at once.
    pub fn shown(&self) -> usize {
        self.window
    }

    /// Rows are hidden above the ones shown.
    pub fn more_above(&self) -> bool {
        self.top > 0
    }

    /// Rows are hidden below the ones shown.
    pub fn more_below(&self) -> bool {
        self.top + self.window < self.rows.len()
    }

    /// Apply this frame's input. B wins over everything; a pick, a change or a refusal
    /// wins over a move made the same frame.
    pub fn update(
        &mut self,
        input: impl Into<Input>,
        state: &impl RowState<Id>,
    ) -> Option<Event<Id>> {
        let input = input.into();
        if input.b && self.has_back() {
            return Some(Event::Back);
        }
        let n = self.rows.len();
        let before = self.selected;
        if input.up {
            self.selected = step(self.selected, n, false);
        }
        if input.down {
            self.selected = step(self.selected, n, true);
        }
        self.scroll_to_selected();
        let moved = (self.selected != before).then_some(Event::Moved);
        let i = self.selected;
        let row = &self.rows[i];
        if !row.choices.is_empty() && (input.left || input.right) {
            if state.disabled(row.id) {
                return Some(Event::Refused(row.id));
            }
            self.values[i] = step(self.value_at(i), row.choices.len(), input.right) as u8;
            return Some(Event::Changed(row.id));
        }
        if input.a {
            return Some(if state.disabled(row.id) {
                Event::Refused(row.id)
            } else if row.back {
                Event::Back
            } else {
                Event::Picked(row.id)
            });
        }
        moved
    }

    fn scroll_to_selected(&mut self) {
        if self.selected < self.top {
            self.top = self.selected;
        } else if self.selected >= self.top + self.window {
            self.top = self.selected + 1 - self.window;
        }
    }
}

/// `i` moved one step through `0..n`, wrapping (no `%`: on the GBA it's a software
/// division).
fn step(i: usize, n: usize, forward: bool) -> usize {
    match (forward, i) {
        (true, _) if i + 1 == n => 0,
        (true, _) => i + 1,
        (false, 0) => n - 1,
        (false, _) => i - 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum R {
        A,
        B,
        C,
        D,
        Back,
    }

    static ROWS: [Row<R>; 5] = [
        Row::action(R::A, "A"),
        Row::choice(R::B, "B", &["X", "Y", "Z"]),
        Row::action(R::C, "C"),
        Row::action(R::D, "D").with_description("DEE"),
        Row::back(R::Back),
    ];

    struct Off(R);

    impl RowState<R> for Off {
        fn disabled(&self, id: R) -> bool {
            id == self.0
        }
    }

    fn press(f: impl FnOnce(&mut Input)) -> Input {
        let mut input = Input::default();
        f(&mut input);
        input
    }

    #[test]
    fn moves_wrapping() {
        let mut m = Menu::new(&ROWS);
        assert_eq!(m.update(press(|i| i.up = true), &Fixed), Some(Event::Moved));
        assert_eq!(m.selected(), R::Back);
        assert_eq!(
            m.update(press(|i| i.down = true), &Fixed),
            Some(Event::Moved)
        );
        assert_eq!(m.selected(), R::A);
        assert_eq!(m.update(Input::default(), &Fixed), None);
    }

    #[test]
    fn changes_choices_wrapping() {
        let mut m = Menu::new(&ROWS);
        m.select(R::B);
        assert_eq!(
            m.update(press(|i| i.left = true), &Fixed),
            Some(Event::Changed(R::B))
        );
        assert_eq!(m.value(R::B), 2);
        assert_eq!(
            m.update(press(|i| i.right = true), &Fixed),
            Some(Event::Changed(R::B))
        );
        assert_eq!(m.value(R::B), 0);
        // ◀ / ▶ do nothing on a row without choices.
        m.select(R::A);
        assert_eq!(m.update(press(|i| i.right = true), &Fixed), None);
    }

    #[test]
    fn picks_and_goes_back() {
        let mut m = Menu::new(&ROWS);
        assert_eq!(
            m.update(press(|i| i.a = true), &Fixed),
            Some(Event::Picked(R::A))
        );
        assert_eq!(m.update(press(|i| i.b = true), &Fixed), Some(Event::Back));
        m.select(R::Back);
        assert_eq!(m.update(press(|i| i.a = true), &Fixed), Some(Event::Back));
    }

    #[test]
    fn b_needs_a_way_back() {
        static NO_BACK: [Row<R>; 2] = [Row::action(R::A, "A"), Row::action(R::B, "B")];
        let mut m = Menu::new(&NO_BACK);
        assert!(!m.has_back());
        assert_eq!(m.update(press(|i| i.b = true), &Fixed), None);
        let mut m = Menu::new(&NO_BACK).with_back_button();
        assert_eq!(m.update(press(|i| i.b = true), &Fixed), Some(Event::Back));
    }

    #[test]
    fn refuses_disabled_rows_but_selects_them() {
        let mut m = Menu::new(&ROWS);
        assert_eq!(
            m.update(press(|i| i.down = true), &Off(R::B)),
            Some(Event::Moved)
        );
        assert_eq!(m.selected(), R::B);
        assert_eq!(
            m.update(press(|i| i.a = true), &Off(R::B)),
            Some(Event::Refused(R::B))
        );
        assert_eq!(
            m.update(press(|i| i.right = true), &Off(R::B)),
            Some(Event::Refused(R::B))
        );
        assert_eq!(m.value(R::B), 0);
    }

    #[test]
    fn scrolls_to_keep_the_selection_shown() {
        let mut m = Menu::new(&ROWS).with_window(3);
        assert_eq!((m.top(), m.shown()), (0, 3));
        assert!(!m.more_above() && m.more_below());
        for _ in 0..3 {
            m.update(press(|i| i.down = true), &Fixed);
        }
        assert_eq!((m.selected(), m.top()), (R::D, 1));
        assert!(m.more_above() && m.more_below());
        m.update(press(|i| i.down = true), &Fixed);
        assert_eq!(m.top(), 2);
        assert!(m.more_above() && !m.more_below());
        // Wrapping to the first row scrolls back to the top.
        m.update(press(|i| i.down = true), &Fixed);
        assert_eq!((m.selected(), m.top()), (R::A, 0));
        m.select(R::Back);
        assert_eq!(m.top(), 2);
    }

    #[test]
    fn a_window_past_the_rows_shows_them_all() {
        let m = Menu::new(&ROWS).with_window(9);
        assert_eq!(m.shown(), 5);
        assert!(!m.more_below());
    }

    #[test]
    fn up_and_down_together_stay_put() {
        let mut m = Menu::new(&ROWS);
        let both = press(|i| {
            i.up = true;
            i.down = true;
        });
        assert_eq!(m.update(both, &Fixed), None);
        assert_eq!(m.selected(), R::A);
    }

    #[test]
    fn a_window_shows_a_row_at_least() {
        let m = Menu::new(&ROWS).with_window(0);
        assert_eq!(m.shown(), 1);
    }

    #[test]
    #[should_panic(expected = "more than 256 choices")]
    fn choices_fit_a_byte() {
        static MANY: [&str; 257] = [""; 257];
        static BAD: [Row<R>; 1] = [Row::choice(R::A, "A", &MANY)];
        Menu::new(&BAD);
    }

    #[test]
    #[should_panic(expected = "BACK goes last")]
    fn back_goes_last() {
        static BAD: [Row<R>; 2] = [Row::back(R::Back), Row::action(R::A, "A")];
        Menu::new(&BAD);
    }
}
