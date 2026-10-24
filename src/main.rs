//! Template

#![no_std]
#![no_main]

mod backlight;
mod debounce;
mod discrete;
mod game;
mod generated;
mod hardware;
mod record;
mod screens;
mod sections;
mod store;
mod ui;

use crate::hardware::Buttons;
use crate::sections::{Dashboard, Identity, StatusLine, Wordmark};
use crate::ui::konami::Konami;
use crate::ui::menu::{Item, Menu};
use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::{
    clocks::ClockConfig,
    gpio::{self, Output},
    pwm::{self, Pwm},
};
use embassy_time::{Duration, Timer};
use rand::{RngExt, SeedableRng, distr::Uniform};

use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let peripherals = embassy_rp::init(embassy_rp::config::Config::new(ClockConfig::crystal(
        12_000_000,
    )));

    // before spawning anything, since this briefly takes over the flash
    let mut flash_device = hardware::init_flash(peripherals.FLASH);
    let unique_id = hardware::unique_id(&mut flash_device);
    store::init(flash_device);

    spawner.spawn(led(Output::new(peripherals.PIN_25, gpio::Level::Low)).unwrap());
    info!("Hello World!");

    let mut display = hardware::init_display(
        peripherals.SPI0,
        peripherals.PIN_1,
        peripherals.PIN_3,
        peripherals.PIN_2,
        peripherals.PIN_5,
        peripherals.PIN_7,
    );

    let mut buttons = Buttons::new(
        peripherals.PIN_13,
        peripherals.PIN_12,
        peripherals.PIN_11,
        peripherals.PIN_10,
        peripherals.PIN_9,
    );

    let backlight_config = {
        let mut config = pwm::Config::default();
        config.invert_a = true;
        config.enable = true;
        config.compare_a = 0;
        config.top = 256;
        config.divider = 28.into();
        config
    };
    let backlight_red = Pwm::new_output_a(
        peripherals.PWM_SLICE2,
        peripherals.PIN_4,
        backlight_config.clone(),
    )
    .split()
    .0
    .unwrap();
    let backlight_green = Pwm::new_output_a(
        peripherals.PWM_SLICE3,
        peripherals.PIN_6,
        backlight_config.clone(),
    )
    .split()
    .0
    .unwrap();
    let backlight_blue = Pwm::new_output_a(
        peripherals.PWM_SLICE4,
        peripherals.PIN_8,
        backlight_config.clone(),
    )
    .split()
    .0
    .unwrap();

    let led_config = {
        let mut config = backlight_config.clone();
        config.invert_a = false;

        // I know you can change these values.  Modern LEDs are *extremely* bright, so please don't
        // be a dick.
        config.top = 256 * 28;
        config.divider = 1.into();
        config
    };

    let (led_bottom, led_middle) = {
        let (a, b) = Pwm::new_output_ab(
            peripherals.PWM_SLICE0,
            peripherals.PIN_16,
            peripherals.PIN_17,
            led_config.clone(),
        )
        .split();
        (a.unwrap(), b.unwrap())
    };
    let led_top = {
        Pwm::new_output_a(peripherals.PWM_SLICE1, peripherals.PIN_18, led_config)
            .split()
            .0
            .unwrap()
    };

    spawner.spawn(backlight::run([backlight_red, backlight_green, backlight_blue]).unwrap());

    spawner.spawn(discrete::run([led_top, led_middle, led_bottom]).unwrap());

    backlight::set(backlight::DEFAULT);
    discrete::set(discrete::Mode::Random);

    let mut identity = Identity::new();
    let mut wordmark = Wordmark::new();
    let mut status_line = StatusLine::new();
    let dashboard = Dashboard::new([&mut identity, &mut wordmark, &mut status_line]);

    let mut files = screens::Files::new();
    let mut links = screens::Links::new();
    let mut led_colors = screens::LedColors::new();
    let mut leds = screens::Leds::new();

    let mut snake_game = screens::Snake::new(unique_id);
    let mut flappy = screens::Flappy::new(unique_id);
    let mut crossy = screens::Crossy::new(unique_id);
    let mut tetris = screens::Tetris::new(unique_id);
    let mut games = Menu::with_header(
        "◀◀◀ quit   ● play",
        [
            Item::new("Snake", &mut snake_game),
            Item::new("Flappy", &mut flappy),
            Item::new("Crossy", &mut crossy),
            Item::new("Tetris", &mut tetris),
        ],
    );

    let mut about = screens::About::new();
    let mut status = screens::Status::new(
        hardware::Battery::new(peripherals.ADC, peripherals.PIN_29),
        unique_id,
    );
    let mut bootloader = screens::Bootloader;
    let menu = Menu::new([
        Item::new("Files", &mut files),
        Item::new("Links", &mut links),
        Item::new("Games", &mut games),
        Item::new("Colors", &mut led_colors),
        Item::new("LEDs", &mut leds),
        Item::new("About", &mut about),
        Item::new("Status", &mut status),
        Item::new("Bootloader", &mut bootloader),
    ]);

    let mut home = screens::Home::new(dashboard, menu);
    let mut root = Konami::new(&mut home, screens::Snake::new(unique_id));

    ui::run(&mut display, &mut buttons, &mut root).await
}

#[embassy_executor::task]
async fn led(mut led: Output<'static>) {
    let mut rng = rand::rngs::SmallRng::from_seed([42; _]);
    let distribution = Uniform::new(1, 500).expect("low < high");
    let mut wait = || {
        let ms = rng.sample(distribution);
        async move {
            Timer::after(Duration::from_millis(ms)).await;
        }
    };

    loop {
        wait().await;
        led.set_high();
        wait().await;
        led.set_low();
    }
}
