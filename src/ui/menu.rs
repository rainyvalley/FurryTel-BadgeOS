use embedded_graphics::{
    Drawable,
    geometry::{Point, Size},
    pixelcolor::BinaryColor,
    primitives::{Primitive, PrimitiveStyle, Rectangle},
    text::{Alignment, Baseline, Text, TextStyleBuilder},
};

use super::{Button, Canvas, Response, Screen};

const ROW_HEIGHT: u32 = super::FONT.character_size.height;

pub struct Item<'a> {
    pub label: &'static str,
    pub screen: &'a mut dyn Screen,
}

impl<'a> Item<'a> {
    pub fn new(label: &'static str, screen: &'a mut dyn Screen) -> Self {
        Self { label, screen }
    }
}

/// A scrolling list of items, each of which opens another screen.
///
/// Up/Down move the selection, Center/Right open the selected item, and Left exits the menu.  When
/// the opened screen exits, the menu comes back.  A `Menu` is itself a [`Screen`], so submenus are
/// just items whose screen is another `Menu`.
pub struct Menu<'a, const N: usize> {
    /// A line of small text above the items.
    header: Option<&'static str>,
    items: [Item<'a>; N],
    selected: usize,
    /// whether the selected item's screen is currently being shown
    open: bool,
}

impl<'a, const N: usize> Menu<'a, N> {
    pub fn new(items: [Item<'a>; N]) -> Self {
        const { assert!(N > 0, "a menu needs at least one item") };
        Self {
            header: None,
            items,
            selected: 0,
            open: false,
        }
    }

    /// A menu with a small hint line along the top, which costs a row of items.
    pub fn with_header(header: &'static str, items: [Item<'a>; N]) -> Self {
        Self {
            header: Some(header),
            ..Self::new(items)
        }
    }
}

impl<const N: usize> Screen for Menu<'_, N> {
    fn draw(&self, canvas: &mut Canvas) {
        if self.open {
            return self.items[self.selected].screen.draw(canvas);
        }

        let top_y = match self.header {
            Some(header) => {
                Text::with_text_style(header, Point::zero(), super::SMALL_TEXT, super::TOP_LEFT)
                    .draw(canvas)
                    .unwrap();
                super::SMALL_FONT.character_size.height as i32
            }
            None => 0,
        };
        let visible =
            ((crate::hardware::DISPLAY_HEIGHT as i32 - top_y) / ROW_HEIGHT as i32) as usize;

        // keep the selection in the middle row, except near the ends of the list where that would
        // leave blank rows
        let top = self
            .selected
            .saturating_sub(visible / 2)
            .min(N.saturating_sub(visible));
        for (row, (i, item)) in self
            .items
            .iter()
            .enumerate()
            .skip(top)
            .take(visible)
            .enumerate()
        {
            let y = top_y + (row as u32 * ROW_HEIGHT) as i32;
            let style = if i == self.selected {
                Rectangle::new(
                    Point::new(0, y),
                    Size::new(crate::hardware::DISPLAY_WIDTH, ROW_HEIGHT),
                )
                .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                .draw(canvas)
                .unwrap();
                super::TEXT_INVERTED
            } else {
                super::TEXT
            };
            Text::with_text_style(item.label, Point::new(2, y), style, super::TOP_LEFT)
                .draw(canvas)
                .unwrap();
        }

        // a hint that the list carries on below
        if top + visible < N {
            Text::with_text_style(
                "▼",
                Point::new(
                    crate::hardware::DISPLAY_WIDTH as i32 - 1,
                    crate::hardware::DISPLAY_HEIGHT as i32 - 6,
                ),
                super::SMALL_TEXT,
                TextStyleBuilder::new()
                    .alignment(Alignment::Right)
                    .baseline(Baseline::Top)
                    .build(),
            )
            .draw(canvas)
            .unwrap();
        }
    }

    fn handle(&mut self, button: Button) -> Response {
        if self.open {
            return match self.items[self.selected].screen.handle(button) {
                Response::Exit => {
                    self.open = false;
                    Response::Redraw
                }
                r => r,
            };
        }

        match button {
            Button::Up => self.selected = self.selected.saturating_sub(1),
            Button::Down => self.selected = (self.selected + 1).min(N - 1),
            Button::Center | Button::Right => {
                self.open = true;
                self.items[self.selected].screen.enter();
            }
            Button::Left => return Response::Exit,
        }
        Response::Redraw
    }

    fn tick_interval(&self) -> Option<embassy_time::Duration> {
        self.open
            .then(|| self.items[self.selected].screen.tick_interval())
            .flatten()
    }

    fn tick(&mut self) -> Response {
        match self.open {
            true => self.items[self.selected].screen.tick(),
            false => Response::Ignored,
        }
    }
}
