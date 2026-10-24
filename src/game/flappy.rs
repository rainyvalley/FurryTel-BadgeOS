//! The rules of flappy bird, in sixteenths of a pixel so the motion stays smooth without floats.

use super::rng::Rng;

pub const WIDTH: i32 = 128;
pub const HEIGHT: i32 = 32;
pub const BIRD_X: i32 = 24;
pub const BIRD_W: i32 = 8;
pub const BIRD_H: i32 = 6;
pub const PIPE_W: i32 = 8;
/// Height of the hole the bird flies through.
pub const GAP: i32 = 14;

const PIPE_COUNT: usize = 2;
const PIPE_SPACING: i32 = 64;
const GAP_MARGIN: i32 = 5;

/// Fixed point: this many sixteenths per pixel.
const FP: i32 = 16;
const START_Y: i32 = 12 * FP;
/// Added to the fall speed each tick: half a pixel per tick, per tick.
const GRAVITY: i32 = 8;
/// A flap sets the fall speed to this: two and a half pixels up per tick.
const FLAP: i32 = -40;
const MAX_FALL: i32 = 6 * FP;
/// Pipes move this many pixels per tick.
const SPEED: i32 = 1;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    /// Waiting for the player to start.
    Ready,
    Playing,
    Dead,
}

#[derive(Clone, Copy)]
pub struct Pipe {
    pub x: i32,
    /// y of the top of the hole.
    pub gap_top: i32,
    scored: bool,
}

/// One game of flappy bird.
pub struct FlappyGame {
    rng: Rng,
    /// Top of the bird, in sixteenths of a pixel.
    y: i32,
    /// Pixels per tick, in sixteenths.
    velocity: i32,
    pipes: [Pipe; PIPE_COUNT],
    score: u32,
    state: State,
}

impl FlappyGame {
    pub fn new(seed: u64) -> Self {
        let mut this = Self {
            rng: Rng::new(seed),
            y: START_Y,
            velocity: 0,
            pipes: [Pipe {
                x: WIDTH,
                gap_top: GAP_MARGIN,
                scored: false,
            }; PIPE_COUNT],
            score: 0,
            state: State::Ready,
        };
        this.reset();
        this
    }

    pub fn reset(&mut self) {
        self.y = START_Y;
        self.velocity = 0;
        self.score = 0;
        for i in 0..PIPE_COUNT {
            let x = WIDTH + i as i32 * PIPE_SPACING;
            self.pipes[i] = self.new_pipe(x);
        }
        self.state = State::Ready;
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn score(&self) -> u32 {
        self.score
    }

    pub fn pipes(&self) -> &[Pipe; PIPE_COUNT] {
        &self.pipes
    }

    /// Top of the bird, in whole pixels.
    pub fn bird_y(&self) -> i32 {
        self.y / FP
    }

    pub fn start(&mut self) {
        if self.state == State::Ready {
            self.state = State::Playing;
            self.velocity = FLAP;
        }
    }

    pub fn flap(&mut self) {
        if self.state == State::Playing {
            self.velocity = FLAP;
        }
    }

    /// Move one step.  Returns false if the game isn't running.
    pub fn tick(&mut self) -> bool {
        if self.state != State::Playing {
            return false;
        }

        self.velocity = (self.velocity + GRAVITY).min(MAX_FALL);
        self.y += self.velocity;

        if self.y + BIRD_H * FP >= HEIGHT * FP {
            return self.die();
        }
        if self.y < 0 {
            self.y = 0;
            if self.velocity < 0 {
                self.velocity = 0;
            }
        }

        for i in 0..PIPE_COUNT {
            self.pipes[i].x -= SPEED;

            if !self.pipes[i].scored && self.pipes[i].x + PIPE_W < BIRD_X {
                self.pipes[i].scored = true;
                self.score += 1;
            }

            if self.pipes[i].x + PIPE_W < 0 {
                let x = self.pipes[i].x + PIPE_COUNT as i32 * PIPE_SPACING;
                self.pipes[i] = self.new_pipe(x);
            }
        }

        let y = self.bird_y();
        for pipe in &self.pipes {
            let overlaps_x = BIRD_X + BIRD_W > pipe.x && BIRD_X < pipe.x + PIPE_W;
            let in_gap = y >= pipe.gap_top && y + BIRD_H <= pipe.gap_top + GAP;
            if overlaps_x && !in_gap {
                return self.die();
            }
        }

        true
    }

    fn die(&mut self) -> bool {
        self.state = State::Dead;
        true
    }

    fn new_pipe(&mut self, x: i32) -> Pipe {
        Pipe {
            x,
            gap_top: self.rng.range(GAP_MARGIN, HEIGHT - GAP - GAP_MARGIN),
            scored: false,
        }
    }

    #[cfg(test)]
    pub fn play(&mut self) {
        self.state = State::Playing;
    }

    #[cfg(test)]
    pub fn set_pipes(&mut self, pipes: [(i32, i32); PIPE_COUNT]) {
        for (pipe, (x, gap_top)) in self.pipes.iter_mut().zip(pipes) {
            *pipe = Pipe {
                x,
                gap_top,
                scored: false,
            };
        }
    }

    #[cfg(test)]
    pub fn set_y(&mut self, y: i32) {
        self.y = y * FP;
    }

    #[cfg(test)]
    pub fn velocity(&self) -> i32 {
        self.velocity
    }
}
