use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Watch};
use firmware_drone_core::supervisor_core::SystemState;

const MAX_SUBSCRIBERS: usize = 8;

static STATUS: Watch<CriticalSectionRawMutex, SystemState, MAX_SUBSCRIBERS> = Watch::new();

pub type StatusReceiver =
    embassy_sync::watch::Receiver<'static, CriticalSectionRawMutex, SystemState, MAX_SUBSCRIBERS>;

pub fn subscribe() -> StatusReceiver {
    STATUS.receiver().unwrap()
}

pub fn set(s: SystemState) {
    STATUS.sender().send(s);
}
