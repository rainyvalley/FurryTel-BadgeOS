use core::fmt::Write as _;

use embassy_time::Duration;
use embedded_graphics::{
    Drawable,
    geometry::Point,
    text::{Alignment, Baseline, Text, TextStyleBuilder},
};

use crate::{
    hardware, record, store,
    ui::{self, Button, Canvas, Response, Screen},
};

const LETTERS: u8 = 26;

fn centred(canvas: &mut Canvas, message: &str, y: i32, big: bool) {
    let style = TextStyleBuilder::new()
        .alignment(Alignment::Center)
        .baseline(Baseline::Top)
        .build();
    let at = Point::new(hardware::DISPLAY_WIDTH as i32 / 2, y);
    if big {
        Text::with_text_style(message, at, ui::TEXT, style).draw(canvas)
    } else {
        Text::with_text_style(message, at, ui::SMALL_TEXT, style).draw(canvas)
    }
    .unwrap();
}

/// One game's top scores: `rank NAME score`, three lines under a title.
fn draw_list(canvas: &mut Canvas, game: usize) {
    let data = store::data();
    centred(canvas, record::GAME_NAMES[game], 0, false);
    for (rank, entry) in data.scores(game).iter().enumerate() {
        let mut line = heapless::String::<24>::new();
        let name = core::str::from_utf8(&entry.name).unwrap_or("---");
        if entry.score == 0 {
            write!(&mut line, "{}  ---", rank + 1).unwrap();
        } else {
            write!(&mut line, "{}  {}  {}", rank + 1, name, entry.score).unwrap();
        }
        centred(canvas, &line, 8 + rank as i32 * 8, false);
    }
}

/// The high scores of every game, one game per page.
pub struct Scoreboard {
    game: usize,
}

impl Scoreboard {
    pub fn new() -> Self {
        Self { game: 0 }
    }
}

impl Screen for Scoreboard {
    fn enter(&mut self) {
        self.game = 0;
    }

    fn draw(&self, canvas: &mut Canvas) {
        draw_list(canvas, self.game);
        centred(canvas, "▲▼ game   ◀ back", 26, false);
    }

    fn handle(&mut self, button: Button) -> Response {
        let games = record::GAMES;
        match button {
            Button::Left => return Response::Exit,
            Button::Up => self.game = (self.game + games - 1) % games,
            _ => self.game = (self.game + 1) % games,
        }
        Response::Redraw
    }
}

enum Phase {
    /// the game is in charge
    Playing,
    /// choosing three initials: which letters, and which one is being changed
    Initials { name: [u8; 3], cursor: usize },
    /// showing the list the new score is on
    Board,
}

/// Wraps a game so that a score good enough for its top three asks for initials, saves them, and
/// shows the list, before handing back to the game's own game-over screen.
pub struct Scored<S: Screen> {
    game: usize,
    inner: S,
    phase: Phase,
    /// the score has been dealt with for this game over
    handled: bool,
}

impl<S: Screen> Scored<S> {
    pub fn new(game: usize, inner: S) -> Self {
        Self {
            game,
            inner,
            phase: Phase::Playing,
            handled: false,
        }
    }

    /// After the game moves on, see whether it has just ended with a score worth keeping.
    fn check(&mut self, response: Response) -> Response {
        match self.inner.final_score() {
            None => self.handled = false,
            Some(score) if !self.handled => {
                self.handled = true;
                if store::qualifies(self.game, score) {
                    self.phase = Phase::Initials {
                        name: *b"AAA",
                        cursor: 0,
                    };
                    return Response::Redraw;
                }
            }
            Some(_) => {}
        }
        response
    }
}

impl<S: Screen> Screen for Scored<S> {
    fn enter(&mut self) {
        self.phase = Phase::Playing;
        self.handled = false;
        self.inner.enter();
    }

    fn draw(&self, canvas: &mut Canvas) {
        match &self.phase {
            Phase::Playing => self.inner.draw(canvas),
            Phase::Initials { name, cursor } => {
                let score = self.inner.final_score().unwrap_or(0);
                let mut line = heapless::String::<24>::new();
                write!(&mut line, "NEW HIGH SCORE {score}").unwrap();
                centred(canvas, &line, 0, false);
                let mut letters = heapless::String::<8>::new();
                for (n, &letter) in name.iter().enumerate() {
                    letters.push(if n == *cursor { '[' } else { ' ' }).unwrap();
                    letters.push(letter as char).unwrap();
                    letters.push(if n == *cursor { ']' } else { ' ' }).unwrap();
                }
                centred(canvas, &letters, 9, true);
                centred(canvas, "▲▼ letter  ◀▶ move  ● ok", 25, false);
            }
            Phase::Board => {
                draw_list(canvas, self.game);
                centred(canvas, "any button", 26, false);
            }
        }
    }

    fn handle(&mut self, button: Button) -> Response {
        match &mut self.phase {
            Phase::Playing => {
                let response = self.inner.handle(button);
                self.check(response)
            }
            Phase::Initials { name, cursor } => {
                match button {
                    Button::Up => name[*cursor] = (name[*cursor] - b'A' + 1) % LETTERS + b'A',
                    Button::Down => {
                        name[*cursor] = (name[*cursor] - b'A' + LETTERS - 1) % LETTERS + b'A';
                    }
                    Button::Left => *cursor = cursor.saturating_sub(1),
                    Button::Right => *cursor = (*cursor + 1).min(2),
                    Button::Center if *cursor < 2 => *cursor += 1,
                    Button::Center => {
                        let score = self.inner.final_score().unwrap_or(0);
                        store::add_score(self.game, *name, score);
                        self.phase = Phase::Board;
                    }
                }
                Response::Redraw
            }
            Phase::Board => {
                self.phase = Phase::Playing;
                Response::Redraw
            }
        }
    }

    fn tick_interval(&self) -> Option<Duration> {
        match self.phase {
            Phase::Playing => self.inner.tick_interval(),
            _ => None,
        }
    }

    fn tick(&mut self) -> Response {
        let response = self.inner.tick();
        self.check(response)
    }
}
