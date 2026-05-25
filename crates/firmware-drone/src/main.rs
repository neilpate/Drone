#![no_std]
#![no_main]

//! Drone firmware entry point.
//!
//! Initialises the board, publishes the initial [`SystemState`], and spawns
//! the task set. Subsystem work — IMU sampling, motor mixing, control loops,
//! radio link — lives in tasks spawned from here. Pure logic for those tasks
//! lives in `firmware-drone-core` per ADR 0007 / ADR 0009. Per-board wiring
//! lives in [`board`] per ADR 0010; tasks accept BSP wrapper types and never
//! see physical pins. Shared state is published via [`system_state::STATUS`]
//! per ADR 0013.

use defmt_rtt as _;
use panic_probe as _;

use embassy_executor::Spawner;
use embassy_nrf::config::Config;

use embassy_time::Timer;

mod board;
mod system_state;
use system_state::SystemState;
mod tasks;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    system_state::set(SystemState::Booting);

    let p = embassy_nrf::init(Config::default());
    let board = board::Board::new(p);

    defmt::info!("firmware-drone on {}: boot (scaffold)", board::NAME);

    spawner.must_spawn(tasks::status_led::update_status_indicator(board.status_led));

    // Throwaway demo: cycle through every SystemState so all LED patterns get
    // exercised on hardware. Will be replaced by real subsystem-driven
    // transitions as subsystems come online.
    Timer::after_millis(5000).await;
    defmt::info!("firmware-drone on {}: -> Idle", board::NAME);
    system_state::set(SystemState::Idle);

    Timer::after_millis(5000).await;
    defmt::info!("firmware-drone on {}: -> Fault", board::NAME);
    system_state::set(SystemState::Fault);
}
