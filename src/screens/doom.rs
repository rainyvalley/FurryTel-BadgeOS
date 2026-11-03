use core::fmt::Write as _;

use embassy_time::Duration;
use embedded_graphics::{
    Drawable, Pixel,
    draw_target::DrawTarget,
    geometry::{Point, Size},
    mono_font::MonoTextStyle,
    pixelcolor::BinaryColor,
    primitives::{Primitive, PrimitiveStyle, Rectangle},
    text::{Alignment, Baseline, Text, TextStyle, TextStyleBuilder},
};

use crate::{
    backlight,
    game::doom::{self, DoomGame, Frame, State},
    hardware,
    ui::{self, Button, Canvas, Response, Screen},
};

const CENTER_TOP: TextStyle = TextStyleBuilder::new()
    .alignment(Alignment::Center)
    .baseline(Baseline::Top)
    .build();
/// The red the backlight turns when the player dies.
const DEATH_COLOR: [u8; 3] = [120, 0, 0];
/// Pressing these in turn pauses the game.
const PAUSE_COMBO: [Button; 4] = [Button::Up, Button::Down, Button::Up, Button::Down];

/// A tiny first-person shooter.  Up and Down walk, Left and Right turn, Center fires at whatever is
/// straight ahead.  Up, Down, Up, Down pauses, and three presses of Left leave from the ready,
/// paused, or game-over screens.
pub struct Doom {
    game: DoomGame,
    exit_armed: u8,
    combo: u8,
}

impl Doom {
    pub fn new(seed: u64) -> Self {
        Self {
            game: DoomGame::new(seed),
            exit_armed: 0,
            combo: 0,
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

    fn exit_hint(&self, normal: &'static str) -> &'static str {
        match self.exit_armed {
            0 => normal,
            1 => "◀◀◀ to quit",
            _ => "◀◀ to quit",
        }
    }

    fn draw_view(&self, canvas: &mut Canvas) {
        let mut frame = Frame::new();
        self.game.render(&mut frame);
        let lit = (0..doom::HEIGHT).flat_map(|y| (0..doom::WIDTH).map(move |x| (x, y)));
        canvas
            .draw_iter(
                lit.filter(|&(x, y)| frame.get(x, y))
                    .map(|(x, y)| Pixel(Point::new(x as i32, y as i32), BinaryColor::On)),
            )
            .unwrap();
    }

    fn draw_hud(&self, canvas: &mut Canvas) {
        let mut health = heapless::String::<16>::new();
        write!(&mut health, "{}", self.game.health()).unwrap();
        self.banner_at(canvas, 0, 25, 14, 7);
        Text::with_baseline(
            health.as_str(),
            Point::new(1, 26),
            ui::SMALL_TEXT,
            Baseline::Top,
        )
        .draw(canvas)
        .unwrap();

        let mut kills = heapless::String::<16>::new();
        write!(
            &mut kills,
            "L{} {}/{}",
            self.game.level(),
            self.game.kills(),
            self.game.total()
        )
        .unwrap();
        let width = kills.len() as u32 * 4 + 1;
        self.banner_at(
            canvas,
            hardware::DISPLAY_WIDTH as i32 - width as i32,
            0,
            width,
            7,
        );
        Text::with_text_style(
            kills.as_str(),
            Point::new(hardware::DISPLAY_WIDTH as i32 - 1, 1),
            ui::SMALL_TEXT,
            TextStyleBuilder::new()
                .alignment(Alignment::Right)
                .baseline(Baseline::Top)
                .build(),
        )
        .draw(canvas)
        .unwrap();
    }

    fn draw_ammo_hint(&self, canvas: &mut Canvas) {
        let message = if self.game.reloading() {
            "RELOAD"
        } else if self.game.ammo() == 0 {
            "● RELOAD"
        } else {
            return;
        };
        self.banner_at(canvas, 40, 0, 48, 7);
        self.small_text(canvas, message, 1);
    }

    fn banner_at(&self, canvas: &mut Canvas, x: i32, y: i32, width: u32, height: u32) {
        Rectangle::new(Point::new(x, y), Size::new(width, height))
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::Off))
            .draw(canvas)
            .unwrap();
    }

    fn banner(&self, canvas: &mut Canvas, y: i32, height: u32) {
        Rectangle::new(Point::new(0, y), Size::new(hardware::DISPLAY_WIDTH, height))
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::Off))
            .draw(canvas)
            .unwrap();
    }

    fn text(&self, canvas: &mut Canvas, message: &str, y: i32) {
        Text::with_text_style(
            message,
            Point::new(hardware::DISPLAY_WIDTH as i32 / 2, y),
            ui::TEXT,
            CENTER_TOP,
        )
        .draw(canvas)
        .unwrap();
    }

    fn small_text(&self, canvas: &mut Canvas, message: &str, y: i32) {
        Text::with_text_style(
            message,
            Point::new(hardware::DISPLAY_WIDTH as i32 / 2, y),
            ui::SMALL_TEXT,
            CENTER_TOP,
        )
        .draw(canvas)
        .unwrap();
    }

    fn big_text_at(&self, canvas: &mut Canvas, message: &str, x: i32, y: i32) {
        Text::with_text_style(
            message,
            Point::new(x, y),
            MonoTextStyle::new(&embedded_vintage_fonts::FONT_12X16, BinaryColor::On),
            TextStyleBuilder::new().baseline(Baseline::Top).build(),
        )
        .draw(canvas)
        .unwrap();
    }

    #[allow(dead_code)]
    fn big_text(&self, canvas: &mut Canvas, message: &str, y: i32) {
        Text::with_text_style(
            message,
            Point::new(hardware::DISPLAY_WIDTH as i32 / 2, y),
            MonoTextStyle::new(&embedded_vintage_fonts::FONT_12X16, BinaryColor::On),
            CENTER_TOP,
        )
        .draw(canvas)
        .unwrap();
    }
}

impl Screen for Doom {
    fn final_score(&self) -> Option<u32> {
        matches!(self.game.state(), State::Dead | State::Retired).then(|| self.game.score())
    }

    fn enter(&mut self) {
        self.game.reset();
        self.exit_armed = 0;
        self.combo = 0;
        backlight::set(backlight::DEFAULT);
    }

    fn draw(&self, canvas: &mut Canvas) {
        match self.game.state() {
            State::Ready => {
                let mut level = heapless::String::<32>::new();
                write!(&mut level, "▲▼ level {}", self.game.difficulty().name()).unwrap();
                self.big_text_at(canvas, "DOOM", 2, 8);
                for (n, line) in [
                    "▲▼ walk  ◀▶ turn",
                    "● fire / reload",
                    "▲▼▲▼ pause",
                    level.as_str(),
                    self.exit_hint("● start ◀◀◀ exit"),
                ]
                .into_iter()
                .enumerate()
                {
                    Text::with_text_style(
                        line,
                        Point::new(54, n as i32 * 6 + 1),
                        ui::SMALL_TEXT,
                        TextStyleBuilder::new().baseline(Baseline::Top).build(),
                    )
                    .draw(canvas)
                    .unwrap();
                }
            }
            State::Playing => {
                self.draw_view(canvas);
                self.draw_hud(canvas);
                self.draw_ammo_hint(canvas);
            }
            State::Paused => {
                self.draw_view(canvas);
                self.banner(canvas, 7, 25);
                self.text(canvas, "PAUSED", 8);
                self.small_text(canvas, self.exit_hint("● resume ◀◀◀ quit"), 20);
            }
            State::Dead | State::Retired => {
                self.draw_view(canvas);
                self.banner(canvas, 7, 25);
                self.text(
                    canvas,
                    if self.game.state() == State::Dead {
                        "YOU DIED"
                    } else {
                        "RETIRED"
                    },
                    8,
                );
                let mut line = heapless::String::<32>::new();
                write!(
                    &mut line,
                    "LEVEL {}  SCORE {}",
                    self.game.level(),
                    self.game.score()
                )
                .unwrap();
                self.small_text(canvas, &line, 19);
                self.small_text(canvas, self.exit_hint("● retry ◀◀◀ exit"), 26);
            }
            State::Cleared => {
                self.draw_view(canvas);
                self.banner(canvas, 7, 25);
                let mut line = heapless::String::<32>::new();
                write!(&mut line, "LEVEL {} CLEAR", self.game.level()).unwrap();
                self.text(canvas, &line, 8);
                let mut line = heapless::String::<32>::new();
                write!(&mut line, "SCORE {}", self.game.score()).unwrap();
                self.small_text(canvas, &line, 19);
                self.small_text(canvas, self.exit_hint("● next level ◀◀◀ stop"), 26);
            }
        }
    }

    fn handle(&mut self, button: Button) -> Response {
        match self.game.state() {
            State::Playing => {
                self.exit_armed = 0;
                if button == PAUSE_COMBO[self.combo as usize] {
                    self.combo += 1;
                } else {
                    self.combo = u8::from(button == PAUSE_COMBO[0]);
                }
                if usize::from(self.combo) == PAUSE_COMBO.len() {
                    self.combo = 0;
                    self.game.pause();
                    return Response::Redraw;
                }
                match button {
                    Button::Up => {
                        self.game.walk(true);
                    }
                    Button::Down => {
                        self.game.walk(false);
                    }
                    Button::Left => self.game.turn(false),
                    Button::Right => self.game.turn(true),
                    Button::Center => {
                        self.game.fire();
                    }
                }
                Response::Redraw
            }
            State::Ready => match button {
                Button::Up => {
                    self.exit_armed = 0;
                    self.game.set_difficulty(self.game.difficulty().harder());
                    Response::Redraw
                }
                Button::Down => {
                    self.exit_armed = 0;
                    self.game.set_difficulty(self.game.difficulty().easier());
                    Response::Redraw
                }
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
            State::Cleared => match button {
                Button::Center => {
                    self.exit_armed = 0;
                    self.game.next_level();
                    Response::Redraw
                }
                // three Lefts stop here, keeping the score
                Button::Left => {
                    self.exit_armed += 1;
                    if self.exit_armed >= 3 {
                        self.exit_armed = 0;
                        self.game.retire();
                    }
                    Response::Redraw
                }
                _ => {
                    self.exit_armed = 0;
                    Response::Ignored
                }
            },
            State::Dead | State::Retired => match button {
                Button::Center => {
                    self.exit_armed = 0;
                    self.game.reset();
                    self.game.start();
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

    fn fixed_ticks(&self) -> bool {
        true
    }

    fn tick_interval(&self) -> Option<Duration> {
        (self.game.state() == State::Playing).then(|| Duration::from_millis(300))
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
