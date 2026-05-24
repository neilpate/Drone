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
use embassy_futures::select::{Either, select};
use panic_probe as _;

use embassy_executor::Spawner;
use embassy_nrf::config::Config;

use embassy_time::Timer;

mod board;
mod system_state;
use system_state::SystemState;

use crate::system_state::StatusReceiver;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    system_state::set(SystemState::Booting);

    let p = embassy_nrf::init(Config::default());
    let board = board::Board::new(p);

    defmt::info!("firmware-drone on {}: boot (scaffold)", board::NAME);

    spawner.must_spawn(update_status_indicator(board.status_led));

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

enum LedPattern {
    Blinking { on_ms: u64, off_ms: u64 },
}

fn pattern_for_state(s: SystemState) -> LedPattern {
    match s {
        // Fast symmetric blink: "working hard" during init.
        SystemState::Booting => LedPattern::Blinking {
            on_ms: 125,
            off_ms: 125,
        },
        // Heartbeat blip: short flash, long pause. Classic "alive but idle".
        SystemState::Idle => LedPattern::Blinking {
            on_ms: 50,
            off_ms: 1950,
        },
        // Rapid strobe: universally reads as "alarm".
        SystemState::Fault => LedPattern::Blinking {
            on_ms: 50,
            off_ms: 50,
        },
    }
}

async fn play_pattern(
    status_led: &mut board::StatusLed,
    pattern: LedPattern,
    state_change_receiver: &mut StatusReceiver,
) -> SystemState {
    match pattern {
        LedPattern::Blinking { on_ms, off_ms } => loop {
            status_led.on();
            if let Either::Second(state) =
                select(Timer::after_millis(on_ms), state_change_receiver.changed()).await
            {
                return state;
            }

            status_led.off();
            if let Either::Second(state) =
                select(Timer::after_millis(off_ms), state_change_receiver.changed()).await
            {
                return state;
            }
        },
    }
}

#[embassy_executor::task]
async fn update_status_indicator(mut status_led: board::StatusLed) -> ! {
    defmt::info!("update_status_indicator task: started");

    let mut status_receiver = system_state::STATUS.receiver().unwrap();

    let mut current_state = status_receiver.changed().await;

    loop {
        let pattern = pattern_for_state(current_state);
        current_state = play_pattern(&mut status_led, pattern, &mut status_receiver).await;
    }
}
