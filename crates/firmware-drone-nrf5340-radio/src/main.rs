#![no_std]
#![no_main]

//! Raw 802.15.4 payload bridge. Application message types stay on the app core.

use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_nrf::config::{Config, HfclkSource};
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use embassy_nrf::radio::ieee802154::Radio;
use embassy_nrf::{bind_interrupts, peripherals, radio};
use panic_probe as _;

mod tasks;

include!(concat!(env!("OUT_DIR"), "/version.rs"));
#[unsafe(link_section = ".buildconfig")]
#[used]
static VERSION: u32 = FIRMWARE_VERSION;

bind_interrupts!(struct Irqs {
    RADIO => radio::InterruptHandler<peripherals::RADIO>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut config = Config::default();
    config.hfclk_source = HfclkSource::ExternalXtal;
    let p = embassy_nrf::init(config);
    spawner.must_spawn(tasks::status_led::status_led(Output::new(
        p.P0_31,
        Level::Low,
        OutputDrive::Standard,
    )));
    spawner.must_spawn(tasks::remote_link::remote_link(Radio::new(p.RADIO, Irqs)));
}
