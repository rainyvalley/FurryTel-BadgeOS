//! A secret, watched for on every screen: the Konami code unlocks [`Snake`].
//!
//! Presses that continue the code are swallowed, so entering it doesn't also drive whatever is on
//! screen.  A press that breaks the code goes through to the screen as usual, so the badge stays
//! usable while someone is halfway through the sequence.

use embassy_time::{Duration, Instant};

use crate::screens::Snake;

use super::{Button, Canvas, Response, Screen};

/// Up Up Down Down Left Right Left Right Center Center.
const SEQUENCE: [Button; 10] = [
    Button::Up,
    Button::Up,
    Button::Down,
    Button::Down,
    Button::Left,
    Button::Right,
    Button::Left,
    Button::Right,
    Button::Center,
    Button::Center,
];

/// How long a partial code stays alive without the next press.
const TIMEOUT: Duration = Duration::from_secs(3);
const TIMEOUT_CHECK: Duration = Duration::from_millis(250);

/// Wraps another screen and listens for the code.  While the snake is out, it gets every press.
pub struct Konami<'a> {
    inner: &'a mut dyn Screen,
    snake: Snake,
    /// how many presses of the code have matched so far
    progress: usize,
    last_press: Option<Instant>,
    snake_out: bool,
}

impl<'a> Konami<'a> {
    pub fn new(inner: &'a mut dyn Screen, snake: Snake) -> Self {
        Self {
            inner,
            snake,
            progress: 0,
            last_press: None,
            snake_out: false,
        }
    }

    fn reset_progress(&mut self) {
        self.progress = 0;
        self.last_press = None;
    }
}

impl Screen for Konami<'_> {
    fn enter(&mut self) {
        self.inner.enter();
    }

    fn draw(&self, canvas: &mut Canvas) {
        if self.snake_out {
            self.snake.draw(canvas);
        } else {
            self.inner.draw(canvas);
        }
    }

    fn handle(&mut self, button: Button) -> Response {
        if self.snake_out {
            return match self.snake.handle(button) {
                Response::Exit => {
                    self.snake_out = false;
                    Response::Redraw
                }
                response => response,
            };
        }

        if !self.inner.secret_ok() {
            self.reset_progress();
            return self.inner.handle(button);
        }

        if button == SEQUENCE[self.progress] {
            self.progress += 1;
            self.last_press = Some(Instant::now());
            if self.progress == SEQUENCE.len() {
                self.reset_progress();
                self.snake_out = true;
                self.snake.enter();
                return Response::Redraw;
            }
            return Response::Ignored;
        }

        // start over, but count this press if it's the first one of the code
        self.progress = usize::from(button == SEQUENCE[0]);
        self.last_press = (self.progress > 0).then(Instant::now);
        self.inner.handle(button)
    }

    fn tick_interval(&self) -> Option<Duration> {
        let content = if self.snake_out {
            self.snake.tick_interval()
        } else {
            self.inner.tick_interval()
        };
        let timeout = (!self.snake_out && self.progress > 0 && self.inner.secret_ok())
            .then_some(TIMEOUT_CHECK);
        match (content, timeout) {
            (Some(content), Some(timeout)) => Some(content.min(timeout)),
            (content, timeout) => content.or(timeout),
        }
    }

    fn tick(&mut self) -> Response {
        if self.snake_out {
            return self.snake.tick();
        }

        if !self.inner.secret_ok() {
            self.reset_progress();
        } else if let Some(last) = self.last_press
            && Instant::now() - last >= TIMEOUT
        {
            self.reset_progress();
        }
        self.inner.tick()
    }
}
