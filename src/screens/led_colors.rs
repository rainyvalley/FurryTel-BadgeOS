use embedded_graphics::{Drawable, geometry::Point, text::Text};

use crate::backlight;
use crate::store;
use crate::ui::{self, Button, Canvas, Response, Screen};

/// A named color, or the rainbow.
struct Preset {
    name: &'static str,
    rgb: Option<[u8; 3]>,
}

const PRESETS: &[Preset] = &[
    Preset {
        name: "PURPLE",
        rgb: Some(backlight::DEFAULT_RGB),
    },
    Preset {
        name: "WHITE",
        rgb: Some([110, 105, 95]),
    },
    Preset {
        name: "CYAN",
        rgb: Some([0, 110, 120]),
    },
    Preset {
        name: "LIME",
        rgb: Some([40, 120, 0]),
    },
    Preset {
        name: "AMBER",
        rgb: Some([140, 80, 0]),
    },
    Preset {
        name: "RED",
        rgb: Some([140, 0, 0]),
    },
    Preset {
        name: "BLUE",
        rgb: Some([0, 40, 140]),
    },
    Preset {
        name: "RAINBOW",
        rgb: None,
    },
];

/// The name shown when the sliders have been touched.
const CUSTOM: &str = "CUSTOM";
/// No preset is selected while the sliders are being used.
const NO_PRESET: usize = usize::MAX;
/// Index of the rainbow in [`PRESETS`].
const RAINBOW_PRESET: usize = PRESETS.len() - 1;
/// Per-press step for the sliders.
const STEP: u8 = 16;

/// The two colors the screen edits.
const TARGETS: [&str; 2] = ["MAIN", "FLASH"];

/// Six fields: target, preset, then red, green, and blue.
const FIELDS: usize = 5;

struct Light {
    rgb: [u8; 3],
    preset: usize,
    rainbow: bool,
}

impl Light {
    /// The preset that matches a saved color, if any.
    fn from_saved(rgb: [u8; 3], rainbow: bool) -> Self {
        if rainbow {
            return Self {
                rgb,
                preset: RAINBOW_PRESET,
                rainbow: true,
            };
        }
        let preset = (0..PRESETS.len())
            .find(|&i| PRESETS[i].rgb == Some(rgb))
            .unwrap_or(NO_PRESET);
        Self {
            rgb,
            preset,
            rainbow: false,
        }
    }

    fn name(&self) -> &'static str {
        if self.preset == NO_PRESET {
            CUSTOM
        } else {
            PRESETS[self.preset].name
        }
    }
}

/// Colors for the backlight: presets and sliders for the main color, and for the quick flash on
/// each button press.  Left/Right picks a field, Up/Down changes it, Center leaves.
pub struct LedColors {
    selected: usize,
    target: usize,
    lights: [Light; 2],
}

impl LedColors {
    pub fn new() -> Self {
        let saved = store::data();
        Self {
            selected: 0,
            target: 0,
            lights: [
                Light::from_saved(saved.main, saved.rainbow),
                Light::from_saved(saved.flash, false),
            ],
        }
    }

    fn light(&self) -> &Light {
        &self.lights[self.target]
    }

    fn apply(&self) {
        let main = &self.lights[0];
        backlight::set(if main.rainbow {
            backlight::Mode::Rainbow
        } else {
            backlight::Mode::Solid(main.rgb)
        });
        backlight::set_flash(self.lights[1].rgb);
        store::set_colors(main.rgb, self.lights[1].rgb, main.rainbow);
    }

    fn step(&mut self, up: bool) {
        match self.selected {
            0 => {
                self.target = (self.target + 1) % TARGETS.len();
            }
            1 => {
                let count = PRESETS.len();
                let mut preset = if self.light().preset == NO_PRESET {
                    0
                } else {
                    self.light().preset
                };
                for _ in 0..count {
                    preset = if up {
                        (preset + count - 1) % count
                    } else {
                        (preset + 1) % count
                    };
                    // a flash can't be a rainbow
                    if self.target == 0 || PRESETS[preset].rgb.is_some() {
                        break;
                    }
                }
                let rgb = PRESETS[preset].rgb;
                let target = self.target;
                let light = &mut self.lights[target];
                light.preset = preset;
                light.rainbow = rgb.is_none() && target == 0;
                if let Some(rgb) = rgb {
                    light.rgb = rgb;
                }
            }
            field => {
                let channel = field - 2;
                let target = self.target;
                let light = &mut self.lights[target];
                if up {
                    light.rgb[channel] = light.rgb[channel].saturating_add(STEP);
                } else {
                    light.rgb[channel] = light.rgb[channel].saturating_sub(STEP);
                }
                light.rainbow = false;
                light.preset = NO_PRESET;
            }
        }
        self.apply();
    }
}

impl Screen for LedColors {
    fn enter(&mut self) {
        self.apply();
    }

    fn draw(&self, canvas: &mut Canvas) {
        ui::label(canvas, 0, TARGETS[self.target], 0, self.selected == 0);
        ui::label(canvas, 40, self.light().name(), 0, self.selected == 1);

        let rgb = self.light().rgb;
        for (i, (label, value)) in [("R", rgb[0]), ("G", rgb[1]), ("B", rgb[2])]
            .into_iter()
            .enumerate()
        {
            ui::slider(
                canvas,
                label,
                value,
                6 + i as i32 * 6,
                self.selected == i + 2,
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
