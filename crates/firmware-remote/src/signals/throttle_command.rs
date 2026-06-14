use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Watch};
use firmware_types::Throttle;

const MAX_SUBSCRIBERS: usize = 8;

static THROTTLE_COMMAND: Watch<CriticalSectionRawMutex, Throttle, MAX_SUBSCRIBERS> = Watch::new();

pub type Receiver =
    embassy_sync::watch::Receiver<'static, CriticalSectionRawMutex, Throttle, MAX_SUBSCRIBERS>;

pub fn subscribe() -> Receiver {
    THROTTLE_COMMAND.receiver().unwrap()
}

pub fn set(throttle: Throttle) {
    THROTTLE_COMMAND.sender().send(throttle);
}
