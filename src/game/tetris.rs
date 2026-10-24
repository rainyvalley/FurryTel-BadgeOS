//! The rules of tetris: an 8-column well, rotated into the screen's 32-pixel width.

use super::rng::Rng;

pub const COLS: u8 = 8;
pub const ROWS: u8 = 24;

/// The seven tetrominoes, each as a 4x4 mask: bit `row * 4 + column`.
const PIECES: [u16; 7] = [
    0x00F0, // I
    0x0033, // O
    0x0072, // T
    0x0036, // S
    0x0063, // Z
    0x0071, // J
    0x0074, // L
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    Ready,
    Playing,
    Paused,
    Dead,
}

#[derive(Clone, Copy)]
pub struct Piece {
    /// the rotated shape
    pub mask: u16,
    pub x: i8,
    pub y: i8,
}

pub struct TetrisGame {
    rng: Rng,
    /// bit `column` set means the cell is filled
    board: [u8; ROWS as usize],
    piece: Piece,
    next: u8,
    score: u32,
    lines: u32,
    state: State,
}

/// The starting shape of a piece, for the next-piece preview.
pub fn piece_mask_for(kind: u8) -> u16 {
    PIECES[kind as usize]
}

fn bit(row: u32, column: u32) -> u16 {
    1 << (row * 4 + column)
}

fn rotate(mask: u16) -> u16 {
    let mut out = 0;
    for row in 0..4 {
        for column in 0..4 {
            if mask & bit(row, column) != 0 {
                out |= bit(column, 3 - row);
            }
        }
    }
    out
}

impl TetrisGame {
    pub fn new(seed: u64) -> Self {
        let mut this = Self {
            rng: Rng::new(seed),
            board: [0; ROWS as usize],
            piece: Piece {
                mask: PIECES[0],
                x: 2,
                y: 0,
            },
            next: 0,
            score: 0,
            lines: 0,
            state: State::Ready,
        };
        this.reset();
        this
    }

    pub fn reset(&mut self) {
        self.board = [0; ROWS as usize];
        self.score = 0;
        self.lines = 0;
        self.next = self.rng.below(7);
        self.spawn();
        self.state = State::Ready;
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn score(&self) -> u32 {
        self.score
    }

    pub fn board(&self) -> &[u8; ROWS as usize] {
        &self.board
    }

    pub fn piece(&self) -> Piece {
        self.piece
    }

    pub fn next(&self) -> u8 {
        self.next
    }

    /// How long the next gravity step should take.
    pub fn delay_ms(&self) -> u64 {
        600_u64.saturating_sub(u64::from(self.lines) * 20).max(120)
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

    pub fn shift(&mut self, dx: i8) -> bool {
        if self.state != State::Playing {
            return false;
        }
        self.try_move(dx, 0)
    }

    pub fn rotate_piece(&mut self) -> bool {
        if self.state != State::Playing {
            return false;
        }
        let rotated = rotate(self.piece.mask);
        // a simple wall kick: try the same place, then a step either way
        for dx in [0, -1, 1, -2, 2] {
            if !self.collides(rotated, self.piece.x + dx, self.piece.y) {
                self.piece.mask = rotated;
                self.piece.x += dx;
                return true;
            }
        }
        false
    }

    /// All the way to the bottom.  The piece is left there for the next gravity tick to lock.
    pub fn hard_drop(&mut self) -> bool {
        if self.state != State::Playing {
            return false;
        }
        let mut moved = false;
        while self.try_move(0, 1) {
            moved = true;
        }
        moved
    }

    /// Gravity: down a row, or lock the piece and bring in the next one.
    pub fn tick(&mut self) -> bool {
        if self.state != State::Playing {
            return false;
        }
        if self.try_move(0, 1) {
            return true;
        }
        self.lock();
        true
    }

    fn try_move(&mut self, dx: i8, dy: i8) -> bool {
        let x = self.piece.x + dx;
        let y = self.piece.y + dy;
        if self.collides(self.piece.mask, x, y) {
            return false;
        }
        self.piece.x = x;
        self.piece.y = y;
        true
    }

    fn collides(&self, mask: u16, x: i8, y: i8) -> bool {
        for row in 0..4 {
            for column in 0..4 {
                if mask & bit(row, column) == 0 {
                    continue;
                }
                let column = x + column as i8;
                let row = y + row as i8;
                if !(0..COLS as i8).contains(&column) || row >= ROWS as i8 {
                    return true;
                }
                if row >= 0 && self.board[row as usize] & (1 << column) != 0 {
                    return true;
                }
            }
        }
        false
    }

    fn lock(&mut self) {
        for row in 0..4 {
            for column in 0..4 {
                if self.piece.mask & bit(row, column) == 0 {
                    continue;
                }
                let column = self.piece.x + column as i8;
                let row = self.piece.y + row as i8;
                if (0..ROWS as i8).contains(&row) && (0..COLS as i8).contains(&column) {
                    // PANIC SAFETY: the bounds are checked just above
                    self.board[row as usize] |= 1 << column;
                }
            }
        }

        let cleared = self.clear_lines();
        self.score += match cleared {
            1 => 100,
            2 => 300,
            3 => 500,
            4 => 800,
            _ => 0,
        };

        self.spawn();
        if self.collides(self.piece.mask, self.piece.x, self.piece.y) {
            self.state = State::Dead;
        }
    }

    fn clear_lines(&mut self) -> u32 {
        // PANIC SAFETY: the board is one byte per row, so there are eight columns
        let full = u8::MAX;
        let mut cleared = 0;
        let mut row = ROWS as usize;
        while row > 0 {
            row -= 1;
            if self.board[row] == full {
                cleared += 1;
                for shift in (1..=row).rev() {
                    self.board[shift] = self.board[shift - 1];
                }
                self.board[0] = 0;
                // the row above just moved down into this one, so look at it again
                row += 1;
            }
        }
        self.lines += cleared;
        cleared
    }

    fn spawn(&mut self) {
        let kind = self.next;
        self.next = self.rng.below(7);
        self.piece = Piece {
            mask: PIECES[kind as usize],
            x: 2,
            y: 0,
        };
    }

    #[cfg(test)]
    pub fn play(&mut self) {
        self.state = State::Playing;
    }

    #[cfg(test)]
    pub fn set_board_row(&mut self, row: u8, filled: u8) {
        self.board[row as usize] = filled;
    }

    #[cfg(test)]
    pub fn set_piece(&mut self, kind: u8, x: i8, y: i8) {
        self.piece = Piece {
            mask: PIECES[kind as usize],
            x,
            y,
        };
    }

    #[cfg(test)]
    pub fn rotate_mask(mask: u16) -> u16 {
        rotate(mask)
    }

    #[cfg(test)]
    pub fn piece_mask(kind: u8) -> u16 {
        PIECES[kind as usize]
    }
}
