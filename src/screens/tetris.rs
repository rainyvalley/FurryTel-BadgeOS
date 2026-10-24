use core::fmt::Write as _;

use embassy_time::Duration;
use embedded_graphics::{
    Drawable,
    geometry::{Point, Size},
    pixelcolor::BinaryColor,
    primitives::{Primitive, PrimitiveStyle, Rectangle},
};

use crate::game::tetris::{self, State, TetrisGame};
use crate::{
    backlight,
    ui::{Button, Canvas, Response, Screen},
};

/// The game is drawn a quarter turn round, so you turn the badge clockwise to play.
/// The board is this many screen pixels per cell.
const CELL: i32 = 4;
const BOARD_HEIGHT: i32 = tetris::ROWS as i32 * CELL;
/// The strip below the board, holding the next piece and the score.
const HUD_Y: i32 = BOARD_HEIGHT;
/// Logical size: the screen turned on its side.
const LOGICAL_WIDTH: i32 = 32;
const GLYPH_WIDTH: i32 = 4;

/// The red the backlight turns when the well fills up.
const DEATH_COLOR: [u8; 3] = [120, 0, 0];

/// A 3x5 font, so text can be drawn in the rotated view, with bit `row * 3 + column`.
const FONT: [u16; 36] = [
    0x7B6F, 0x749A, 0x73E7, 0x79A7, 0x49ED, 0x79CF, 0x7BCF, 0x24A7, 0x7BEF, 0x79EF, 0x5BEA, 0x3AEB,
    0x624E, 0x3B6B, 0x72CF, 0x12CF, 0x6B4E, 0x5BED, 0x7497, 0x2B24, 0x5AED, 0x7249, 0x5BFD, 0x5B6B,
    0x7B6F, 0x13EF, 0x4F6F, 0x5AEF, 0x388E, 0x2497, 0x7B6D, 0x2B6D, 0x5FED, 0x5AAD, 0x24AD, 0x72A7,
];

/// One logical rectangle, rotated a quarter turn onto the screen so that turning the badge
/// clockwise leaves it upright, with the board's left on the player's left.
fn fill(canvas: &mut Canvas, x: i32, y: i32, width: u32, height: u32, color: BinaryColor) {
    Rectangle::new(
        Point::new(y, LOGICAL_WIDTH - x - width as i32),
        Size::new(height, width),
    )
    .into_styled(PrimitiveStyle::with_fill(color))
    .draw(canvas)
    .unwrap();
}

fn glyph(character: char) -> Option<u16> {
    match character {
        '0'..='9' => Some(FONT[character as usize - '0' as usize]),
        'A'..='Z' => Some(FONT[10 + character as usize - 'A' as usize]),
        _ => None,
    }
}

fn text(canvas: &mut Canvas, x: i32, y: i32, message: &str, scale: i32) {
    let mut cursor = x;
    for character in message.chars() {
        if let Some(bits) = glyph(character) {
            for row in 0..5 {
                for column in 0..3 {
                    if bits & (1 << (row * 3 + column)) != 0 {
                        fill(
                            canvas,
                            cursor + column * scale,
                            y + row * scale,
                            scale as u32,
                            scale as u32,
                            BinaryColor::On,
                        );
                    }
                }
            }
        }
        cursor += GLYPH_WIDTH * scale;
    }
}

/// Text centred across the rotated screen's width.
fn centred_text(canvas: &mut Canvas, y: i32, message: &str, scale: i32) {
    let width = message.chars().count() as i32 * GLYPH_WIDTH * scale - scale;
    text(canvas, (LOGICAL_WIDTH - width) / 2, y, message, scale);
}

/// Tetris: turn the badge clockwise.  The buttons turn with it, so the one pointing at the player's
/// right moves the piece right, the one pointing down drops it, and so on.  Center rotates, and
/// three presses of the button on the player's left leave from the ready, paused, or game-over
/// screens.
pub struct Tetris {
    game: TetrisGame,
    exit_armed: u8,
}

impl Tetris {
    pub fn new(seed: u64) -> Self {
        Self {
            game: TetrisGame::new(seed),
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

    fn quit_line(&self) -> &'static str {
        match self.exit_armed {
            0 => "LLL QUIT",
            1 => "LL QUIT",
            _ => "L QUIT",
        }
    }

    fn draw_board(&self, canvas: &mut Canvas) {
        for (row, filled) in self.game.board().iter().enumerate() {
            for column in 0..tetris::COLS {
                if filled & (1 << column) != 0 {
                    fill(
                        canvas,
                        column as i32 * CELL,
                        row as i32 * CELL,
                        CELL as u32 - 1,
                        CELL as u32 - 1,
                        BinaryColor::On,
                    );
                }
            }
        }

        let piece = self.game.piece();
        for row in 0..4 {
            for column in 0..4 {
                if piece.mask & (1 << (row * 4 + column)) == 0 {
                    continue;
                }
                let x = piece.x as i32 + column;
                let y = piece.y as i32 + row;
                if x >= 0 && y >= 0 {
                    fill(
                        canvas,
                        x * CELL,
                        y * CELL,
                        CELL as u32 - 1,
                        CELL as u32 - 1,
                        BinaryColor::On,
                    );
                }
            }
        }
    }

    fn draw_hud(&self, canvas: &mut Canvas) {
        fill(canvas, 0, HUD_Y, LOGICAL_WIDTH as u32, 1, BinaryColor::On);

        let next = self.game.next();
        let mask = tetris::piece_mask_for(next);
        let cell = 3;
        let left = (LOGICAL_WIDTH - 4 * cell) / 2;
        for row in 0..4 {
            for column in 0..4 {
                if mask & (1 << (row * 4 + column)) != 0 {
                    fill(
                        canvas,
                        left + column * cell,
                        HUD_Y + 3 + row * cell,
                        cell as u32 - 1,
                        cell as u32 - 1,
                        BinaryColor::On,
                    );
                }
            }
        }

        let mut score = heapless::String::<8>::new();
        write!(&mut score, "{}", self.game.score()).unwrap();
        centred_text(canvas, HUD_Y + 20, score.as_str(), 1);
    }

    fn draw_ready(&self, canvas: &mut Canvas) {
        fill(
            canvas,
            0,
            0,
            LOGICAL_WIDTH as u32,
            BOARD_HEIGHT as u32,
            BinaryColor::Off,
        );
        let lines = [
            ("TETRIS", 10),
            ("SIDEWAYS", 20),
            ("C ROTATE", 34),
            ("L R MOVE", 42),
            ("D DROP", 50),
            ("U PAUSE", 58),
            (self.quit_line(), 68),
        ];
        for (message, y) in lines {
            centred_text(canvas, y, message, 1);
        }
    }

    fn draw_paused(&self, canvas: &mut Canvas) {
        fill(
            canvas,
            0,
            0,
            LOGICAL_WIDTH as u32,
            BOARD_HEIGHT as u32,
            BinaryColor::Off,
        );
        for (message, y) in [("PAUSED", 28), ("C RESUME", 40), (self.quit_line(), 52)] {
            centred_text(canvas, y, message, 1);
        }
    }

    fn draw_dead(&self, canvas: &mut Canvas) {
        fill(
            canvas,
            0,
            0,
            LOGICAL_WIDTH as u32,
            BOARD_HEIGHT as u32,
            BinaryColor::Off,
        );
        let mut score = heapless::String::<8>::new();
        write!(&mut score, "{}", self.game.score()).unwrap();
        for (message, y) in [
            ("GAME", 18),
            ("OVER", 26),
            ("SCORE", 38),
            (score.as_str(), 46),
            ("C AGAIN", 58),
            (self.quit_line(), 68),
        ] {
            centred_text(canvas, y, message, 1);
        }
    }
}

impl Screen for Tetris {
    fn enter(&mut self) {
        self.game.reset();
        self.exit_armed = 0;
        backlight::set(backlight::DEFAULT);
    }

    fn draw(&self, canvas: &mut Canvas) {
        match self.game.state() {
            State::Ready => self.draw_ready(canvas),
            State::Playing => {
                self.draw_board(canvas);
                self.draw_hud(canvas);
            }
            State::Paused => {
                self.draw_board(canvas);
                self.draw_hud(canvas);
                self.draw_paused(canvas);
            }
            State::Dead => {
                self.draw_board(canvas);
                self.draw_hud(canvas);
                self.draw_dead(canvas);
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
                // the button on the player's left, which is "down" on the badge
                Button::Down => self.exit_response(),
                _ => {
                    self.exit_armed = 0;
                    Response::Ignored
                }
            },
            State::Playing => match button {
                Button::Center => {
                    self.exit_armed = 0;
                    self.game.rotate_piece();
                    Response::Redraw
                }
                // world right
                Button::Up => {
                    self.exit_armed = 0;
                    self.game.shift(1);
                    Response::Redraw
                }
                // world left
                Button::Down => {
                    self.exit_armed = 0;
                    self.game.shift(-1);
                    Response::Redraw
                }
                // world down: all the way to the bottom
                Button::Right => {
                    self.exit_armed = 0;
                    self.game.hard_drop();
                    Response::Redraw
                }
                // world up
                Button::Left => {
                    self.exit_armed = 0;
                    self.game.pause();
                    Response::Redraw
                }
            },
            State::Paused => match button {
                Button::Center => {
                    self.exit_armed = 0;
                    self.game.start();
                    Response::Redraw
                }
                Button::Down => self.exit_response(),
                _ => {
                    self.exit_armed = 0;
                    Response::Ignored
                }
            },
            State::Dead => match button {
                Button::Center => {
                    self.exit_armed = 0;
                    self.game.reset();
                    self.game.start();
                    self.leave();
                    Response::Redraw
                }
                Button::Down => self.exit_response(),
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
        if !self.game.tick() {
            return Response::Ignored;
        }
        if self.game.state() == State::Dead {
            backlight::set(backlight::Mode::Solid(DEATH_COLOR));
        }
        Response::Redraw
    }
}
