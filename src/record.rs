//! The settings record that lives in flash, with no hardware or allocator in it.

/// The badge's usual backlight color.
pub const DEFAULT_MAIN: [u8; 3] = [80, 32, 120];
/// The usual color it blinks on a button press.
pub const DEFAULT_FLASH: [u8; 3] = [140, 0, 0];
/// The usual brightness of the three discrete LEDs.
pub const DEFAULT_LEDS: [u8; 3] = [0x50, 0x50, 0x50];

pub const MAGIC: u32 = 0x4241_4447;
/// 2 adds the LED mode and brightnesses; 1 is still readable.
pub const VERSION: u8 = 2;
pub const NOTE_LEN: usize = 128;

/// magic, version, rainbow, two colors, LED mode, three LED levels, note length, note, checksum
const HEADER: usize = 17;
pub const RECORD_MAX: usize = HEADER + 4 + NOTE_LEN;

#[derive(Clone)]
pub struct Data {
    pub main: [u8; 3],
    pub flash: [u8; 3],
    pub rainbow: bool,
    /// index into the LEDs screen's modes: random, solid, chase, button
    pub led_mode: u8,
    pub led_levels: [u8; 3],
    note: [u8; NOTE_LEN],
    note_len: u8,
}

pub const fn defaults() -> Data {
    Data {
        main: DEFAULT_MAIN,
        flash: DEFAULT_FLASH,
        rainbow: false,
        led_mode: 0,
        led_levels: DEFAULT_LEDS,
        note: [0; NOTE_LEN],
        note_len: 0,
    }
}

impl Data {
    pub fn note(&self) -> &str {
        core::str::from_utf8(&self.note[..self.note_len as usize]).unwrap_or("")
    }

    pub fn set_note(&mut self, text: &str) {
        let bytes = text.as_bytes();
        let mut len = bytes.len().min(NOTE_LEN);
        while !text.is_char_boundary(len) {
            len -= 1;
        }
        self.note[..len].copy_from_slice(&bytes[..len]);
        self.note_len = len as u8;
    }
}

pub fn encode(data: &Data, buffer: &mut [u8; RECORD_MAX]) -> usize {
    let len = data.note_len as usize;

    buffer[0..4].copy_from_slice(&MAGIC.to_le_bytes());
    buffer[4] = VERSION;
    buffer[5] = u8::from(data.rainbow);
    buffer[6..9].copy_from_slice(&data.main);
    buffer[9..12].copy_from_slice(&data.flash);
    buffer[12] = data.led_mode;
    buffer[13..16].copy_from_slice(&data.led_levels);
    buffer[16] = len as u8;
    buffer[HEADER..HEADER + len].copy_from_slice(&data.note[..len]);

    let checksum = checksum(&buffer[..HEADER + len]);
    buffer[HEADER + len..HEADER + 4 + len].copy_from_slice(&checksum.to_le_bytes());

    HEADER + 4 + len
}

pub fn decode(buffer: &[u8]) -> Option<Data> {
    if buffer.len() < 5 || u32::from_le_bytes(buffer[0..4].try_into().ok()?) != MAGIC {
        return None;
    }

    match buffer[4] {
        1 => decode_v1(buffer),
        2 => decode_v2(buffer),
        _ => None,
    }
}

fn decode_v2(buffer: &[u8]) -> Option<Data> {
    if buffer.len() < HEADER {
        return None;
    }
    let len = buffer[16] as usize;
    if len > NOTE_LEN || HEADER + 4 + len > buffer.len() {
        return None;
    }
    if stored_checksum(buffer, HEADER, len) != checksum(&buffer[..HEADER + len]) {
        return None;
    }

    let mut data = defaults();
    data.main = [buffer[6], buffer[7], buffer[8]];
    data.flash = [buffer[9], buffer[10], buffer[11]];
    data.rainbow = buffer[5] != 0;
    data.led_mode = buffer[12];
    data.led_levels = [buffer[13], buffer[14], buffer[15]];
    data.set_note(core::str::from_utf8(&buffer[HEADER..HEADER + len]).unwrap_or(""));
    Some(data)
}

/// The first layout, which had no LED settings.
fn decode_v1(buffer: &[u8]) -> Option<Data> {
    const V1_HEADER: usize = 13;
    if buffer.len() < V1_HEADER {
        return None;
    }
    let len = buffer[12] as usize;
    if len > NOTE_LEN || V1_HEADER + 4 + len > buffer.len() {
        return None;
    }
    if stored_checksum(buffer, V1_HEADER, len) != checksum(&buffer[..V1_HEADER + len]) {
        return None;
    }

    let mut data = defaults();
    data.main = [buffer[6], buffer[7], buffer[8]];
    data.flash = [buffer[9], buffer[10], buffer[11]];
    data.rainbow = buffer[5] != 0;
    data.set_note(core::str::from_utf8(&buffer[V1_HEADER..V1_HEADER + len]).unwrap_or(""));
    Some(data)
}

fn checksum(bytes: &[u8]) -> u32 {
    bytes
        .iter()
        .fold(0u32, |sum, &byte| sum.wrapping_add(u32::from(byte)))
}

fn stored_checksum(buffer: &[u8], header: usize, len: usize) -> u32 {
    u32::from_le_bytes(
        buffer[header + len..header + 4 + len]
            .try_into()
            .expect("length checked by the caller"),
    )
}
