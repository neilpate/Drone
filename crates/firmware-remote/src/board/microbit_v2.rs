//! BBC micro:bit v2 (nRF52833) board support.
//!
//! Phase 1–3 development target. Sensors and actuators are breadboarded onto
//! the edge connector; only the on-board 5x5 LED matrix is used directly.
//!
//! See [ADR 0010](../../../../doc/decisions/0010-board-support-package.md) for
//! the contract this module satisfies.

pub const NAME: &str = "BBC micro:bit v2";

use embassy_nrf::gpio::{Level, Output, OutputDrive, Pin};
use embassy_nrf::{bind_interrupts, peripherals, radio};

/// BSP-typed alias for the embassy BLE-mode radio driver bound to this board.
pub type Radio = radio::ble::Radio<'static, peripherals::RADIO>;

bind_interrupts!(struct Irqs {
    RADIO => radio::InterruptHandler<peripherals::RADIO>;
});

pub struct Board {
    pub status_led: StatusLed,
    // Consumed by the telemetry RX task in a later commit of issue #5.
    #[allow(dead_code)]
    pub radio: Radio,
}

impl Board {
    pub fn new(p: embassy_nrf::Peripherals) -> Self {
        Self {
            status_led: StatusLed::new(p.P0_21, p.P0_28),
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
