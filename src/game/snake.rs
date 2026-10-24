//! The rules of snake, with no display or hardware in them.

use super::rng::Rng;

/// The playfield is 32 x 8 cells, which tiles the 128 x 32 screen at 4 pixels per cell.
pub const COLS: u8 = 32;
pub const ROWS: u8 = 8;
const CELLS: usize = COLS as usize * ROWS as usize;

const START_LENGTH: u8 = 3;
/// Milliseconds between steps at the start of a game, and the fastest it ever gets.
const START_DELAY_MS: u64 = 200;
const MIN_DELAY_MS: u64 = 80;
/// Each point of score makes the snake this much faster.
const SPEEDUP_MS: u64 = 6;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

impl Dir {
    fn delta(self) -> (i8, i8) {
        match self {
            Dir::Up => (0, -1),
            Dir::Down => (0, 1),
            Dir::Left => (-1, 0),
            Dir::Right => (1, 0),
        }
    }

    pub fn opposite(self) -> Dir {
        match self {
            Dir::Up => Dir::Down,
            Dir::Down => Dir::Up,
            Dir::Left => Dir::Right,
            Dir::Right => Dir::Left,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    /// Waiting for the player to start.
    Ready,
    Playing,
    Paused,
    Dead,
}

/// What one step did.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Step {
    Moved,
    /// The snake ate the food and grew.
    Ate,
    /// The snake ran into a wall or itself.
    Died,
}

/// One game of snake.
pub struct SnakeGame {
    rng: Rng,
    /// The snake's cells in a ring buffer, newest at `head`.
    cells: [(u8, u8); CELLS],
    head: usize,
    length: usize,
    dir: Dir,
    /// The direction the next step uses, so a turn can't immediately double back into the neck.
    next_dir: Dir,
    food: (u8, u8),
    score: u32,
    state: State,
}

impl SnakeGame {
    pub fn new(seed: u64) -> Self {
        let mut this = Self {
            rng: Rng::new(seed),
            cells: [(0, 0); CELLS],
            head: 0,
            length: 0,
            dir: Dir::Right,
            next_dir: Dir::Right,
            food: (0, 0),
            score: 0,
            state: State::Ready,
        };
        this.reset();
        this
    }

    /// Start over, on the "press Center" screen.
    pub fn reset(&mut self) {
        self.length = 0;
        self.head = 0;
        self.score = 0;
        self.dir = Dir::Right;
        self.next_dir = Dir::Right;
        let row = ROWS / 2;
        for x in COLS / 4..COLS / 4 + START_LENGTH {
            self.push((x, row));
        }
        self.place_food();
        self.state = State::Ready;
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn score(&self) -> u32 {
        self.score
    }

    #[cfg(test)]
    pub fn length(&self) -> usize {
        self.length
    }

    pub fn food(&self) -> (u8, u8) {
        self.food
    }

    /// How long the next step should take.
    pub fn delay_ms(&self) -> u64 {
        START_DELAY_MS
            .saturating_sub(u64::from(self.score) * SPEEDUP_MS)
            .max(MIN_DELAY_MS)
    }

    /// Newest first, which is how they're drawn.
    pub fn segments(&self) -> impl Iterator<Item = (u8, u8)> + '_ {
        (0..self.length).map(|i| self.cells[(self.head + CELLS - i) % CELLS])
    }

    pub fn start(&mut self) {
        if matches!(self.state, State::Ready | State::Paused) {
            self.state = State::Playing;
        }
    }

    pub fn pause(&mut self) {
        if self.state == State::Playing {
            self.state = State::Paused;
        }
    }

    /// Steer, if the snake isn't already mid-turn.  Nothing moves until the next step.
    pub fn turn(&mut self, dir: Dir) {
        if self.next_dir != self.dir || dir == self.dir || dir == self.dir.opposite() {
            return;
        }
        self.next_dir = dir;
    }

    /// Move one cell.  `None` if the game isn't running.
    pub fn step(&mut self) -> Option<Step> {
        if self.state != State::Playing {
            return None;
        }

        self.dir = self.next_dir;
        let (dx, dy) = self.dir.delta();
        let (x, y) = self.cells[self.head];
        let (x, y) = (i16::from(x) + i16::from(dx), i16::from(y) + i16::from(dy));

        if x < 0 || y < 0 || x >= i16::from(COLS) || y >= i16::from(ROWS) {
            return Some(self.die());
        }
        let next = (x as u8, y as u8);

        let eating = next == self.food;
        // the tail moves out of the way this step, unless the snake is growing
        let solid = self.length - usize::from(!eating);
        if self.segments().take(solid).any(|cell| cell == next) {
            return Some(self.die());
        }

        self.head = (self.head + 1) % CELLS;
        self.cells[self.head] = next;

        if !eating {
            return Some(Step::Moved);
        }

        self.length += 1;
        self.score += 1;
        if self.length >= CELLS {
            return Some(self.die());
        }
        self.place_food();
        Some(Step::Ate)
    }

    fn die(&mut self) -> Step {
        self.state = State::Dead;
        Step::Died
    }

    fn push(&mut self, cell: (u8, u8)) {
        self.head = if self.length == 0 {
            0
        } else {
            (self.head + 1) % CELLS
        };
        self.cells[self.head] = cell;
        self.length += 1;
    }

    fn place_food(&mut self) {
        for _ in 0..CELLS {
            let cell = (self.rng.below(COLS), self.rng.below(ROWS));
            if !self.occupied(cell) {
                self.food = cell;
                return;
            }
        }
    }

    fn occupied(&self, cell: (u8, u8)) -> bool {
        self.segments().any(|other| other == cell)
    }

    #[cfg(test)]
    pub fn set_food(&mut self, cell: (u8, u8)) {
        self.food = cell;
    }
}
