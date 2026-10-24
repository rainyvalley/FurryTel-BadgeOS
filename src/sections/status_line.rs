use embedded_graphics::{
    Drawable,
    geometry::{Point, Size},
    primitives::Rectangle,
    text::{Alignment, Baseline, Text, TextStyleBuilder},
};

use crate::{hardware, ui};

use super::{Section, SectionTarget};

/// The little hint in the bottom-right corner.
pub struct StatusLine;

impl StatusLine {
    pub const fn new() -> Self {
        Self
    }
}

impl Section for StatusLine {
    fn area(&self) -> Rectangle {
        let height = ui::SMALL_FONT.character_size.height;
        Rectangle::new(
            Point::new(0, (hardware::DISPLAY_HEIGHT - height) as i32),
            Size::new(hardware::DISPLAY_WIDTH, height),
        )
    }

    fn draw(&self, target: &mut SectionTarget<'_>) {
        Text::with_text_style(
            "● menu",
            Point::new(hardware::DISPLAY_WIDTH as i32 - 1, 0),
            ui::SMALL_TEXT,
            TextStyleBuilder::new()
                .alignment(Alignment::Right)
                .baseline(Baseline::Top)
                .build(),
        )
        .draw(target)
        .unwrap();
    }
}
