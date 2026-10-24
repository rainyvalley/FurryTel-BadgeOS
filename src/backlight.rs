//! The RGB backlight runs in its own task so effects like the rainbow keep going no matter what's
//! on screen.  Everything else just tells it what to do with [`set`], and [`flash`] blinks it red
//! for button feedback.

use embassy_futures::select::{Either4, select4};
use embassy_rp::pwm::{PwmOutput, SetDutyCycle};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, signal::Signal};
use embassy_time::{Duration, Instant, Timer};

#[derive(Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum Mode {
    /// red, green, blue
    Solid([u8; 3]),
    /// Cycle through every fully-saturated hue.
    Rainbow,
}

/// The badge's usual color: a purple wash, bright enough to read by and dim enough not to be rude.
pub const DEFAULT_RGB: [u8; 3] = crate::record::DEFAULT_MAIN;
pub const DEFAULT: Mode = Mode::Solid(DEFAULT_RGB);

static MODE: Signal<ThreadModeRawMutex, Mode> = Signal::new();
static FLASH: Signal<ThreadModeRawMutex, ()> = Signal::new();
static FLASH_RGB: Signal<ThreadModeRawMutex, [u8; 3]> = Signal::new();

pub fn set(mode: Mode) {
    MODE.signal(mode);
}

/// Blink the backlight, as feedback for a button press.
pub fn flash() {
    FLASH.signal(());
}

/// Set the color the backlight blinks when a button is pressed.
pub fn set_flash(rgb: [u8; 3]) {
    FLASH_RGB.signal(rgb);
}

/// How long a flash lasts.  Long enough to see, short enough not to slow anything down.
const FLASH_TIME: Duration = Duration::from_millis(50);
pub const DEFAULT_FLASH_RGB: [u8; 3] = crate::record::DEFAULT_FLASH;

const RAINBOW_FRAME: Duration = Duration::from_hz(60);
/// Hue is in 1/256ths of a sixth of the color wheel, so a full cycle is 6 * 256.
const HUE_CYCLE: u16 = 6 * 256;
/// Hue advanced per frame: a full cycle every 256 frames, about 4.3s.
const HUE_STEP: u16 = HUE_CYCLE / 256;
/// Nothing needs waking up while a solid color is showing, but an hour-long timer is simpler than a
/// third kind of wait.
const IDLE: Duration = Duration::from_secs(3600);

/// `pwms` is red, green, blue.
#[embassy_executor::task]
pub async fn run(mut pwms: [PwmOutput<'static>; 3]) -> ! {
    let mut mode = Mode::Solid([0; 3]);
    let mut hue = 0;
    let mut flash_until: Option<Instant> = None;
    let mut flash_rgb = DEFAULT_FLASH_RGB;

    loop {
        if flash_until.is_some_and(|end| end <= Instant::now()) {
            flash_until = None;
        }

        let rgb = match flash_until {
            Some(_) => flash_rgb,
            None => match mode {
                Mode::Solid(rgb) => rgb,
                Mode::Rainbow => {
                    let rgb = rainbow(hue);
                    hue = (hue + HUE_STEP) % HUE_CYCLE;
                    rgb
                }
            },
        };
        show(&mut pwms, rgb);

        let now = Instant::now();
        let frame_at = match (flash_until, mode) {
            (Some(end), _) => end,
            (None, Mode::Rainbow) => now + RAINBOW_FRAME,
            (None, Mode::Solid(_)) => now + IDLE,
        };

        match select4(
            MODE.wait(),
            FLASH.wait(),
            FLASH_RGB.wait(),
            Timer::at(frame_at),
        )
        .await
        {
            Either4::First(mode_) => mode = mode_,
            Either4::Second(()) => flash_until = Some(Instant::now() + FLASH_TIME),
            Either4::Third(rgb) => flash_rgb = rgb,
            Either4::Fourth(()) => {}
        }
    }
}

fn show(pwms: &mut [PwmOutput<'static>; 3], rgb: [u8; 3]) {
    for (pwm, value) in pwms.iter_mut().zip(rgb) {
        pwm.set_duty_cycle(value.into()).unwrap();
    }
}

/// The fully-saturated, full-brightness color at `hue` (out of [`HUE_CYCLE`]), per the "HSV to
/// RGB" section of Wikipedia's HSL and HSV article.
fn rainbow(hue: u16) -> [u8; 3] {
    let rising = (hue % 256) as u8;
    let falling = 255 - rising;
    match hue / 256 {
        0 => [255, rising, 0],
        1 => [falling, 255, 0],
        2 => [0, 255, rising],
        3 => [0, falling, 255],
        4 => [rising, 0, 255],
        _ => [255, 0, falling],
    }
}
