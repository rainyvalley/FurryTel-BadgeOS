use embedded_graphics::{Drawable, geometry::Point, text::Text};

use crate::ui::{self, Button, Canvas, Response, Screen};

/// The on-board LED, which the bootloader blinks on USB activity.
const ACTIVITY_LED_PIN: u32 = 25;

/// Reboots into the RP2040's USB bootloader (as if BOOTSEL were held) on Center; any other button
/// exits.
///
/// The screen goes blank in the bootloader: the reboot resets the GPIOs, whose default pull-downs
/// hold the display in reset.
pub struct Bootloader;

impl Bootloader {
    fn enter_bootloader(&self) {
        defmt::info!("rebooting into the USB bootloader");
        // with both the mass storage and PICOBOOT interfaces enabled, as on a cold BOOTSEL boot
        embassy_rp::rom_data::reset_to_usb_boot(1 << ACTIVITY_LED_PIN, 0);
    }
}

impl Screen for Bootloader {
    fn draw(&self, canvas: &mut Canvas) {
        let lines = [
            "Press ● to enter the",
            "bootloader, any other",
            "button to exit.",
        ];
        for (i, line) in lines.into_iter().enumerate() {
            let y = i as i32 * ui::FONT.character_size.height as i32;
            Text::with_text_style(line, Point::new(0, y), ui::TEXT, ui::TOP_LEFT)
                .draw(canvas)
                .unwrap();
        }
    }

    fn handle(&mut self, button: Button) -> Response {
        if button == Button::Center {
            self.enter_bootloader();
        }
        // the ROM doesn't return, but in case it ever does, go back to the menu
        Response::Exit
    }
}
