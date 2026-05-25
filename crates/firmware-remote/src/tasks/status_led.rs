use crate::board;
use crate::tasks::supervisor::{StatusReceiver, SystemState, subscribe};
use embassy_futures::select::{Either, select};
use embassy_time::Timer;

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
pub async fn update_status_indicator(mut status_led: board::StatusLed) -> ! {
    defmt::info!("update_status_indicator task: started");

    let mut status_receiver = subscribe();

    let mut current_state = status_receiver.changed().await;

    loop {
        let pattern = pattern_for_state(current_state);
        current_state = play_pattern(&mut status_led, pattern, &mut status_receiver).await;
    }
}
