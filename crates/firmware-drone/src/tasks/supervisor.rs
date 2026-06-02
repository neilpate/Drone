use embassy_futures::select::{Either, select};
use embassy_time::{Duration, Ticker};

use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Watch};

pub use firmware_drone_core::supervisor_core::SystemState;
use firmware_types::Throttle;

use crate::tasks::pilot_command;

const TIMEOUT_PERIOD: Duration = Duration::from_millis(10);

const MAX_SUBSCRIBERS: usize = 8;

static STATUS: Watch<CriticalSectionRawMutex, SystemState, MAX_SUBSCRIBERS> = Watch::new();

pub type StatusReceiver =
    embassy_sync::watch::Receiver<'static, CriticalSectionRawMutex, SystemState, MAX_SUBSCRIBERS>;

pub fn subscribe() -> StatusReceiver {
    STATUS.receiver().unwrap()
}

fn set(s: SystemState) {
    STATUS.sender().send(s);
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SafeValues {
    pub throttle: Throttle,
}

static SAFE_VALUES: Watch<CriticalSectionRawMutex, SafeValues, MAX_SUBSCRIBERS> = Watch::new();

pub type SafeValuesReceiver =
    embassy_sync::watch::Receiver<'static, CriticalSectionRawMutex, SafeValues, MAX_SUBSCRIBERS>;

pub fn subscribe_safe_values() -> SafeValuesReceiver {
    SAFE_VALUES.receiver().unwrap()
}

pub fn set_safe_values(safe_values: SafeValues) {
    SAFE_VALUES.sender().send(safe_values);
}

#[embassy_executor::task]
pub async fn supervise() -> ! {
    defmt::info!("supervisor task: started");
    set(SystemState::Initialising);

    let mut ticker = Ticker::every(TIMEOUT_PERIOD);

    let mut pilot_command_receiver = pilot_command::subscribe();

    let mut safe_values = SafeValues {
        throttle: Throttle::ZERO,
    };

    let mut ticks_without_command = 0;

    loop {
        // Wait either for a new PilotCommand to arrive, or for the timeout to elapse.

        let event = select(pilot_command_receiver.changed(), ticker.next()).await;

        match event {
            Either::First(pilot_command) => {
                safe_values.throttle = pilot_command.throttle;
                set_safe_values(safe_values);
                ticks_without_command = 0;
            }

            Either::Second(_) => {
                ticks_without_command += 1;

                if ticks_without_command > 10 {
                    defmt::warn!(
                        "supervisor: no pilot command received for 100ms, entering degraded mode"
                    );
                    safe_values.throttle = Throttle::ZERO;
                    set_safe_values(safe_values);
                }
                set(SystemState::Degraded);
            }
        }
    }
}
