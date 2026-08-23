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
use embassy_executor::{InterruptExecutor, Spawner};
use embassy_nrf::interrupt;
use embassy_nrf::interrupt::{InterruptExt, Priority};
use firmware_types::CpuLoad;
use panic_probe as _;

mod board;
mod radio_link;
mod signals;
mod tasks;

use crate::signals::cpu_load;

static EXEC: InterruptExecutor = InterruptExecutor::new();
#[interrupt]
unsafe fn SWI0_EGU0() {
    unsafe { EXEC.on_interrupt() }
}

#[embassy_executor::main]
async fn main(_thread_mode_spawner: Spawner) {
    let board = board::Board::new();

    defmt::info!("firmware-drone on {}: boot ", board::NAME);

    let _calibration_baseline = tasks::load_profiler::calibrate();

    interrupt::SWI0_EGU0.set_priority(Priority::P6);
    let high_priority_spawner = EXEC.start(interrupt::SWI0_EGU0);

    high_priority_spawner.must_spawn(tasks::supervisor::supervisor());
    high_priority_spawner.must_spawn(tasks::status_led::status_led(board.status_led));
    high_priority_spawner.must_spawn(tasks::remote_link::remote_link(board.radio));
    high_priority_spawner.must_spawn(tasks::esc_telemetry::esc_telemetry(board.esc_telemetry));
    high_priority_spawner.must_spawn(tasks::motor_controller::motor_controller(board.motors));
    high_priority_spawner.must_spawn(tasks::temperature::temperature(board.temperature_sensor));
    high_priority_spawner.must_spawn(tasks::sensors_aggregator::sensors_aggregator());
    high_priority_spawner.must_spawn(tasks::imu::imu(board.imu));
    high_priority_spawner.must_spawn(tasks::attitude_estimator::attitude_estimator());
    high_priority_spawner.must_spawn(tasks::control_system::control_system());
    high_priority_spawner.must_spawn(tasks::telemetry_aggregator::telemetry_aggregator());

    // Seed cpu_load so the telemetry aggregator never blocks on first-publish.
    // Harmless when the profiler runs (it overwrites this 0%); keeps telemetry
    // alive when the profiler is disabled.
    cpu_load::set(CpuLoad::from_percentage(0.0));

    // thread_mode_spawner.must_spawn(tasks::load_profiler::load_profiler(calibration_baseline));
}
