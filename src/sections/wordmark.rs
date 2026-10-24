use embedded_graphics::{
    Drawable,
    geometry::{Point, Size},
    primitives::Rectangle,
    text::Text,
};

use crate::ui;

use super::{Section, SectionTarget};

/// The edition name, sitting under the owner's name.
pub struct Wordmark;

impl Wordmark {
    pub const NAME: &str = crate::generated::SPECIES;

    pub const fn new() -> Self {
        Self
    }
}

impl Section for Wordmark {
    fn area(&self) -> Rectangle {
        Rectangle::new(
            Point::new(0, 16),
            Size::new(64, ui::FONT.character_size.height),
        )
    }

    fn draw(&self, target: &mut SectionTarget<'_>) {
        Text::with_text_style(Self::NAME, Point::zero(), ui::TEXT, ui::TOP_LEFT)
            .draw(target)
            .unwrap();
    }
}
