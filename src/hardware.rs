use embassy_futures::select::select_array;
use embassy_rp::{
    Peri, adc, flash,
    gpio::{self, Input, Output},
    peripherals,
    spi::{self, Spi},
};
use st7565::{GraphicsPageBuffer, ST7565};
use static_cell::StaticCell;

pub type DisplayCS = peripherals::PIN_1;
pub type DisplayTX = peripherals::PIN_3;
pub type DisplayClk = peripherals::PIN_2;
pub type DisplayA0 = peripherals::PIN_5;
pub type DisplayReset = peripherals::PIN_7;
pub type DisplaySPI = peripherals::SPI0;
pub type BatterySense = peripherals::PIN_29;
pub type ButtonUp = peripherals::PIN_13;
pub type ButtonLeft = peripherals::PIN_12;
pub type ButtonCenter = peripherals::PIN_11;
pub type ButtonRight = peripherals::PIN_10;
pub type ButtonDown = peripherals::PIN_9;

pub const DISPLAY_WIDTH: u32 = 128;
pub const DISPLAY_HEIGHT: u32 = 32;

pub struct BadgeDisplay;

impl st7565::DisplaySpecs<128, 32, 4> for BadgeDisplay {
    const FLIP_ROWS: bool = true;

    const FLIP_COLUMNS: bool = false;

    const INVERTED: bool = false;

    const BIAS_MODE_1: bool = false;

    const POWER_CONTROL: st7565::types::PowerControlMode = st7565::types::PowerControlMode {
        booster_circuit: true,
        voltage_regulator_circuit: true,
        voltage_follower_circuit: true,
    };

    const VOLTAGE_REGULATOR_RESISTOR_RATIO: u8 = 1;

    const ELECTRONIC_VOLUME: u8 = 0x20;

    const BOOSTER_RATIO: st7565::types::BoosterRatio = st7565::types::BoosterRatio::StepUp2x3x4x;

    const COLUMN_OFFSET: u8 = 0;
}

// This type is a nested nightmare; its real type was carefully obtained by ... temporarily using
// `()` and then copy/pasting out of the error message.  You might need to do that again if
// something changes about initialization.
pub type DrawTarget = ST7565<
    display_interface_spi::SPIInterface<
        embedded_hal_bus::spi::ExclusiveDevice<
            embassy_rp::spi::Spi<'static, DisplaySPI, spi::Blocking>,
            Output<'static>,
            embedded_hal_bus::spi::NoDelay,
        >,
        Output<'static>,
    >,
    BadgeDisplay,
    st7565::modes::GraphicsMode<'static, 128, 4>,
    128,
    32,
    4,
>;

pub fn init_display(
    spi: Peri<'static, DisplaySPI>,
    cs: Peri<'static, DisplayCS>,
    tx: Peri<'static, DisplayTX>,
    clk: Peri<'static, DisplayClk>,
    a0: Peri<'static, DisplayA0>,
    rst: Peri<'static, DisplayReset>,
) -> DrawTarget {
    let cs = Output::new(cs, gpio::Level::Low);
    let a0 = Output::new(a0, gpio::Level::Low);
    let mut rst = Output::new(rst, gpio::Level::Low);

    let spi = Spi::new_blocking_txonly(spi, clk, tx, {
        let mut config = spi::Config::default();
        config.frequency = 500_000;
        config
    });

    let spi = embedded_hal_bus::spi::ExclusiveDevice::new_no_delay(spi, cs).unwrap();
    let spi = display_interface_spi::SPIInterface::new(spi, a0);
    let mut display = st7565::ST7565::new(spi, BadgeDisplay);

    display.reset(&mut rst, &mut embassy_time::Delay).unwrap();
    // don't run the Drop implementation on the Output, which (via `impl Drop for Flex`) resets the
    // GPIO pin to high-Z, which will in turn accidentally hold the display in reset.
    core::mem::forget(rst);

    let buffer = {
        static BUFFER: StaticCell<GraphicsPageBuffer<128, 4>> = StaticCell::new();
        // PANIC SAFETY: this function takes specific pins by move, so it's impossible to call
        // init_display() twice.
        BUFFER.init_with(GraphicsPageBuffer::new)
    };
    display.into_graphics_mode(buffer)
}

pub struct Buttons {
    pub up: crate::debounce::Button,
    pub left: crate::debounce::Button,
    pub center: crate::debounce::Button,
    pub right: crate::debounce::Button,
    pub down: crate::debounce::Button,
}

impl Buttons {
    pub fn new(
        up: Peri<'static, ButtonUp>,
        left: Peri<'static, ButtonLeft>,
        center: Peri<'static, ButtonCenter>,
        right: Peri<'static, ButtonRight>,
        down: Peri<'static, ButtonDown>,
    ) -> Buttons {
        Self {
            up: crate::debounce::Button::new(Input::new(up, gpio::Pull::Up)),
            left: crate::debounce::Button::new(Input::new(left, gpio::Pull::Up)),
            center: crate::debounce::Button::new(Input::new(center, gpio::Pull::Up)),
            right: crate::debounce::Button::new(Input::new(right, gpio::Pull::Up)),
            down: crate::debounce::Button::new(Input::new(down, gpio::Pull::Up)),
        }
    }
}

impl Buttons {
    /// Wait for any button to be pressed.
    pub async fn next_press(&mut self) -> crate::ui::Button {
        use crate::ui::Button;
        let (_, i) = select_array([
            self.up.wait_for_pressed(),
            self.down.wait_for_pressed(),
            self.left.wait_for_pressed(),
            self.right.wait_for_pressed(),
            self.center.wait_for_pressed(),
        ])
        .await;
        [
            Button::Up,
            Button::Down,
            Button::Left,
            Button::Right,
            Button::Center,
        ][i]
    }
}

/// The 3xAAA battery voltage, read through a divider on [`BatterySense`].
pub struct Battery {
    adc: adc::Adc<'static, adc::Blocking>,
    channel: adc::Channel<'static>,
}

impl Battery {
    /// battery voltage / voltage at the pin
    const DIVIDER: u32 = 3;
    /// The ADC's reference, which is the 3.3V rail.
    const REFERENCE_MILLIVOLTS: u32 = 3300;
    /// Readings averaged together, to smooth out noise.
    const SAMPLES: u32 = 16;

    pub fn new(adc: Peri<'static, peripherals::ADC>, pin: Peri<'static, BatterySense>) -> Self {
        Self {
            adc: adc::Adc::new_blocking(adc, adc::Config::default()),
            channel: adc::Channel::new_pin(pin, gpio::Pull::None),
        }
    }

    pub fn millivolts(&mut self) -> u32 {
        let total: u32 = (0..Self::SAMPLES)
            .map(|_| u32::from(self.adc.blocking_read(&mut self.channel).unwrap()))
            .sum();
        // 12-bit readings, so full scale is 4096
        total * Self::REFERENCE_MILLIVOLTS * Self::DIVIDER / (4096 * Self::SAMPLES)
    }
}

/// The RP2040 has no unique ID of its own, so use the flash chip's 64-bit one, as the Pico SDK's
/// `pico_unique_board_id` does.
///
/// This briefly runs from RAM with interrupts off and flash unavailable, so call it before
/// anything else is running.
pub type FlashDevice =
    flash::Flash<'static, peripherals::FLASH, flash::Blocking, { 2 * 1024 * 1024 }>;

pub fn init_flash(flash: Peri<'static, peripherals::FLASH>) -> FlashDevice {
    flash::Flash::new_blocking(flash)
}

pub fn unique_id(flash: &mut FlashDevice) -> u64 {
    let mut id = [0; 8];
    flash.blocking_unique_id(&mut id).unwrap();
    u64::from_be_bytes(id)
}
