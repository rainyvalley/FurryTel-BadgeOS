use core::fmt::Write as _;

use embassy_time::Duration;
use embedded_graphics::{
    Drawable,
    geometry::{Point, Size},
    mono_font::{MonoFont, MonoTextStyle},
    pixelcolor::BinaryColor,
    primitives::{Primitive, PrimitiveStyle, Rectangle},
    text::{Alignment, Baseline, Text, TextStyle, TextStyleBuilder},
};

use crate::{
    backlight,
    game::flappy::{self, FlappyGame, State},
    hardware,
    ui::{self, Button, Canvas, Response, Screen},
};

const BIG_FONT: &MonoFont = &embedded_vintage_fonts::FONT_12X16;
const CENTER_TOP: TextStyle = TextStyleBuilder::new()
    .alignment(Alignment::Center)
    .baseline(Baseline::Top)
    .build();
/// The red the backlight turns when the bird goes down.
const DEATH_COLOR: [u8; 3] = [120, 0, 0];
const TICK: Duration = Duration::from_millis(50);

/// Flappy bird: Center flaps, and the pipes do the rest.
pub struct Flappy {
    game: FlappyGame,
    /// Lefts needed to leave; any other button resets it.
    exit_armed: u8,
}

impl Flappy {
    pub fn new(seed: u64) -> Self {
        Self {
            game: FlappyGame::new(seed),
            exit_armed: 0,
        }
    }

    fn leave(&self) {
        backlight::set(backlight::DEFAULT);
    }

    fn exit_response(&mut self) -> Response {
        self.exit_armed += 1;
        if self.exit_armed >= 3 {
            self.leave();
            Response::Exit
        } else {
            Response::Redraw
        }
    }

    /// The normal hint, or how many Lefts are still needed to leave.
    fn exit_hint(&self, normal: &'static str) -> &'static str {
        match self.exit_armed {
            0 => normal,
            1 => "◀◀◀ to quit",
            _ => "◀◀ to quit",
        }
    }

    fn draw_field(&self, canvas: &mut Canvas) {
        let ink = PrimitiveStyle::with_fill(BinaryColor::On);

        for pipe in self.game.pipes() {
            let x = pipe.x;
            if x + flappy::PIPE_W <= 0 {
                continue;
            }
            let bottom = pipe.gap_top + flappy::GAP;
            Rectangle::new(
                Point::new(x, 0),
                Size::new(flappy::PIPE_W as u32, pipe.gap_top as u32),
            )
            .into_styled(ink)
            .draw(canvas)
            .unwrap();
            Rectangle::new(
                Point::new(x, bottom),
                Size::new(flappy::PIPE_W as u32, (flappy::HEIGHT - bottom) as u32),
            )
            .into_styled(ink)
            .draw(canvas)
            .unwrap();
            // the wider lip at the mouth of each pipe
            for y in [pipe.gap_top - 2, pipe.gap_top + flappy::GAP] {
                Rectangle::new(
                    Point::new(x - 1, y),
                    Size::new(flappy::PIPE_W as u32 + 2, 2),
                )
                .into_styled(ink)
                .draw(canvas)
                .unwrap();
            }
        }

        let y = self.game.bird_y();
        Rectangle::new(
            Point::new(flappy::BIRD_X, y),
            Size::new(flappy::BIRD_W as u32 - 1, flappy::BIRD_H as u32 - 1),
        )
        .into_styled(ink)
        .draw(canvas)
        .unwrap();
        Rectangle::new(Point::new(flappy::BIRD_X + 7, y + 2), Size::new(2, 2))
            .into_styled(ink)
            .draw(canvas)
            .unwrap();
        Rectangle::new(Point::new(flappy::BIRD_X + 5, y + 1), Size::new(1, 1))
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::Off))
            .draw(canvas)
            .unwrap();
    }
}

fn big_text(canvas: &mut Canvas, text: &str, y: i32) {
    Text::with_text_style(
        text,
        Point::new(hardware::DISPLAY_WIDTH as i32 / 2, y),
        MonoTextStyle::new(BIG_FONT, BinaryColor::On),
        CENTER_TOP,
    )
    .draw(canvas)
    .unwrap();
}

fn small_text(canvas: &mut Canvas, text: &str, y: i32) {
    Text::with_text_style(
        text,
        Point::new(hardware::DISPLAY_WIDTH as i32 / 2, y),
        ui::SMALL_TEXT,
        CENTER_TOP,
    )
    .draw(canvas)
    .unwrap();
}

fn banner(canvas: &mut Canvas, y: i32, height: u32) {
    Rectangle::new(Point::new(0, y), Size::new(hardware::DISPLAY_WIDTH, height))
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::Off))
        .draw(canvas)
        .unwrap();
}

impl Screen for Flappy {
    fn enter(&mut self) {
        self.game.reset();
        self.exit_armed = 0;
        backlight::set(backlight::DEFAULT);
    }

    fn draw(&self, canvas: &mut Canvas) {
        match self.game.state() {
            State::Ready => {
                big_text(canvas, "FLAPPY", 0);
                small_text(canvas, self.exit_hint("● fly    ◀ exit"), 19);
            }
            State::Playing => {
                self.draw_field(canvas);
                if self.exit_armed > 0 {
                    banner(canvas, 0, 8);
                    small_text(canvas, self.exit_hint(""), 1);
                }
            }
            State::Dead => {
                self.draw_field(canvas);
                banner(canvas, 9, 23);
                let mut line = heapless::String::<20>::new();
                write!(&mut line, "GAME OVER {}", self.game.score()).unwrap();
                Text::with_text_style(
                    line.as_str(),
                    Point::new(hardware::DISPLAY_WIDTH as i32 / 2, 10),
                    ui::TEXT,
                    CENTER_TOP,
                )
                .draw(canvas)
                .unwrap();
                small_text(canvas, self.exit_hint("● retry   ◀ exit"), 22);
            }
        }
    }

    fn handle(&mut self, button: Button) -> Response {
        match self.game.state() {
            State::Ready => match button {
                Button::Center => {
                    self.exit_armed = 0;
                    self.game.start();
                    Response::Redraw
                }
                Button::Left => self.exit_response(),
                _ => {
                    self.exit_armed = 0;
                    Response::Ignored
                }
            },
            State::Playing => match button {
                Button::Center => {
                    self.exit_armed = 0;
                    self.game.flap();
                    Response::Ignored
                }
                // three lefts still leave, even mid-flight
                Button::Left => self.exit_response(),
                _ => {
                    self.exit_armed = 0;
                    Response::Ignored
                }
            },
            State::Dead => match button {
                Button::Center => {
                    self.exit_armed = 0;
                    self.game.reset();
                    self.leave();
                    Response::Redraw
                }
                Button::Left => self.exit_response(),
                _ => {
                    self.exit_armed = 0;
                    Response::Ignored
                }
            },
        }
    }

    fn tick_interval(&self) -> Option<Duration> {
        (self.game.state() == State::Playing).then_some(TICK)
    }

    fn tick(&mut self) -> Response {
        if !self.game.tick() {
            return Response::Ignored;
        }
        if self.game.state() == State::Dead {
            backlight::set(backlight::Mode::Solid(DEATH_COLOR));
        }
        Response::Redraw
    }
}
