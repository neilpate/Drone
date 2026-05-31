//! BBC micro:bit v2 (nRF52833) board support.
//!
//! Phase 1–3 development target. Sensors and actuators are breadboarded onto
//! the edge connector; only the on-board 5x5 LED matrix is used directly.
//!
//! See [ADR 0010](../../../../doc/decisions/0010-board-support-package.md) for
//! the contract this module satisfies.

pub const NAME: &str = "BBC micro:bit v2";

use embassy_nrf::config::{Config, HfclkSource};
use embassy_nrf::gpio::{Level, Output, OutputDrive, Pin};
use embassy_nrf::pwm::SimplePwm;
use embassy_nrf::{bind_interrupts, peripherals, radio};

/// BSP-typed alias for the embassy IEEE 802.15.4 radio driver bound to this board.
pub type Radio = radio::ieee802154::Radio<'static, peripherals::RADIO>;

bind_interrupts!(struct Irqs {
    RADIO => radio::InterruptHandler<peripherals::RADIO>;
});

pub struct Board {
    pub status_led: StatusLed,
    pub motors: Motors,
    pub radio: Radio,
}

impl Board {
    pub fn new() -> Self {
        let mut config = Config::default();
        config.hfclk_source = HfclkSource::ExternalXtal;
        let p = embassy_nrf::init(config);

        Self {
            status_led: StatusLed::new(p.P0_21, p.P0_28),
            motors: Motors::new(p.PWM0, p.P0_17),
            radio: Radio::new(p.RADIO, Irqs),
        }
    }
}

pub struct StatusLed {
    _row: Output<'static>,
    col: Output<'static>,
}

impl StatusLed {
    pub fn new(row: impl Pin, col: impl Pin) -> Self {
        Self {
            _row: Output::new(row.degrade(), Level::High, OutputDrive::Standard),
            col: Output::new(col.degrade(), Level::High, OutputDrive::Standard),
        }
    }

    pub fn on(&mut self) {
        self.col.set_low();
    }
    pub fn off(&mut self) {
        self.col.set_high();
    }
}

pub struct Motors {
    channels: SimplePwm<'static, peripherals::PWM0>,
}

impl Motors {
    const MAX_DUTY: u16 = 1000;

    pub fn new(peripherals: peripherals::PWM0, ch0: impl Pin) -> Self {
        let mut channels = SimplePwm::new_1ch(peripherals, ch0);
        channels.set_prescaler(embassy_nrf::pwm::Prescaler::Div1);
        channels.set_max_duty(Self::MAX_DUTY);
        channels.set_duty(0, Self::MAX_DUTY); //Default to off (100% duty means always off, 0% duty means always on)

        Self { channels }
    }

    pub fn enable(&mut self) {
        self.channels.enable();
    }

    // API symmetry with enable(); will be used by the failsafe path.
    #[expect(dead_code)]
    pub fn disable(&mut self) {
        self.channels.disable();
    }

    pub fn set_throttle(&mut self, channel: usize, percent: u16) {
        let percent = percent.min(100);

        let on_ticks = (u32::from(percent) * u32::from(Self::MAX_DUTY) / 100) as u16; //Promoting to u32 to avoid overflow during multiplication

        // Note, duty is a bit counterintuitive: 0 is full on, max_duty is full off.
        // Duty means off ticks per period
        let duty = Self::MAX_DUTY - on_ticks; //Invert to get duty (off ticks)

        self.channels.set_duty(channel, duty);
    }
}
