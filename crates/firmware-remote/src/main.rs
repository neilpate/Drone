#![no_std]
#![no_main]

//! Remote firmware entry point.
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

mod board;
mod radio_link;
mod signals;
mod tasks;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let board = board::Board::new();

    defmt::info!("firmware-remote on {}: boot (scaffold)", board::NAME);

    let (uart_tx, uart_rx) = board.uart.split();

    spawner.must_spawn(tasks::supervisor::supervisor());
    spawner.must_spawn(tasks::status_led::status_led(board.status_led));
    spawner.must_spawn(tasks::drone_link::drone_link(board.radio));
    spawner.must_spawn(tasks::serial_link_rx::serial_link_rx(uart_rx));
    spawner.must_spawn(tasks::serial_link_tx::serial_link_tx(uart_tx));
}
