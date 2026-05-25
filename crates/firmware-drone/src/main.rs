#![no_std]
#![no_main]

//! Drone firmware entry point.
//!
//! Initialises the board and spawns the task set. Subsystem work — IMU
//! sampling, motor mixing, control loops, radio link — lives in tasks
//! spawned from here. Pure logic for those tasks lives in
//! `firmware-drone-core` per ADR 0007 / ADR 0009. Per-board wiring lives in
//! [`board`] per ADR 0010; tasks accept BSP wrapper types and never see
//! physical pins. System-wide state is owned and published by the
//! supervisor task ([`tasks::supervisor`]) per ADR 0013.

use defmt_rtt as _;
use panic_probe as _;

use embassy_executor::Spawner;
use embassy_nrf::config::Config;

mod board;
mod tasks;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_nrf::init(Config::default());
    let board = board::Board::new(p);

    defmt::info!("firmware-drone on {}: boot (scaffold)", board::NAME);

    spawner.must_spawn(tasks::supervisor::supervise());
    spawner.must_spawn(tasks::status_led::update_status_indicator(board.status_led));
}
