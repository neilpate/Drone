#![no_std]
#![no_main]

//! Raw 802.15.4 payload bridge. Application message types stay on the app core.

use defmt_rtt as _;
use embassy_executor::Spawner;
use panic_probe as _;

mod board;
mod tasks;

include!(concat!(env!("OUT_DIR"), "/version.rs"));
#[unsafe(link_section = ".buildconfig")]
#[used]
static VERSION: u32 = FIRMWARE_VERSION;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let board = board::Board::new().unwrap_or_else(|error| {
        defmt::panic!("radio-core board initialization failed: {:?}", error)
    });
    spawner.must_spawn(tasks::status_led::status_led(board.status_led));
    spawner.must_spawn(tasks::remote_link::remote_link(
        board.radio,
        board.app_core_interface,
    ));
}
