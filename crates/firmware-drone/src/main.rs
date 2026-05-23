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
use embassy_time::Timer;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let _p = embassy_nrf::init(Config::default());

    defmt::info!("firmware-drone: boot (scaffold)");

    loop {
        Timer::after_secs(1).await;
    }
}
