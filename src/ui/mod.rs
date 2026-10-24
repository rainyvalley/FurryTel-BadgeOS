//! A tiny screen system: each [`Screen`] knows how to draw itself and how to react to a button
//! press.  Screens compose (see [`menu::Menu`]), so the whole UI is a single root screen that
//! [`run`] drives.  Screens that animate ask for periodic [`Screen::tick`]s.

pub mod glyphs;
pub mod konami;
pub mod menu;
pub mod text_input;

use core::fmt::Write as _;

use embassy_futures::select::{Either, select};
use embassy_time::{Duration, Timer};
use embedded_graphics::{
    Drawable,
    draw_target::DrawTarget as _,
    geometry::{Point, Size},
    mono_font::{self, MonoTextStyle, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    primitives::{Primitive, PrimitiveStyle, Rectangle},
    text::{self, Text, TextStyle, TextStyleBuilder},
};

use crate::backlight;
use crate::hardware::Buttons;
use glyphs::GlyphTextStyle;

/// What screens draw on.  This is the concrete display type rather than a generic
/// `embedded_graphics::DrawTarget`, which keeps [`Screen`] usable as a trait object.
pub type Canvas = crate::hardware::DrawTarget;

#[derive(Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum Button {
    Up,
    Down,
    Left,
    Right,
    Center,
}

#[derive(Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum Response {
    /// The event changed something; the screen needs to be redrawn.
    Redraw,
    /// The event didn't do anything.
    Ignored,
    /// The screen is done; whoever is showing it should take back over.
    Exit,
}

pub trait Screen {
    /// Called each time the screen is opened, before it's first drawn.  Not called on redraws.
    fn enter(&mut self) {}

    /// Draw the whole screen.  The canvas has already been cleared.
    fn draw(&self, canvas: &mut Canvas);

    fn handle(&mut self, button: Button) -> Response;

    /// How often the screen wants [`Screen::tick`] called, or `None` (the default) for never.
    /// Checked after every event, so a screen can start and stop animating as it likes.
    fn tick_interval(&self) -> Option<Duration> {
        None
    }

    /// Called every [`Screen::tick_interval`] while that's `Some`.
    fn tick(&mut self) -> Response {
        Response::Ignored
    }

    /// Whether the secret code is listened for right now.  Only the home screen wants it, so
    /// games and menus keep every button press for themselves.
    fn secret_ok(&self) -> bool {
        false
    }
}

/// Drive `root` forever: wait for a button (or a tick, if the screen wants them), hand it to the
/// screen, redraw if asked.
pub async fn run(canvas: &mut Canvas, buttons: &mut Buttons, root: &mut dyn Screen) -> ! {
    canvas.set_display_on(true).unwrap();
    root.enter();
    redraw(canvas, root);

    loop {
        let press = buttons.next_press();
        let event = match root.tick_interval() {
            Some(interval) => select(press, Timer::after(interval)).await,
            None => Either::First(press.await),
        };
        let response = match event {
            Either::First(button) => {
                defmt::debug!("button {}", button);
                backlight::flash();
                crate::discrete::flash();
                root.handle(button)
            }
            Either::Second(()) => root.tick(),
        };
        match response {
            // the root has nowhere to exit to, so just show it again
            Response::Redraw | Response::Exit => redraw(canvas, root),
            Response::Ignored => {}
        }
    }
}

fn redraw(canvas: &mut Canvas, screen: &dyn Screen) {
    canvas.clear(BinaryColor::Off).unwrap();
    screen.draw(canvas);
    canvas.flush().unwrap();
}

pub const FONT: &mono_font::MonoFont = &mono_font::ascii::FONT_6X10;

// All the text styles can draw the button glyphs in `glyphs::GLYPHS`, e.g. "● to edit".

/// Light text on a dark background.
pub const TEXT: GlyphTextStyle =
    GlyphTextStyle::new(MonoTextStyle::new(FONT, BinaryColor::On), &glyphs::FONT);
/// Dark text with its own light background, for highlighting a bit of text.
pub const TEXT_HIGHLIGHTED: GlyphTextStyle = GlyphTextStyle::new(
    MonoTextStyleBuilder::new()
        .font(FONT)
        .text_color(BinaryColor::Off)
        .background_color(BinaryColor::On)
        .build(),
    &glyphs::FONT,
);
/// Dark text, for drawing on top of a highlight.
pub const TEXT_INVERTED: GlyphTextStyle =
    GlyphTextStyle::new(MonoTextStyle::new(FONT, BinaryColor::Off), &glyphs::FONT);
/// For when [`FONT`] is too big, e.g. the on-screen keyboard.
/// ISO 8859-1 rather than ASCII for a few extra symbols, like the keyboard's « and ».
pub const SMALL_FONT: &mono_font::MonoFont = &mono_font::iso_8859_1::FONT_4X6;
pub const SMALL_TEXT: GlyphTextStyle = GlyphTextStyle::new(
    MonoTextStyle::new(SMALL_FONT, BinaryColor::On),
    &glyphs::SMALL_FONT,
);
pub const SMALL_TEXT_INVERTED: GlyphTextStyle = GlyphTextStyle::new(
    MonoTextStyle::new(SMALL_FONT, BinaryColor::Off),
    &glyphs::SMALL_FONT,
);

/// Position text by its top-left corner.
pub const TOP_LEFT: TextStyle = TextStyleBuilder::new()
    .alignment(text::Alignment::Left)
    .baseline(text::Baseline::Top)
    .build();

/// A small label, inverted when it's the selected field.
pub fn label(canvas: &mut Canvas, x: i32, text: &str, y: i32, selected: bool) {
    let size = SMALL_FONT.character_size;
    if selected {
        Rectangle::new(
            Point::new(x, y),
            Size::new(text.len() as u32 * size.width + 2, size.height),
        )
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
        .draw(canvas)
        .unwrap();
        Text::with_text_style(text, Point::new(x + 1, y), SMALL_TEXT_INVERTED, TOP_LEFT)
            .draw(canvas)
            .unwrap();
    } else {
        Text::with_text_style(text, Point::new(x + 1, y), SMALL_TEXT, TOP_LEFT)
            .draw(canvas)
            .unwrap();
    }
}

/// A labelled 0-255 slider with a bar and its value.
pub fn slider(canvas: &mut Canvas, text: &str, value: u8, y: i32, selected: bool) {
    label(canvas, 0, text, y, selected);

    Rectangle::new(Point::new(10, y + 1), Size::new(80, 4))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
        .draw(canvas)
        .unwrap();

    let filled = u32::from(value) * 76 / 255;
    if filled > 0 {
        Rectangle::new(Point::new(12, y + 2), Size::new(filled, 2))
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
            .draw(canvas)
            .unwrap();
    }

    let mut number = heapless::String::<4>::new();
    write!(&mut number, "{value:3}").unwrap();
    Text::with_text_style(number.as_str(), Point::new(94, y), SMALL_TEXT, TOP_LEFT)
        .draw(canvas)
        .unwrap();
}
