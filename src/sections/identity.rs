use embedded_graphics::{
    Drawable,
    geometry::{Point, Size},
    mono_font::MonoTextStyle,
    pixelcolor::BinaryColor,
    primitives::{Circle, Ellipse, Primitive, PrimitiveStyle, Rectangle},
    text::Text,
};

use crate::{hardware, ui};

use super::{Section, SectionTarget};

/// Who the badge belongs to: the name in the biggest font that fits, with a paw print.
pub struct Identity;

impl Identity {
    const NAME: &str = crate::generated::NAME;
    /// The name uses the biggest font that fits, so longer names still work.
    const BIG_FONT: &'static embedded_graphics::mono_font::MonoFont<'static> =
        &embedded_vintage_fonts::FONT_12X16;
    const MID_FONT: &'static embedded_graphics::mono_font::MonoFont<'static> =
        &embedded_vintage_fonts::FONT_8X16;
    /// Height of this section.
    const HEIGHT: u32 = 16;
    /// Left edge of the paw, just right of the name.
    const PAW_X: i32 = 103;

    pub const fn new() -> Self {
        Self
    }

    fn font() -> &'static embedded_graphics::mono_font::MonoFont<'static> {
        match Self::NAME.len() {
            0..=8 => Self::BIG_FONT,
            9..=12 => Self::MID_FONT,
            _ => ui::FONT,
        }
    }
}

impl Section for Identity {
    fn area(&self) -> Rectangle {
        Rectangle::new(
            Point::zero(),
            Size::new(hardware::DISPLAY_WIDTH, Self::HEIGHT),
        )
    }

    fn draw(&self, target: &mut SectionTarget<'_>) {
        let font = Self::font();
        let y = Self::HEIGHT.saturating_sub(font.character_size.height) / 2;
        Text::with_text_style(
            Self::NAME,
            Point::new(2, y as i32),
            MonoTextStyle::new(font, BinaryColor::On),
            ui::TOP_LEFT,
        )
        .draw(target)
        .unwrap();

        let ink = PrimitiveStyle::with_fill(BinaryColor::On);
        Ellipse::new(Point::new(Self::PAW_X, 7), Size::new(16, 8))
            .into_styled(ink)
            .draw(target)
            .unwrap();
        for (x, y) in [(1, 2), (8, 0), (15, 2)] {
            Circle::new(Point::new(Self::PAW_X + x, y), 5)
                .into_styled(ink)
                .draw(target)
                .unwrap();
        }
    }
}
