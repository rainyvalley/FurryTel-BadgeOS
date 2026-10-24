//! The settings record that lives in flash, with no hardware or allocator in it.

/// The badge's usual backlight color.
pub const DEFAULT_MAIN: [u8; 3] = [80, 32, 120];
/// The usual color it blinks on a button press.
pub const DEFAULT_FLASH: [u8; 3] = [140, 0, 0];

pub const MAGIC: u32 = 0x4241_4447;
pub const VERSION: u8 = 1;
pub const NOTE_LEN: usize = 128;
/// Magic, version, rainbow, two colors, note length, note, checksum.
pub const RECORD_MAX: usize = 17 + NOTE_LEN;

#[derive(Clone)]
pub struct Data {
    pub main: [u8; 3],
    pub flash: [u8; 3],
    pub rainbow: bool,
    note: [u8; NOTE_LEN],
    note_len: u8,
}

pub const fn defaults() -> Data {
    Data {
        main: DEFAULT_MAIN,
        flash: DEFAULT_FLASH,
        rainbow: false,
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
    buffer[12] = len as u8;
    buffer[13..13 + len].copy_from_slice(&data.note[..len]);

    let checksum = buffer[..13 + len]
        .iter()
        .fold(0u32, |sum, &byte| sum.wrapping_add(u32::from(byte)));
    buffer[13 + len..17 + len].copy_from_slice(&checksum.to_le_bytes());

    17 + len
}

pub fn decode(buffer: &[u8]) -> Option<Data> {
    if buffer.len() < 13 {
        return None;
    }
    if u32::from_le_bytes(buffer[0..4].try_into().ok()?) != MAGIC || buffer[4] != VERSION {
        return None;
    }

    let len = buffer[12] as usize;
    if len > NOTE_LEN || 17 + len > buffer.len() {
        return None;
    }

    let checksum = u32::from_le_bytes(buffer[13 + len..17 + len].try_into().ok()?);
    let expected = buffer[..13 + len]
        .iter()
        .fold(0u32, |sum, &byte| sum.wrapping_add(u32::from(byte)));
    if checksum != expected {
        return None;
    }

    // a note that is not valid UTF-8 is dropped, but the colors are still worth keeping
    let note = core::str::from_utf8(&buffer[13..13 + len]).unwrap_or("");
    let mut data = defaults();
    data.main = [buffer[6], buffer[7], buffer[8]];
    data.flash = [buffer[9], buffer[10], buffer[11]];
    data.rainbow = buffer[5] != 0;
    data.set_note(note);
    Some(data)
}
