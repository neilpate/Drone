use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Watch};
use firmware_types::Telemetry;

const MAX_SUBSCRIBERS: usize = 8;

static THROTTLE_COMMAND: Watch<CriticalSectionRawMutex, Telemetry, MAX_SUBSCRIBERS> =
    Watch::new();

pub type Receiver = embassy_sync::watch::Receiver<
    'static,
    CriticalSectionRawMutex,
    Telemetry,
    MAX_SUBSCRIBERS,
>;

pub fn subscribe() -> Receiver {
    THROTTLE_COMMAND.receiver().unwrap()
}

pub fn set(throttle: Telemetry) {
    THROTTLE_COMMAND.sender().send(throttle);
}
