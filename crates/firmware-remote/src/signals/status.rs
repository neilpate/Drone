use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Watch};
use firmware_types::RemoteState;

const MAX_SUBSCRIBERS: usize = 8;

static STATUS: Watch<CriticalSectionRawMutex, RemoteState, MAX_SUBSCRIBERS> = Watch::new();

pub type Receiver =
    embassy_sync::watch::Receiver<'static, CriticalSectionRawMutex, RemoteState, MAX_SUBSCRIBERS>;

pub fn subscribe() -> Receiver {
    STATUS.receiver().unwrap()
}

pub fn set(s: RemoteState) {
    STATUS.sender().send(s);
}
