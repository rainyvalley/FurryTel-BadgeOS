//! A tiny xorshift, so games are unpredictable between badges without a random number generator
//! crate.

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        // xorshift can't start from zero
        Self(seed | 1)
    }

    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    pub fn below(&mut self, limit: u8) -> u8 {
        if limit == 0 {
            return 0;
        }
        (self.next() % u64::from(limit)) as u8
    }

    /// A number in `low..high`.
    pub fn range(&mut self, low: i32, high: i32) -> i32 {
        if high <= low {
            return low;
        }
        low + (self.next() % (high - low) as u64) as i32
    }
}
