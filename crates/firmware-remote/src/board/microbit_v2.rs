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
use embassy_nrf::uarte::{self, Baudrate, Parity, Uarte, UarteRx, UarteTx};
use embassy_nrf::{bind_interrupts, peripherals, radio};

/// BSP-typed alias for the embassy IEEE 802.15.4 radio driver bound to this board.
pub type Radio = radio::ieee802154::Radio<'static, peripherals::RADIO>;

pub type Uart = Uarte<'static, peripherals::UARTE0>;
pub type UartRx = UarteRx<'static, peripherals::UARTE0>;
pub type UartTx = UarteTx<'static, peripherals::UARTE0>;

bind_interrupts!(struct Irqs {
    RADIO => radio::InterruptHandler<peripherals::RADIO>;
    UARTE0_UART0 => uarte::InterruptHandler<peripherals::UARTE0>;
});

pub struct Board {
    pub status_led: StatusLed,
    pub radio: Radio,
    pub uart: Uart,
}

impl Board {
    pub fn new() -> Self {
        let mut config = Config::default();
        config.hfclk_source = HfclkSource::ExternalXtal;
        let p = embassy_nrf::init(config);

        let mut uart_config = uarte::Config::default();
        uart_config.baudrate = Baudrate::BAUD115200;
        uart_config.parity = Parity::EXCLUDED;

        Self {
            status_led: StatusLed::new(p.P0_21, p.P0_28),
            radio: Radio::new(p.RADIO, Irqs),
            uart: Uart::new(p.UARTE0, Irqs, p.P1_08, p.P0_06, uart_config),
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
