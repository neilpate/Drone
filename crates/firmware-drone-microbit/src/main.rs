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

include!(concat!(env!("OUT_DIR"), "/version.rs"));
#[unsafe(link_section = ".buildconfig")]
#[used]
static VERSION: u32 = FIRMWARE_VERSION;

use defmt_rtt as _;
use embassy_executor::{InterruptExecutor, SendSpawner, Spawner};
use embassy_nrf::interrupt;
use embassy_nrf::interrupt::{InterruptExt, Priority};
use firmware_drone_shared::signals::cpu_load;

use firmware_drone_shared::spawn_shared_tasks;
// use firmware_drone_shared::{load_profiler, spawn_load_profiler}; //Not used for now
use firmware_types::CpuLoad;
use panic_probe as _;

mod board;
mod radio_link;
mod tasks;

static EXEC: InterruptExecutor = InterruptExecutor::new();
#[interrupt]
unsafe fn SWI0_EGU0() {
    unsafe { EXEC.on_interrupt() }
}

fn spawn_hardware_tasks(spawner: SendSpawner, board: board::Board) {
    spawner.must_spawn(tasks::status_led::status_led(board.status_led));
    spawner.must_spawn(tasks::remote_link::remote_link(board.radio));
    spawner.must_spawn(tasks::esc_telemetry::esc_telemetry(board.esc_telemetry));
    spawner.must_spawn(tasks::motor_controller::motor_controller(board.motors));
    spawner.must_spawn(tasks::temperature::temperature(board.temperature_sensor));
    spawner.must_spawn(tasks::imu::imu(board.imu));
    spawner.must_spawn(tasks::config_manager::config_manager(board.config_storage));
}

#[embassy_executor::main]
async fn main(_thread_mode_spawner: Spawner) {
    let board = board::Board::new();

    defmt::info!("firmware-drone-microbit on {}: boot ", board::NAME);

    // let calibration_baseline = load_profiler::calibrate();
    // Seed cpu_load so the telemetry aggregator never blocks on first-publish.
    // Harmless when the profiler runs (it overwrites this 0%); keeps telemetry
    // alive when the profiler is disabled.
    cpu_load::set(CpuLoad::from_percentage(0.0));

    interrupt::SWI0_EGU0.set_priority(Priority::P6);
    let high_priority_spawner = EXEC.start(interrupt::SWI0_EGU0);

    spawn_shared_tasks(high_priority_spawner, FIRMWARE_VERSION);
    spawn_hardware_tasks(high_priority_spawner, board);

    // spawn_load_profiler(_thread_mode_spawner, calibration_baseline);
}
