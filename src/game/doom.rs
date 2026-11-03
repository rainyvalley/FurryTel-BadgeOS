//! A tiny first-person shooter: a Wolfenstein-style raycaster rendered into a 128x32 one-bit frame.
//! Everything is integer maths, since the RP2040 has no FPU.  Positions are in 1/256ths of a cell
//! and angles in 1/256ths of a turn, clockwise from east (the map's y axis points down).

use super::rng::Rng;

pub const WIDTH: usize = 128;
pub const HEIGHT: usize = 32;
pub const MAP_SIZE: usize = 16;
pub const MAX_ENEMIES: usize = 8;

/// Where the player starts, as a cell.
const START: (i32, i32) = (1, 1);
const MAP: [&[u8; MAP_SIZE]; MAP_SIZE] = [
    b"################",
    b"#..............#",
    b"#.####..####...#",
    b"#.#......E...#.#",
    b"#.#..####....#.#",
    b"#....#..#....E.#",
    b"###..#..#..#####",
    b"#......E.......#",
    b"#..##......##..#",
    b"#..#...E....#..#",
    b"#..#..####..#..#",
    b"#.....#..#.....#",
    b"#.##..#..#..##.#",
    b"#.E.........E..#",
    b"#..............#",
    b"################",
];

/// sin of 0..=64 quarter-turn steps, times 256
const SIN: [i16; 65] = [
    0, 6, 13, 19, 25, 31, 38, 44, 50, 56, 62, 68, 74, 80, 86, 92, 98, 104, 109, 115, 121, 126, 132,
    137, 142, 147, 152, 157, 162, 167, 172, 177, 181, 185, 190, 194, 198, 202, 206, 209, 213, 216,
    220, 223, 226, 229, 231, 234, 237, 239, 241, 243, 245, 247, 248, 250, 251, 252, 253, 254, 255,
    255, 256, 256, 256,
];

/// Distance a forward or back press moves, and the angle a turn press covers.
const STEP: i32 = 72;
const TURN: u8 = 8;
/// How close the player may get to a wall.
const MARGIN: i32 = 56;
/// The field of view: the camera plane is this much of the direction vector (about 66 degrees).
const PLANE: i32 = 169;
/// A wall one cell away fills this many pixels.
const SCALE: i64 = 36;
/// Rounds in a magazine, and ticks a reload takes.
pub const MAGAZINE: u8 = 6;
const RELOAD_TICKS: u8 = 3;
const ENEMY_HP: i8 = 2;
const PLAYER_HP: i16 = 100;

fn sin(angle: u8) -> i32 {
    let a = angle as usize;
    i32::from(match a {
        0..=64 => SIN[a],
        65..=128 => SIN[128 - a],
        129..=192 => -SIN[a - 128],
        _ => -SIN[256 - a],
    })
}

fn cos(angle: u8) -> i32 {
    sin(angle.wrapping_add(64))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    Ready,
    Playing,
    Paused,
    Dead,
    Won,
}

#[derive(Clone, Copy)]
struct Enemy {
    /// the cell it stands in
    x: i32,
    y: i32,
    hp: i8,
    cooldown: u8,
}

impl Enemy {
    fn alive(&self) -> bool {
        self.hp > 0
    }

    /// The middle of its cell.
    fn centre(&self) -> (i32, i32) {
        (self.x * 256 + 128, self.y * 256 + 128)
    }
}

/// A one-bit picture, a bit per pixel, row by row.
pub struct Frame {
    bits: [u8; WIDTH * HEIGHT / 8],
}

impl Frame {
    pub fn new() -> Self {
        Self {
            bits: [0; WIDTH * HEIGHT / 8],
        }
    }

    pub fn get(&self, x: usize, y: usize) -> bool {
        let n = y * WIDTH + x;
        self.bits[n / 8] & (1 << (n % 8)) != 0
    }

    fn put(&mut self, x: i32, y: i32, on: bool) {
        if !(0..WIDTH as i32).contains(&x) || !(0..HEIGHT as i32).contains(&y) {
            return;
        }
        let n = y as usize * WIDTH + x as usize;
        if on {
            self.bits[n / 8] |= 1 << (n % 8);
        } else {
            self.bits[n / 8] &= !(1 << (n % 8));
        }
    }

    fn rect(&mut self, x: i32, y: i32, width: i32, height: i32, on: bool) {
        for row in y..y + height {
            for column in x..x + width {
                self.put(column, row, on);
            }
        }
    }

    fn invert(&mut self) {
        for byte in &mut self.bits {
            *byte = !*byte;
        }
    }
}

/// How densely a wall is inked, from solid (0) to a sparse haze (3).
fn shade_on(level: u8, x: i32, y: i32) -> bool {
    match level {
        0 => true,
        1 => !(x & 1 == 0 && y & 1 == 0),
        2 => (x + y) & 1 == 0,
        _ => x & 1 == 0 && y & 1 == 0,
    }
}

/// A demon: which pixels exist, and which of those are ink.  Bit 7 is the left edge.
const DEMON_MASK: [u8; 8] = [0x3C, 0x7E, 0x7E, 0xFF, 0xFF, 0x7E, 0x66, 0xE7];
const DEMON_INK: [u8; 8] = [0x3C, 0x5A, 0x7E, 0xFF, 0xDB, 0x7E, 0x66, 0xE7];

struct Hit {
    /// perpendicular distance, in 1/65536ths of a cell
    distance: i64,
    /// whether it was a wall facing east-west
    side: bool,
}

pub struct DoomGame {
    rng: Rng,
    x: i32,
    y: i32,
    angle: u8,
    health: i16,
    enemies: [Enemy; MAX_ENEMIES],
    count: usize,
    kills: u8,
    state: State,
    flash: u8,
    hurt: u8,
    ticks: u32,
    ammo: u8,
    /// ticks left before the magazine is full again; 0 when not reloading
    reload: u8,
}

fn is_wall(x: i32, y: i32) -> bool {
    if !(0..MAP_SIZE as i32).contains(&x) || !(0..MAP_SIZE as i32).contains(&y) {
        return true;
    }
    MAP[y as usize][x as usize] == b'#'
}

impl DoomGame {
    pub fn new(seed: u64) -> Self {
        let mut this = Self {
            rng: Rng::new(seed),
            x: 0,
            y: 0,
            angle: 0,
            health: PLAYER_HP,
            enemies: [Enemy {
                x: 0,
                y: 0,
                hp: 0,
                cooldown: 0,
            }; MAX_ENEMIES],
            count: 0,
            kills: 0,
            state: State::Ready,
            flash: 0,
            hurt: 0,
            ticks: 0,
            ammo: MAGAZINE,
            reload: 0,
        };
        this.reset();
        this
    }

    pub fn reset(&mut self) {
        self.x = START.0 * 256 + 128;
        self.y = START.1 * 256 + 128;
        self.angle = 0;
        self.health = PLAYER_HP;
        self.kills = 0;
        self.flash = 0;
        self.hurt = 0;
        self.ticks = 0;
        self.ammo = MAGAZINE;
        self.reload = 0;
        self.count = 0;
        for (row, line) in MAP.iter().enumerate() {
            for (column, &cell) in line.iter().enumerate() {
                if cell == b'E' && self.count < MAX_ENEMIES {
                    self.enemies[self.count] = Enemy {
                        x: column as i32,
                        y: row as i32,
                        hp: ENEMY_HP,
                        cooldown: 0,
                    };
                    self.count += 1;
                }
            }
        }
        self.state = State::Ready;
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn health(&self) -> i16 {
        self.health.max(0)
    }

    pub fn kills(&self) -> u8 {
        self.kills
    }

    /// 100 a demon, and for clearing the level, a bonus for the health left.
    pub fn score(&self) -> u32 {
        let bonus = if self.state == State::Won {
            500 + self.health.max(0) as u32 * 5
        } else {
            0
        };
        u32::from(self.kills) * 100 + bonus
    }

    pub fn ammo(&self) -> u8 {
        self.ammo
    }

    pub fn reloading(&self) -> bool {
        self.reload > 0
    }

    pub fn total(&self) -> u8 {
        self.count as u8
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

    /// Turn clockwise (`true`) or anticlockwise.
    pub fn turn(&mut self, clockwise: bool) {
        if self.state != State::Playing {
            return;
        }
        self.flash = 0;
        self.angle = if clockwise {
            self.angle.wrapping_add(TURN)
        } else {
            self.angle.wrapping_sub(TURN)
        };
    }

    /// One step forwards or backwards, sliding along walls.
    pub fn walk(&mut self, forwards: bool) -> bool {
        if self.state != State::Playing {
            return false;
        }
        self.flash = 0;
        let sign = if forwards { 1 } else { -1 };
        let dx = cos(self.angle) * STEP / 256 * sign;
        let dy = sin(self.angle) * STEP / 256 * sign;
        let mut moved = false;
        if self.free(self.x + dx, self.y) {
            self.x += dx;
            moved = true;
        }
        if self.free(self.x, self.y + dy) {
            self.y += dy;
            moved = true;
        }
        moved
    }

    /// Whether the player fits at this spot: clear of walls and of demons.
    fn free(&self, x: i32, y: i32) -> bool {
        for (dx, dy) in [
            (-MARGIN, -MARGIN),
            (MARGIN, -MARGIN),
            (-MARGIN, MARGIN),
            (MARGIN, MARGIN),
        ] {
            if is_wall((x + dx) >> 8, (y + dy) >> 8) {
                return false;
            }
        }
        !self.enemies[..self.count].iter().any(|enemy| {
            let (ex, ey) = enemy.centre();
            enemy.alive() && (ex - x).abs() < 128 && (ey - y).abs() < 128
        })
    }

    /// Shoot straight ahead.  With the magazine empty, this starts a reload instead.  Returns
    /// whether something was hit.
    pub fn fire(&mut self) -> bool {
        if self.state != State::Playing || self.reload > 0 {
            return false;
        }
        if self.ammo == 0 {
            self.reload = RELOAD_TICKS;
            return false;
        }
        self.ammo -= 1;
        self.flash = 2;
        let wall = self.cast(64).distance >> 8;
        let mut target: Option<(usize, i32)> = None;
        for (index, enemy) in self.enemies[..self.count].iter().enumerate() {
            if !enemy.alive() {
                continue;
            }
            let (depth, lateral) = self.relative(enemy.centre());
            if depth > 0
                && i64::from(depth) < wall
                && lateral.abs() < 90
                && target.is_none_or(|(_, best)| depth < best)
            {
                target = Some((index, depth));
            }
        }
        let Some((index, _)) = target else {
            return false;
        };
        self.enemies[index].hp -= 1;
        if !self.enemies[index].alive() {
            self.kills += 1;
            if usize::from(self.kills) == self.count {
                self.state = State::Won;
            }
        }
        true
    }

    /// Where a point is relative to the camera: (distance ahead, distance to the right).
    fn relative(&self, (x, y): (i32, i32)) -> (i32, i32) {
        let (dx, dy) = (x - self.x, y - self.y);
        let (fx, fy) = (cos(self.angle), sin(self.angle));
        ((dx * fx + dy * fy) >> 8, (-dx * fy + dy * fx) >> 8)
    }

    /// The demons move and bite.
    pub fn tick(&mut self) -> bool {
        if self.state != State::Playing {
            return false;
        }
        self.ticks += 1;
        self.flash = self.flash.saturating_sub(1);
        self.hurt = 0;
        if self.reload > 0 {
            self.reload -= 1;
            if self.reload == 0 {
                self.ammo = MAGAZINE;
            }
        }
        let (px, py) = (self.x >> 8, self.y >> 8);
        let map = self.distances();
        for index in 0..self.count {
            let enemy = self.enemies[index];
            if !enemy.alive() {
                continue;
            }
            let (dx, dy) = (px - enemy.x, py - enemy.y);
            let close = dx.abs().max(dy.abs());
            self.enemies[index].cooldown = enemy.cooldown.saturating_sub(1);
            if close <= 1 {
                if enemy.cooldown == 0 && self.rng.below(3) != 0 {
                    self.health -= 8;
                    self.hurt = 1;
                    self.enemies[index].cooldown = 3;
                }
            } else if (self.ticks + index as u32) % 2 == 0 {
                self.advance(index, &map);
            }
        }
        if self.health <= 0 {
            self.state = State::Dead;
        }
        true
    }

    /// Steps from each cell to the player's, around walls; 255 where there is no way.
    fn distances(&self) -> [[u8; MAP_SIZE]; MAP_SIZE] {
        let mut map = [[255_u8; MAP_SIZE]; MAP_SIZE];
        let mut queue = [(0_u8, 0_u8); MAP_SIZE * MAP_SIZE];
        let (mut head, mut tail) = (0, 1);
        let (px, py) = ((self.x >> 8) as usize, (self.y >> 8) as usize);
        map[py][px] = 0;
        queue[0] = (px as u8, py as u8);
        while head < tail {
            let (x, y) = queue[head];
            head += 1;
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (i32::from(x) + dx, i32::from(y) + dy);
                if is_wall(nx, ny) || map[ny as usize][nx as usize] != 255 {
                    continue;
                }
                map[ny as usize][nx as usize] = map[y as usize][x as usize] + 1;
                queue[tail] = (nx as u8, ny as u8);
                tail += 1;
            }
        }
        map
    }

    /// Move a demon a cell along the shortest way to the player.
    fn advance(&mut self, index: usize, map: &[[u8; MAP_SIZE]; MAP_SIZE]) {
        let me = self.enemies[index];
        let mut best = map[me.y as usize][me.x as usize];
        let mut target = None;
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (x, y) = (me.x + dx, me.y + dy);
            if is_wall(x, y) || map[y as usize][x as usize] >= best {
                continue;
            }
            let taken = self.enemies[..self.count]
                .iter()
                .any(|other| other.alive() && other.x == x && other.y == y);
            if !taken && (x, y) != (self.x >> 8, self.y >> 8) {
                best = map[y as usize][x as usize];
                target = Some((x, y));
            }
        }
        if let Some((x, y)) = target {
            self.enemies[index].x = x;
            self.enemies[index].y = y;
        }
    }

    /// Cast the ray through screen column `column`, 0..128.
    fn cast(&self, column: i32) -> Hit {
        let (dx, dy) = (cos(self.angle), sin(self.angle));
        let (plane_x, plane_y) = (-dy * PLANE / 256, dx * PLANE / 256);
        let camera = 4 * column - 256;
        let ray_x = dx + plane_x * camera / 256;
        let ray_y = dy + plane_y * camera / 256;

        let far = i64::from(i32::MAX / 4);
        let delta_x = if ray_x == 0 {
            far
        } else {
            (1 << 24) / i64::from(ray_x.abs())
        };
        let delta_y = if ray_y == 0 {
            far
        } else {
            (1 << 24) / i64::from(ray_y.abs())
        };
        let (mut cell_x, mut cell_y) = (self.x >> 8, self.y >> 8);
        let (frac_x, frac_y) = (i64::from(self.x & 255), i64::from(self.y & 255));
        let (step_x, mut side_x) = if ray_x < 0 {
            (-1, frac_x * delta_x >> 8)
        } else {
            (1, (256 - frac_x) * delta_x >> 8)
        };
        let (step_y, mut side_y) = if ray_y < 0 {
            (-1, frac_y * delta_y >> 8)
        } else {
            (1, (256 - frac_y) * delta_y >> 8)
        };

        let mut side = false;
        for _ in 0..MAP_SIZE * 2 {
            if side_x < side_y {
                side_x += delta_x;
                cell_x += step_x;
                side = true;
            } else {
                side_y += delta_y;
                cell_y += step_y;
                side = false;
            }
            if is_wall(cell_x, cell_y) {
                break;
            }
        }
        let distance = if side {
            side_x - delta_x
        } else {
            side_y - delta_y
        };
        Hit {
            distance: distance.max(1 << 12),
            side,
        }
    }

    /// Draw what the player sees.
    pub fn render(&self, frame: &mut Frame) {
        let mut depth = [0_i64; WIDTH];
        let mid = HEIGHT as i32 / 2;

        for column in 0..WIDTH as i32 {
            let hit = self.cast(column);
            depth[column as usize] = hit.distance;
            let height = (SCALE << 16) / hit.distance;
            let top = mid - (height / 2) as i32;
            let level = match hit.distance {
                0..98_304 => 0,
                98_304..196_608 => 1,
                196_608..327_680 => 2,
                _ => 3,
            };
            let level = (level + u8::from(hit.side)).min(3);
            for row in 0..HEIGHT as i32 {
                let on = if row >= top && row < top + height as i32 {
                    shade_on(level, column, row)
                } else {
                    // a dotted floor
                    row > mid + 3 && column & 3 == 0 && row & 3 == 0
                };
                frame.put(column, row, on);
            }
        }

        // far demons first, so near ones cover them
        let mut order = [0_usize; MAX_ENEMIES];
        for (slot, value) in order.iter_mut().enumerate() {
            *value = slot;
        }
        let count = self.count;
        order[..count].sort_unstable_by_key(|&i| -self.relative(self.enemies[i].centre()).0);
        for &index in &order[..count] {
            let enemy = self.enemies[index];
            if enemy.alive() {
                self.draw_demon(frame, &depth, &enemy);
            }
        }

        // the pistol, lowered and raised again while reloading
        let drop = match (self.reload, RELOAD_TICKS) {
            (0, _) => 0,
            (r, total) if r == total || r == 1 => 5,
            _ => 10,
        };
        frame.rect(58, 25 + drop, 12, 7, false);
        frame.rect(61, 22 + drop, 6, 7, true);
        frame.rect(59, 28 + drop, 10, 4, true);
        if self.flash > 0 {
            frame.rect(60, 16, 8, 6, false);
            for (x, y, w, h) in [(63, 14, 2, 8), (60, 17, 8, 2), (61, 15, 6, 6)] {
                frame.rect(x, y, w, h, true);
            }
        }

        // the magazine: a bar for each round, hollow once spent
        frame.rect(100, 25, 28, 7, false);
        for round in 0..MAGAZINE {
            let x = 102 + i32::from(round) * 4;
            frame.rect(x, 26, 3, 6, true);
            if round >= self.ammo {
                frame.rect(x + 1, 27, 1, 4, false);
            }
        }
        if self.hurt > 0 {
            frame.invert();
        }
    }

    fn draw_demon(&self, frame: &mut Frame, depth: &[i64; WIDTH], enemy: &Enemy) {
        let (ahead, right) = self.relative(enemy.centre());
        if ahead < 40 {
            return;
        }
        let ahead = i64::from(ahead);
        let centre = 64 + (64 * i64::from(right) * 256 / (ahead * i64::from(PLANE))) as i32;
        let size = (SCALE * 256 / ahead) as i32;
        let (left, top) = (centre - size / 2, HEIGHT as i32 / 2 - size / 2);
        for column in left.max(0)..(left + size).min(WIDTH as i32) {
            if depth[column as usize] <= ahead << 8 {
                continue;
            }
            let u = ((column - left) * 8 / size) as usize;
            for row in top.max(0)..(top + size).min(HEIGHT as i32) {
                let v = ((row - top) * 8 / size) as usize;
                if DEMON_MASK[v] & (0x80 >> u) != 0 {
                    frame.put(column, row, DEMON_INK[v] & (0x80 >> u) != 0);
                }
            }
        }
    }

    #[cfg(test)]
    pub fn place(&mut self, x: i32, y: i32, angle: u8) {
        self.x = x;
        self.y = y;
        self.angle = angle;
    }

    #[cfg(test)]
    pub fn put_enemy(&mut self, index: usize, x: i32, y: i32) {
        self.enemies[index].x = x;
        self.enemies[index].y = y;
    }

    #[cfg(test)]
    pub fn enemy_cell(&self, index: usize) -> (i32, i32) {
        (self.enemies[index].x, self.enemies[index].y)
    }

    #[cfg(test)]
    pub fn play(&mut self) {
        self.state = State::Playing;
    }

    #[cfg(test)]
    pub fn position(&self) -> (i32, i32, u8) {
        (self.x, self.y, self.angle)
    }

    #[cfg(test)]
    pub fn map_row(row: usize) -> &'static [u8; MAP_SIZE] {
        MAP[row]
    }
}
