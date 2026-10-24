use embedded_graphics::{
    Drawable,
    geometry::{Point, Size},
    pixelcolor::BinaryColor,
    primitives::{Line, Primitive, PrimitiveStyle, Rectangle},
    text::Text,
};

use super::{Button, Canvas, Response, Screen};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Lower = 0,
    Upper = 1,
    Symbols = 2,
}

#[derive(Clone, Copy)]
enum Action {
    Char(u8),
    Space,
    Backspace,
    CursorLeft,
    CursorRight,
    Page(Page),
    Cancel,
    Done,
}

/// A key that isn't just a single character.
struct Key {
    label: &'static str,
    /// in cells
    width: u8,
    action: Action,
}

/// A keyboard row: single-cell character keys, followed by wider special keys.
struct Row {
    chars: &'static [u8],
    keys: &'static [Key],
}

impl Row {
    const fn len(&self) -> usize {
        self.chars.len() + self.keys.len()
    }

    /// Total width in cells.
    const fn width(&self) -> usize {
        let mut width = self.chars.len();
        let mut i = 0;
        while i < self.keys.len() {
            width += self.keys[i].width as usize;
            i += 1;
        }
        width
    }

    /// The `i`th key's label, starting cell, width in cells, and action.
    fn key(&self, i: usize) -> (&'static str, usize, usize, Action) {
        if let Some(c) = self.chars.get(i) {
            // PANIC SAFETY: the keyboard layout is all ASCII
            let label = core::str::from_utf8(core::slice::from_ref(c)).unwrap();
            return (label, i, 1, Action::Char(*c));
        }
        let i = i - self.chars.len();
        let start = self.chars.len()
            + self.keys[..i]
                .iter()
                .map(|k| k.width as usize)
                .sum::<usize>();
        let key = &self.keys[i];
        (key.label, start, key.width as usize, key.action)
    }

    /// Index of the key covering `x2`, a horizontal position in half-cells; the last key if the
    /// row is too short to reach it.
    fn key_at(&self, x2: usize) -> usize {
        (0..self.len())
            .find(|&i| {
                let (_, start, width, _) = self.key(i);
                x2 < (start + width) * 2
            })
            .unwrap_or(self.len() - 1)
    }
}

const fn key(label: &'static str, width: u8, action: Action) -> Key {
    Key {
        label,
        width,
        action,
    }
}

const CURSOR_LEFT: Key = key("«", 2, Action::CursorLeft);
const CURSOR_RIGHT: Key = key("»", 2, Action::CursorRight);
const BACKSPACE: Key = key("del", 2, Action::Backspace);
const SPACE: Key = key("space", 9, Action::Space);
const CANCEL: Key = key("esc", 3, Action::Cancel);
const DONE: Key = key("OK", 3, Action::Done);
const TO_LOWER: Key = key("abc", 3, Action::Page(Page::Lower));
const TO_UPPER: Key = key("ABC", 3, Action::Page(Page::Upper));
const TO_SYMBOLS: Key = key("123", 3, Action::Page(Page::Symbols));

/// Indexed by `Page`.  Every page's bottom row has the same shape, so switching pages leaves the
/// selection where it was.
#[rustfmt::skip]
const PAGES: [[Row; 3]; 3] = [
    [
        Row { chars: b"abcdefghijklmnopqrstu", keys: &[] },
        Row { chars: b"vwxyz.,'-!?@&:;", keys: &[CURSOR_LEFT, CURSOR_RIGHT, BACKSPACE] },
        Row { chars: b"", keys: &[TO_UPPER, TO_SYMBOLS, SPACE, CANCEL, DONE] },
    ],
    [
        Row { chars: b"ABCDEFGHIJKLMNOPQRSTU", keys: &[] },
        Row { chars: b"VWXYZ.,'-!?@&:;", keys: &[CURSOR_LEFT, CURSOR_RIGHT, BACKSPACE] },
        Row { chars: b"", keys: &[TO_LOWER, TO_SYMBOLS, SPACE, CANCEL, DONE] },
    ],
    [
        Row { chars: b"0123456789()[]<>+=*#%", keys: &[] },
        Row { chars: b"\"$^_~|\\`{}/", keys: &[CURSOR_LEFT, CURSOR_RIGHT, BACKSPACE] },
        Row { chars: b"", keys: &[TO_LOWER, TO_UPPER, SPACE, CANCEL, DONE] },
    ],
];

const CELL_WIDTH: u32 = 6;
const CELLS: usize = 21;
const KEY_ROW_HEIGHT: u32 = 7;
/// Centers the keyboard horizontally.
const KEYBOARD_X: i32 = ((crate::hardware::DISPLAY_WIDTH - CELLS as u32 * CELL_WIDTH) / 2) as i32;
/// The text line, then a divider line, then the keys.
const KEYBOARD_Y: i32 = super::FONT.character_size.height as i32 + 1;

const _: () = {
    let mut p = 0;
    while p < PAGES.len() {
        let mut r = 0;
        while r < PAGES[p].len() {
            assert!(PAGES[p][r].width() <= CELLS, "keyboard row too wide");
            r += 1;
        }
        p += 1;
    }
    assert!(
        KEYBOARD_Y as u32 + 3 * KEY_ROW_HEIGHT <= crate::hardware::DISPLAY_HEIGHT,
        "keyboard too tall"
    );
};

/// Edit a string of up to `N` characters with an on-screen keyboard.
///
/// The D-pad moves around the keyboard and Center presses the selected key.  Typing inserts at the
/// cursor (the highlighted spot in the text), which "«"/"»" move; "del" removes the character
/// before it.  "abc"/"ABC"/"123" switch pages, "esc" cancels (restoring the text from when the
/// screen was entered), and "OK" accepts.  Trailing spaces are trimmed on accept.
///
/// It can sit directly in a [`super::menu::Menu`], or be embedded in another screen that calls
/// [`Screen::enter`] when it starts editing and checks [`TextInput::accepted`] once it exits.
pub struct TextInput<const N: usize> {
    prompt: &'static str,
    /// only ever holds printable ASCII
    text: heapless::Vec<u8, N>,
    original: heapless::Vec<u8, N>,
    /// where typing inserts, from 0 (before the first character) to `text.len()` (after the last)
    cursor: usize,
    accepted: bool,
    page: Page,
    row: usize,
    /// index of the selected key within `row`
    column: usize,
}

impl<const N: usize> TextInput<N> {
    pub fn new(prompt: &'static str, initial: &str) -> Self {
        let mut this = Self {
            prompt,
            text: heapless::Vec::new(),
            original: heapless::Vec::new(),
            cursor: 0,
            accepted: false,
            page: Page::Lower,
            row: 0,
            column: 0,
        };
        this.set_value(initial);
        this
    }

    pub fn value(&self) -> &str {
        core::str::from_utf8(&self.text).unwrap()
    }

    /// Replace the text, dropping non-printable characters and anything past `N`.
    pub fn set_value(&mut self, value: &str) {
        self.text.clear();
        for b in value
            .bytes()
            .filter(|&b| b.is_ascii_graphic() || b == b' ')
            .take(N)
        {
            // PANIC SAFETY: take(N) keeps this within capacity
            self.text.push(b).unwrap();
        }
        self.cursor = self.text.len();
    }

    /// Whether the last edit ended with "OK" rather than "esc".
    pub fn accepted(&self) -> bool {
        self.accepted
    }

    fn rows(&self) -> &'static [Row; 3] {
        &PAGES[self.page as usize]
    }

    /// Move to `row` on the current page, picking the key under the middle of the selected one.
    fn move_to_row(&mut self, row: usize) {
        let (_, start, width, _) = self.rows()[self.row].key(self.column);
        self.row = row;
        self.column = self.rows()[row].key_at(start * 2 + width);
    }

    fn insert(&mut self, c: u8) -> Response {
        match self.text.insert(self.cursor, c) {
            Ok(()) => {
                self.cursor += 1;
                Response::Redraw
            }
            Err(_) => Response::Ignored,
        }
    }

    fn press(&mut self) -> Response {
        let (_, _, _, action) = self.rows()[self.row].key(self.column);
        match action {
            Action::Char(c) => self.insert(c),
            Action::Space => self.insert(b' '),
            Action::Backspace if self.cursor > 0 => {
                self.cursor -= 1;
                self.text.remove(self.cursor);
                Response::Redraw
            }
            Action::CursorLeft if self.cursor > 0 => {
                self.cursor -= 1;
                Response::Redraw
            }
            Action::CursorRight if self.cursor < self.text.len() => {
                self.cursor += 1;
                Response::Redraw
            }
            Action::Backspace | Action::CursorLeft | Action::CursorRight => Response::Ignored,
            Action::Page(page) => {
                self.page = page;
                self.move_to_row(self.row);
                Response::Redraw
            }
            Action::Cancel => {
                self.text = self.original.clone();
                self.cursor = self.text.len();
                self.accepted = false;
                Response::Exit
            }
            Action::Done => {
                while self.text.last() == Some(&b' ') {
                    self.text.pop();
                }
                self.cursor = self.cursor.min(self.text.len());
                self.accepted = true;
                Response::Exit
            }
        }
    }

    fn draw_text_line(&self, canvas: &mut Canvas) {
        let char_width = super::FONT.character_size.width;
        let mut x = 0;
        if !self.prompt.is_empty() {
            Text::with_text_style(self.prompt, Point::zero(), super::TEXT, super::TOP_LEFT)
                .draw(canvas)
                .unwrap();
            x = (self.prompt.len() as u32 + 1) * char_width;
        }

        // scroll just enough to keep the cursor on screen
        let columns =
            (crate::hardware::DISPLAY_WIDTH.saturating_sub(x) / char_width).max(1) as usize;
        let start = (self.cursor + 1)
            .saturating_sub(columns)
            .min(self.text.len());
        let end = self.text.len().min(start + columns);
        Text::with_text_style(
            core::str::from_utf8(&self.text[start..end]).unwrap(),
            Point::new(x as i32, 0),
            super::TEXT,
            super::TOP_LEFT,
        )
        .draw(canvas)
        .unwrap();

        // highlight the character the cursor is before, or a blank cell at the end
        if self.cursor < N {
            let under_cursor = self.text.get(self.cursor).copied().unwrap_or(b' ');
            Text::with_text_style(
                // PANIC SAFETY: `text` and the space are ASCII
                core::str::from_utf8(&[under_cursor]).unwrap(),
                Point::new((x + (self.cursor - start) as u32 * char_width) as i32, 0),
                super::TEXT_HIGHLIGHTED,
                super::TOP_LEFT,
            )
            .draw(canvas)
            .unwrap();
        }

        Line::new(
            Point::new(0, KEYBOARD_Y - 1),
            Point::new(crate::hardware::DISPLAY_WIDTH as i32 - 1, KEYBOARD_Y - 1),
        )
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
        .draw(canvas)
        .unwrap();
    }
}

impl<const N: usize> Screen for TextInput<N> {
    fn enter(&mut self) {
        self.original = self.text.clone();
        self.cursor = self.text.len();
        self.accepted = false;
        self.page = Page::Lower;
        self.row = 0;
        self.column = 0;
    }

    fn draw(&self, canvas: &mut Canvas) {
        self.draw_text_line(canvas);

        let glyph = super::SMALL_FONT.character_size;
        for (r, row) in self.rows().iter().enumerate() {
            let y = KEYBOARD_Y + (r as u32 * KEY_ROW_HEIGHT) as i32;
            for i in 0..row.len() {
                let (label, start, width, _) = row.key(i);
                let left = KEYBOARD_X + (start as u32 * CELL_WIDTH) as i32;
                let span = width as u32 * CELL_WIDTH;

                let style = if r == self.row && i == self.column {
                    Rectangle::new(Point::new(left, y), Size::new(span, KEY_ROW_HEIGHT))
                        .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                        .draw(canvas)
                        .unwrap();
                    super::SMALL_TEXT_INVERTED
                } else {
                    super::SMALL_TEXT
                };

                let label_width = label.chars().count() as u32 * glyph.width;
                let x = left + (span.saturating_sub(label_width) / 2) as i32;
                Text::with_text_style(label, Point::new(x, y), style, super::TOP_LEFT)
                    .draw(canvas)
                    .unwrap();
            }
        }
    }

    fn handle(&mut self, button: Button) -> Response {
        let len = self.rows()[self.row].len();
        match button {
            Button::Left => self.column = (self.column + len - 1) % len,
            Button::Right => self.column = (self.column + 1) % len,
            Button::Up => self.move_to_row((self.row + 2) % 3),
            Button::Down => self.move_to_row((self.row + 1) % 3),
            Button::Center => return self.press(),
        }
        Response::Redraw
    }
}
