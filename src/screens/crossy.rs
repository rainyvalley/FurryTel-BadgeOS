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
    game::crossy::{self, CrossyGame, Dir, Lane, State},
    hardware,
    ui::{self, Button, Canvas, Response, Screen},
};

const CELL: i32 = 4;
const BIG_FONT: &MonoFont = &embedded_vintage_fonts::FONT_12X16;
const CENTER_TOP: TextStyle = TextStyleBuilder::new()
    .alignment(Alignment::Center)
    .baseline(Baseline::Top)
    .build();
/// The red the backlight turns when the chicken gets hit.
const DEATH_COLOR: [u8; 3] = [120, 0, 0];
const TICK: Duration = Duration::from_millis(60);

/// Crossy road: hop up the lanes, and look before you leap.
pub struct Crossy {
    game: CrossyGame,
    /// Lefts needed to leave; any other button resets it.
    exit_armed: u8,
}

impl Crossy {
    pub fn new(seed: u64) -> Self {
        Self {
            game: CrossyGame::new(seed),
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

        for (row, lane) in self.game.lanes().iter().enumerate() {
            let y = row as i32 * CELL;
            match lane {
                Lane::Grass => {
                    for x in (row as u8 * 4 % 8..crossy::COLS).step_by(8) {
                        Rectangle::new(Point::new(x as i32 * CELL, y + CELL - 1), Size::new(1, 1))
                            .into_styled(ink)
                            .draw(canvas)
                            .unwrap();
                    }
                }
                Lane::Road { .. } => {
                    for k in 0..crossy::CARS {
                        let start = crossy::car_start(*lane, k);
                        let x = start as i32 * CELL;
                        let width = (crossy::CAR_LEN as i32 * CELL) as u32;
                        for dx in [0, -(hardware::DISPLAY_WIDTH as i32)] {
                            if x + dx + width as i32 > 0 {
                                Rectangle::new(Point::new(x + dx, y + 1), Size::new(width, 3))
                                    .into_styled(ink)
                                    .draw(canvas)
                                    .unwrap();
                            }
                        }
                    }
                }
            }
        }

        let x = self.game.col as i32 * CELL;
        let y = self.game.row as i32 * CELL;
        Rectangle::new(Point::new(x, y), Size::new(3, 3))
            .into_styled(ink)
            .draw(canvas)
            .unwrap();
        Rectangle::new(Point::new(x + 3, y + 1), Size::new(1, 1))
            .into_styled(ink)
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

impl Screen for Crossy {
    fn enter(&mut self) {
        self.game.reset();
        self.exit_armed = 0;
        backlight::set(backlight::DEFAULT);
    }

    fn draw(&self, canvas: &mut Canvas) {
        match self.game.state() {
            State::Ready => {
                big_text(canvas, "CROSSY", 0);
                small_text(canvas, self.exit_hint("● start   ◀ exit"), 19);
            }
            State::Playing => {
                self.draw_field(canvas);
                if self.game.banner() {
                    banner(canvas, 11, 11);
                    let mut line = heapless::String::<12>::new();
                    write!(&mut line, "LEVEL {}", self.game.level()).unwrap();
                    Text::with_text_style(
                        line.as_str(),
                        Point::new(hardware::DISPLAY_WIDTH as i32 / 2, 12),
                        ui::TEXT,
                        CENTER_TOP,
                    )
                    .draw(canvas)
                    .unwrap();
                }
            }
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
                    let dir = match button {
                        Button::Up => Dir::Up,
                        Button::Down => Dir::Down,
                        Button::Left => Dir::Left,
                        _ => Dir::Right,
                    };
                    let moved = self.game.hop(dir);
                    if self.game.state() == State::Dead {
                        backlight::set(backlight::Mode::Solid(DEATH_COLOR));
                    }
                    if moved {
                        Response::Redraw
                    } else {
                        Response::Ignored
                    }
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
