use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Watch};
use firmware_types::Throttle;

const THROTTLE_MAX_SUBSCRIBERS: usize = 8;

static THROTTLE_COMMAND: Watch<CriticalSectionRawMutex, Throttle, THROTTLE_MAX_SUBSCRIBERS> =
    Watch::new();

pub type ThrottleCommandReceiver = embassy_sync::watch::Receiver<
    'static,
    CriticalSectionRawMutex,
    Throttle,
    THROTTLE_MAX_SUBSCRIBERS,
>;

pub fn subscribe_throttle_command() -> ThrottleCommandReceiver {
    THROTTLE_COMMAND.receiver().unwrap()
}

pub fn set_throttle_command(throttle: Throttle) {
    THROTTLE_COMMAND.sender().send(throttle);
}
