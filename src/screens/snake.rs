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
    game::snake::{Dir, SnakeGame, State, Step},
    hardware,
    ui::{self, Button, Canvas, Response, Screen},
};

/// Cells are 4x4 pixels, which tiles the 128x32 screen exactly.
const CELL: u32 = 4;
const BIG_FONT: &MonoFont = &embedded_vintage_fonts::FONT_12X16;
const CENTER_TOP: TextStyle = TextStyleBuilder::new()
    .alignment(Alignment::Center)
    .baseline(Baseline::Top)
    .build();
/// The red the backlight turns when the snake dies.
const DEATH_COLOR: [u8; 3] = [120, 0, 0];

/// The secret game, unlocked by the Konami code.  Directions steer, Center pauses, and Left leaves
/// the game when it isn't running.
pub struct Snake {
    game: SnakeGame,
    /// Lefts needed to leave; any other button resets it.
    exit_armed: u8,
}

impl Snake {
    pub fn new(seed: u64) -> Self {
        Self {
            game: SnakeGame::new(seed),
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

        Rectangle::new(
            cell_origin(self.game.food()) + Point::new(1, 1),
            Size::new(2, 2),
        )
        .into_styled(ink)
        .draw(canvas)
        .unwrap();

        for (i, cell) in self.game.segments().enumerate() {
            let size = if i == 0 { CELL } else { CELL - 1 };
            Rectangle::new(cell_origin(cell), Size::new(size, size))
                .into_styled(ink)
                .draw(canvas)
                .unwrap();
        }
    }
}

fn dir_of(button: Button) -> Option<Dir> {
    Some(match button {
        Button::Up => Dir::Up,
        Button::Down => Dir::Down,
        Button::Left => Dir::Left,
        Button::Right => Dir::Right,
        Button::Center => return None,
    })
}

/// Top-left pixel of `cell`.
fn cell_origin((x, y): (u8, u8)) -> Point {
    Point::new(x as i32 * CELL as i32, y as i32 * CELL as i32)
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

/// A light rectangle to put text on when the playfield would make it hard to read.
fn banner(canvas: &mut Canvas, y: i32, height: u32) {
    Rectangle::new(Point::new(0, y), Size::new(hardware::DISPLAY_WIDTH, height))
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::Off))
        .draw(canvas)
        .unwrap();
}

impl Screen for Snake {
    fn enter(&mut self) {
        self.game.reset();
        self.exit_armed = 0;
        backlight::set(backlight::DEFAULT);
    }

    fn draw(&self, canvas: &mut Canvas) {
        match self.game.state() {
            State::Ready => {
                big_text(canvas, "SNAKE", 0);
                small_text(canvas, self.exit_hint("● start   ◀ exit"), 19);
            }
            State::Playing => self.draw_field(canvas),
            State::Paused => {
                self.draw_field(canvas);
                banner(canvas, 10, 22);
                Text::with_text_style(
                    "PAUSED",
                    Point::new(hardware::DISPLAY_WIDTH as i32 / 2, 11),
                    ui::TEXT,
                    CENTER_TOP,
                )
                .draw(canvas)
                .unwrap();
                small_text(canvas, self.exit_hint("● resume  ◀ quit"), 22);
            }
            State::Dead => {
                self.draw_field(canvas);
                banner(canvas, 7, 25);
                let mut line = heapless::String::<20>::new();
                write!(&mut line, "GAME OVER {}", self.game.score()).unwrap();
                Text::with_text_style(
                    line.as_str(),
                    Point::new(hardware::DISPLAY_WIDTH as i32 / 2, 8),
                    ui::TEXT,
                    CENTER_TOP,
                )
                .draw(canvas)
                .unwrap();
                small_text(canvas, self.exit_hint("● retry   ◀ exit"), 20);
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
                    self.game.pause();
                    Response::Redraw
                }
                _ => {
                    self.exit_armed = 0;
                    if let Some(dir) = dir_of(button) {
                        self.game.turn(dir);
                    }
                    Response::Ignored
                }
            },
            State::Paused => match button {
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
        (self.game.state() == State::Playing).then(|| Duration::from_millis(self.game.delay_ms()))
    }

    fn tick(&mut self) -> Response {
        match self.game.step() {
            Some(Step::Died) => {
                backlight::set(backlight::Mode::Solid(DEATH_COLOR));
                Response::Redraw
            }
            Some(_) => Response::Redraw,
            None => Response::Ignored,
        }
    }
}
