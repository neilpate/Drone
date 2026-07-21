use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Watch};
use firmware_types::ThrottleCommand;

const MAX_SUBSCRIBERS: usize = 8;

static COMMAND: Watch<CriticalSectionRawMutex, ThrottleCommand, MAX_SUBSCRIBERS> = Watch::new();

pub type Receiver =
    embassy_sync::watch::Receiver<'static, CriticalSectionRawMutex, ThrottleCommand, MAX_SUBSCRIBERS>;

pub fn subscribe() -> Receiver {
    COMMAND.receiver().unwrap()
}

pub fn set(throttle: ThrottleCommand) {
    COMMAND.sender().send(throttle);
}
