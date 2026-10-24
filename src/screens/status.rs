use core::fmt::Write as _;

use embassy_time::{Duration, Instant};
use embedded_graphics::{Drawable, geometry::Point, text::Text};

use crate::hardware::Battery;
use crate::ui::{self, Button, Canvas, Response, Screen};

const REFRESH_INTERVAL: Duration = Duration::from_millis(500);

/// Battery voltage, uptime, and the badge's unique ID, kept up to date; any button exits.
pub struct Status {
    battery: Battery,
    /// the last battery reading
    battery_millivolts: u32,
    unique_id: u64,
}

impl Status {
    pub fn new(battery: Battery, unique_id: u64) -> Self {
        Self {
            battery,
            battery_millivolts: 0,
            unique_id,
        }
    }
}

impl Screen for Status {
    fn enter(&mut self) {
        self.battery_millivolts = self.battery.millivolts();
    }

    fn draw(&self, canvas: &mut Canvas) {
        let mut battery = heapless::String::<21>::new();
        let mut uptime = heapless::String::<21>::new();
        let mut id = heapless::String::<21>::new();

        let mv = self.battery_millivolts;
        core::write!(battery, "Pwr: {}.{:02}V", mv / 1000, mv % 1000 / 10).unwrap();

        let secs = Instant::now().as_secs();
        core::write!(
            uptime,
            "Up:  {}d {:02}:{:02}:{:02}",
            secs / 86400,
            secs / 3600 % 24,
            secs / 60 % 60,
            secs % 60,
        )
        .unwrap();

        core::write!(id, "ID: {:016X}", self.unique_id).unwrap();

        for (i, line) in [battery, uptime, id].iter().enumerate() {
            let y = i as i32 * (ui::FONT.character_size.height as i32 + 1);
            Text::with_text_style(line, Point::new(0, y), ui::TEXT, ui::TOP_LEFT)
                .draw(canvas)
                .unwrap();
        }
    }

    fn handle(&mut self, _: Button) -> Response {
        Response::Exit
    }

    fn tick_interval(&self) -> Option<Duration> {
        Some(REFRESH_INTERVAL)
    }

    fn tick(&mut self) -> Response {
        self.battery_millivolts = self.battery.millivolts();
        Response::Redraw
    }
}
