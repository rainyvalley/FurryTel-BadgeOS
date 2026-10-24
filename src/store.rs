//! The settings record's home in flash: the last sector of the 2 MB chip.

use core::cell::RefCell;

use embassy_rp::{flash, peripherals};
use embassy_sync::{blocking_mutex::Mutex, blocking_mutex::raw::CriticalSectionRawMutex};

use crate::record::{self, Data, RECORD_MAX};

type FlashDevice = flash::Flash<'static, peripherals::FLASH, flash::Blocking, { 2 * 1024 * 1024 }>;

/// The last 4 KB of flash, far past the firmware.
const SECTOR: u32 = 2 * 1024 * 1024 - 4096;
const SECTOR_SIZE: u32 = 4096;

static DATA: Mutex<CriticalSectionRawMutex, RefCell<Data>> =
    Mutex::new(RefCell::new(record::defaults()));
static DEVICE: Mutex<CriticalSectionRawMutex, RefCell<Option<FlashDevice>>> =
    Mutex::new(RefCell::new(None));

/// Read the saved settings, and keep the flash for later saves.  Call this before building the UI.
pub fn init(mut device: FlashDevice) {
    let mut buffer = [0; RECORD_MAX];
    if device.blocking_read(SECTOR, &mut buffer).is_ok()
        && let Some(data) = record::decode(&buffer)
    {
        DATA.lock(|slot| *slot.borrow_mut() = data);
    }

    DEVICE.lock(|slot| *slot.borrow_mut() = Some(device));
}

pub fn data() -> Data {
    DATA.lock(|slot| slot.borrow().clone())
}

pub fn set_colors(main: [u8; 3], flash: [u8; 3], rainbow: bool) {
    DATA.lock(|slot| {
        let mut data = slot.borrow_mut();
        data.main = main;
        data.flash = flash;
        data.rainbow = rainbow;
    });
}

pub fn set_note(note: &str) {
    DATA.lock(|slot| slot.borrow_mut().set_note(note));
}

/// Write the current settings to flash.  This takes a few milliseconds and stops the world while
/// the flash is erased and programmed.
pub fn save() {
    let mut buffer = [0; RECORD_MAX];
    let len = DATA.lock(|slot| record::encode(&slot.borrow(), &mut buffer));

    DEVICE.lock(|slot| {
        if let Some(device) = slot.borrow_mut().as_mut()
            && device.blocking_erase(SECTOR, SECTOR + SECTOR_SIZE).is_ok()
        {
            let _ = device.blocking_write(SECTOR, &buffer[..len]);
        }
    });
}
