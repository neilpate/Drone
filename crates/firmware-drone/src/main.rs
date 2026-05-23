#![no_std]
#![no_main]

//! Drone firmware entry point.
//!
//! Boots the Embassy executor on the nRF52833 and idles. All real subsystem
//! work — IMU sampling, motor mixing, control loops, radio link — lives in
//! tasks spawned from here. Pure logic for those tasks lives in
//! `firmware-drone-core` per ADR 0007 / ADR 0009.
//!
//! ## Milestone 1 (to implement)
//!
//! - Print a defmt boot banner including `FICR.DEVICEID` and the build
//!   flavour (per `doc/dev-environment.md` board-labelling rule).
//! - Refuse to continue if the FICR-derived logical ID does not match the
//!   board's physical sticker (`doc/dev-environment.md`).
//! - Blink one micro:bit LED at a known cadence as a "firmware is alive"
//!   heartbeat.

use defmt_rtt as _;
use panic_probe as _;

use embassy_executor::Spawner;
use embassy_nrf::config::Config;
use embassy_nrf::gpio::{AnyPin, Level, Output, OutputDrive, Pin};
use embassy_time::Timer;

mod board;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_nrf::init(Config::default());
    let board = board::Board::new(p);

    defmt::info!("firmware-drone on {}: boot (scaffold)", board::NAME);

    spawner.must_spawn(heartbeat(
        board.heartbeat_row.degrade(),
        board.heartbeat_col.degrade(),
    ));
}

/// "Firmware is alive" heartbeat: blinks one LED on the board.
///
/// Row is held HIGH for the lifetime of the task; the column is toggled,
/// so column LOW = LED on, column HIGH = LED off. This matches the
/// charlieplexed LED matrix on the micro:bit v2; a future board with a
/// dedicated status LED will short one of the two pins to its power rail.
#[embassy_executor::task]
async fn heartbeat(row: AnyPin, col: AnyPin) -> ! {
    defmt::info!("firmware-drone: hearbeat task started");
    let _row = Output::new(row, Level::High, OutputDrive::Standard);
    let mut col = Output::new(col, Level::High, OutputDrive::Standard);

    loop {
        col.set_low();
        Timer::after_millis(500).await;
        col.set_high();
        Timer::after_millis(500).await;
    }
}
