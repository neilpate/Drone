#![no_std]
#![no_main]

//! Drone firmware entry point.
//!
//! Boots the Embassy executor, builds the board-support struct, and spawns
//! the task set. All real subsystem work — IMU sampling, motor mixing,
//! control loops, radio link — lives in tasks spawned from here. Pure logic
//! for those tasks lives in `firmware-drone-core` per ADR 0007 / ADR 0009.
//! Per-board wiring lives in [`board`] per ADR 0010; tasks accept BSP
//! wrapper types and never see physical pins.
//!
//! ## Milestone 1 (to implement)
//!
//! - Print a defmt boot banner including `FICR.DEVICEID` and the build
//!   flavour (per `doc/dev-environment.md` board-labelling rule).
//! - Refuse to continue if the FICR-derived logical ID does not match the
//!   board's physical sticker (`doc/dev-environment.md`).
//! - ~~Blink one LED at a known cadence as a "firmware is alive" heartbeat.~~
//!   Done — see [`heartbeat`].

use defmt_rtt as _;
use panic_probe as _;

use embassy_executor::Spawner;
use embassy_nrf::config::Config;
use embassy_time::Timer;

mod board;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_nrf::init(Config::default());
    let board = board::Board::new(p);

    defmt::info!("firmware-drone on {}: boot (scaffold)", board::NAME);

    spawner.must_spawn(heartbeat(board.status_led));
}

/// "Firmware is alive" heartbeat: blinks the board's status LED at 1 Hz
/// with 50% duty.
///
/// Board-agnostic: the wiring details (single GPIO vs. a corner of the
/// micro:bit v2's charlieplexed matrix) live behind [`board::StatusLed`]
/// per ADR 0010.
#[embassy_executor::task]
async fn heartbeat(mut status_led: board::StatusLed) -> ! {
    defmt::info!("firmware-drone: heartbeat task started");

    loop {
        status_led.on();
        Timer::after_millis(500).await;
        status_led.off();
        Timer::after_millis(500).await;
    }
}
