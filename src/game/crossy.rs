//! The rules of crossy road: hop up the lanes, don't get run over.

use super::rng::Rng;

pub const COLS: u8 = 32;
pub const ROWS: u8 = 8;
/// Cars per road lane, spaced evenly around its loop.
pub const CARS: u8 = 3;
const CAR_SPACING: u8 = 11;
/// A car is this many cells long.
pub const CAR_LEN: u8 = 2;
/// Forward hops that make up one level.
pub const LANES_PER_LEVEL: u32 = 10;
/// How long the "level 2" banner stays up, in ticks.
const BANNER_TICKS: u8 = 25;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Lane {
    Grass,
    Road {
        leftward: bool,
        /// ticks between one-cell moves
        period: u8,
        /// cells the cars have moved around their loop
        offset: u8,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    Ready,
    Playing,
    Paused,
    Dead,
}

/// Where car `k` of `lane` starts, in cells.  A car covers this cell and the next one.
pub fn car_start(lane: Lane, k: u8) -> u8 {
    match lane {
        Lane::Grass => 0,
        Lane::Road {
            leftward, offset, ..
        } => {
            let base = k * CAR_SPACING;
            if leftward {
                (base + COLS - offset) % COLS
            } else {
                (base + offset) % COLS
            }
        }
    }
}

pub fn lane_covers(lane: Lane, col: u8) -> bool {
    match lane {
        Lane::Grass => false,
        Lane::Road { .. } => (0..CARS).any(|k| {
            let start = car_start(lane, k);
            col == start || col == (start + 1) % COLS
        }),
    }
}

pub struct CrossyGame {
    rng: Rng,
    lanes: [Lane; ROWS as usize],
    counters: [u8; ROWS as usize],
    pub col: u8,
    pub row: u8,
    score: u32,
    level: u32,
    /// forward hops made this level
    progress: u32,
    /// ticks left on the new-level banner
    banner: u8,
    state: State,
}

impl CrossyGame {
    pub fn new(seed: u64) -> Self {
        let mut this = Self {
            rng: Rng::new(seed),
            lanes: [Lane::Grass; ROWS as usize],
            counters: [0; ROWS as usize],
            col: COLS / 2,
            row: ROWS - 1,
            score: 0,
            level: 1,
            progress: 0,
            banner: 0,
            state: State::Ready,
        };
        this.reset();
        this
    }

    pub fn reset(&mut self) {
        self.col = COLS / 2;
        self.row = ROWS - 1;
        self.score = 0;
        self.level = 1;
        self.progress = 0;
        self.banner = 0;
        self.reset_lanes();
        self.state = State::Ready;
    }

    fn reset_lanes(&mut self) {
        self.counters = [0; ROWS as usize];
        self.lanes = [Lane::Grass; ROWS as usize];
        for i in 0..ROWS as usize - 2 {
            self.lanes[i] = self.random_lane();
        }
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn score(&self) -> u32 {
        self.score
    }

    pub fn level(&self) -> u32 {
        self.level
    }

    /// Whether the new-level banner should be showing.
    pub fn banner(&self) -> bool {
        self.banner > 0
    }

    pub fn lanes(&self) -> &[Lane; ROWS as usize] {
        &self.lanes
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

    /// Hop one cell.  Returns false if the game isn't running.
    pub fn hop(&mut self, dir: Dir) -> bool {
        if self.state != State::Playing {
            return false;
        }

        match dir {
            Dir::Up => {
                if self.row == 0 {
                    self.scroll();
                } else {
                    self.row -= 1;
                }
                self.score += 1;
                self.progress += 1;
            }
            Dir::Down => self.row = (self.row + 1).min(ROWS - 1),
            Dir::Left => self.col = self.col.saturating_sub(1),
            Dir::Right => self.col = (self.col + 1).min(COLS - 1),
        }

        if self.collides() {
            self.state = State::Dead;
            return true;
        }

        if self.progress >= LANES_PER_LEVEL {
            self.next_level();
        }
        true
    }

    /// A new set of lanes, with the player back at the bottom.
    fn next_level(&mut self) {
        self.level += 1;
        self.progress = 0;
        self.banner = BANNER_TICKS;
        self.col = COLS / 2;
        self.row = ROWS - 1;
        self.reset_lanes();
    }

    /// Move the cars one step.  Returns false if the game isn't running.
    pub fn tick(&mut self) -> bool {
        if self.state != State::Playing {
            return false;
        }

        if self.banner > 0 {
            self.banner -= 1;
        }

        let mut moved = false;
        for i in 0..ROWS as usize {
            let mut lane = self.lanes[i];
            if let Lane::Road {
                leftward,
                period,
                offset,
            } = lane
            {
                self.counters[i] += 1;
                if self.counters[i] >= period {
                    self.counters[i] = 0;
                    lane = Lane::Road {
                        leftward,
                        period,
                        offset: (offset + 1) % COLS,
                    };
                    self.lanes[i] = lane;
                    moved = true;
                }
            }
        }

        if moved && self.collides() {
            self.state = State::Dead;
        }
        moved
    }

    fn collides(&self) -> bool {
        lane_covers(self.lanes[self.row as usize], self.col)
    }

    /// Add a new lane at the top and push the rest down, with the player staying on it.
    fn scroll(&mut self) {
        for i in (1..ROWS as usize).rev() {
            self.lanes[i] = self.lanes[i - 1];
        }
        self.lanes[0] = self.random_lane();
        self.counters = [0; ROWS as usize];
    }

    fn random_lane(&mut self) -> Lane {
        if self.rng.below(100) < 45 {
            return Lane::Grass;
        }

        for _ in 0..8 {
            let lane = Lane::Road {
                leftward: self.rng.below(2) == 0,
                period: 2 + self.rng.below(3),
                offset: self.rng.below(COLS),
            };
            // never drop a car onto the player
            if !lane_covers(lane, self.col) {
                return lane;
            }
        }
        Lane::Grass
    }

    #[cfg(test)]
    pub fn set_lane(&mut self, row: u8, lane: Lane) {
        self.lanes[row as usize] = lane;
    }

    #[cfg(test)]
    pub fn set_player(&mut self, col: u8, row: u8) {
        self.col = col;
        self.row = row;
    }

    #[cfg(test)]
    pub fn play(&mut self) {
        self.state = State::Playing;
    }
}
