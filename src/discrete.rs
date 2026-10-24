//! The three discrete LEDs run in their own task so effects like random blinking keep going no
//! matter what's on screen.  Everything else tells it what to do with [`set`], and [`flash`] gives
//! them a blink on button presses.

use embassy_futures::select::{Either3, select3};
use embassy_rp::pwm::{PwmOutput, SetDutyCycle};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, signal::Signal};
use embassy_time::{Duration, Instant, Timer};
use rand::{RngExt, SeedableRng, distr::Uniform};

#[derive(Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum Mode {
    /// top, middle, bottom
    Solid([u8; 3]),
    /// Blink each LED on and off at random, like blinkenlights.
    Random,
    /// One LED at a time, top to bottom.
    Chase([u8; 3]),
    /// Dark until a button is pressed.
    Button([u8; 3]),
}

static MODE: Signal<ThreadModeRawMutex, Mode> = Signal::new();
static FLASH: Signal<ThreadModeRawMutex, ()> = Signal::new();

pub fn set(mode: Mode) {
    MODE.signal(mode);
}

/// Light the LEDs briefly, for button feedback in [`Mode::Button`].
pub fn flash() {
    FLASH.signal(());
}

/// Duty cycle of a lit LED while blinking.
// Yes you can change this value, but please don't be a dick and hurt other people's eyes.
const BLINK_BRIGHTNESS: u8 = 60;
/// Range of how long, in milliseconds, an LED stays on or off before flipping.
const BLINK_MIN_MS: u64 = 30;
const BLINK_MAX_MS: u64 = 400;
const CHASE_STEP: Duration = Duration::from_millis(180);
const FLASH_TIME: Duration = Duration::from_millis(150);
/// Nothing needs waking up while a solid color is showing.
const IDLE: Duration = Duration::from_secs(3600);

/// `pwms` is top, middle, bottom.
#[embassy_executor::task]
pub async fn run(mut pwms: [PwmOutput<'static>; 3]) -> ! {
    let mut rng = rand::rngs::SmallRng::from_seed([23; _]);
    let durations = Uniform::new_inclusive(BLINK_MIN_MS, BLINK_MAX_MS).expect("low <= high");

    let mut mode = Mode::Solid([0; 3]);
    let mut lit = [false; 3];
    let mut flip_at = [Instant::now(); 3];
    let mut chase = 0;
    let mut chase_at = Instant::now();
    let mut flash_until: Option<Instant> = None;

    loop {
        let now = Instant::now();
        if flash_until.is_some_and(|end| end <= now) {
            flash_until = None;
        }

        let (levels, deadline) = match mode {
            Mode::Solid(values) => (values, now + IDLE),
            Mode::Random => {
                for (on, at) in lit.iter_mut().zip(&mut flip_at) {
                    if *at <= now {
                        *on = !*on;
                        *at = now + Duration::from_millis(rng.sample(durations));
                    }
                }
                // PANIC SAFETY: there are always three LEDs
                let next = *flip_at.iter().min().unwrap();
                (lit.map(|on| if on { BLINK_BRIGHTNESS } else { 0 }), next)
            }
            Mode::Chase(values) => {
                if chase_at <= now {
                    chase = (chase + 1) % 3;
                    chase_at = now + CHASE_STEP;
                }
                let mut levels = [0; 3];
                levels[chase] = values[chase];
                (levels, chase_at)
            }
            Mode::Button(values) => (
                if flash_until.is_some() {
                    values
                } else {
                    [0; 3]
                },
                flash_until.unwrap_or(now + IDLE),
            ),
        };
        show(&mut pwms, levels);

        match select3(MODE.wait(), FLASH.wait(), Timer::at(deadline)).await {
            Either3::First(mode_) => mode = mode_,
            Either3::Second(()) => flash_until = Some(Instant::now() + FLASH_TIME),
            Either3::Third(()) => {}
        }
    }
}

fn show(pwms: &mut [PwmOutput<'static>; 3], values: [u8; 3]) {
    for (pwm, value) in pwms.iter_mut().zip(values) {
        pwm.set_duty_cycle(value.into()).unwrap();
    }
}
