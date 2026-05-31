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
use embassy_nrf::{bind_interrupts, peripherals, radio};

/// BSP-typed alias for the embassy IEEE 802.15.4 radio driver bound to this board.
pub type Radio = radio::ieee802154::Radio<'static, peripherals::RADIO>;

bind_interrupts!(struct Irqs {
    RADIO => radio::InterruptHandler<peripherals::RADIO>;
});

pub struct Board {
    pub status_led: StatusLed,
    pub motor: Motor,
    pub radio: Radio,
}

impl Board {
    pub fn new() -> Self {
        let mut config = Config::default();
        config.hfclk_source = HfclkSource::ExternalXtal;
        let p = embassy_nrf::init(config);

        Self {
            status_led: StatusLed::new(p.P0_21, p.P0_28),
            motor: Motor::new(p.P0_17, p.P0_01),
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

pub struct Motor {
    ia: Output<'static>,
    ib: Output<'static>,
}

impl Motor {
    pub fn new(ia: impl Pin, ib: impl Pin) -> Self {
        Self {
            ia: Output::new(ia.degrade(), Level::Low, OutputDrive::Standard),
            ib: Output::new(ib.degrade(), Level::Low, OutputDrive::Standard),
        }
    }

    pub fn forward(&mut self) {
        self.ia.set_high();
        self.ib.set_low();
    }
    pub fn reverse(&mut self) {
        self.ia.set_low();
        self.ib.set_high();
    }

    pub fn coast(&mut self) {
        self.ia.set_low();
        self.ib.set_low();
    }
}
