use embedded_graphics::{Drawable, geometry::Point, text::Text};

use crate::discrete;
use crate::store;
use crate::ui::{self, Button, Canvas, Response, Screen};

const MODES: [&str; 4] = ["RANDOM", "SOLID", "CHASE", "BUTTON"];
/// Per-press step for the brightness sliders.
const STEP: u8 = 16;
/// Four fields: the mode, then the top, middle, and bottom LED.
const FIELDS: usize = 4;

/// The three discrete LEDs: a mode, and a brightness for each.  Left/Right picks a field, Up/Down
/// changes it, Center leaves.
pub struct Leds {
    selected: usize,
    mode: usize,
    levels: [u8; 3],
}

impl Leds {
    pub fn new() -> Self {
        let saved = store::data();
        Self {
            selected: 0,
            mode: usize::from(saved.led_mode).min(MODES.len() - 1),
            levels: saved.led_levels,
        }
    }

    fn apply(&self) {
        store::set_leds(self.mode as u8, self.levels);
        discrete::set(match self.mode {
            0 => discrete::Mode::Random,
            1 => discrete::Mode::Solid(self.levels),
            2 => discrete::Mode::Chase(self.levels),
            _ => discrete::Mode::Button(self.levels),
        });
    }

    fn step(&mut self, up: bool) {
        if self.selected == 0 {
            let count = MODES.len();
            self.mode = if up {
                (self.mode + count - 1) % count
            } else {
                (self.mode + 1) % count
            };
        } else {
            let channel = self.selected - 1;
            if up {
                self.levels[channel] = self.levels[channel].saturating_add(STEP);
            } else {
                self.levels[channel] = self.levels[channel].saturating_sub(STEP);
            }
        }
        self.apply();
    }
}

impl Screen for Leds {
    fn enter(&mut self) {
        self.apply();
    }

    fn draw(&self, canvas: &mut Canvas) {
        ui::label(canvas, 0, MODES[self.mode], 0, self.selected == 0);

        for (i, (label, value)) in [
            ("T", self.levels[0]),
            ("M", self.levels[1]),
            ("B", self.levels[2]),
        ]
        .into_iter()
        .enumerate()
        {
            ui::slider(
                canvas,
                label,
                value,
                6 + i as i32 * 6,
                self.selected == i + 1,
            );
        }

        Text::with_text_style(
            "◀▶ field ▲▼ change ● done",
            Point::new(0, 25),
            ui::SMALL_TEXT,
            ui::TOP_LEFT,
        )
        .draw(canvas)
        .unwrap();
    }

    fn handle(&mut self, button: Button) -> Response {
        match button {
            Button::Up => self.step(true),
            Button::Down => self.step(false),
            Button::Left => self.selected = (self.selected + FIELDS - 1) % FIELDS,
            Button::Right => self.selected = (self.selected + 1) % FIELDS,
            Button::Center => {
                store::save();
                return Response::Exit;
            }
        }
        Response::Redraw
    }
}
