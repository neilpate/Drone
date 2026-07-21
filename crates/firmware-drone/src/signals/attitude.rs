use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Watch};
use firmware_types::Attitude;

const MAX_SUBSCRIBERS: usize = 8;

static ATTITUDE: Watch<CriticalSectionRawMutex, Attitude, MAX_SUBSCRIBERS> = Watch::new();

pub type Receiver =
    embassy_sync::watch::Receiver<'static, CriticalSectionRawMutex, Attitude, MAX_SUBSCRIBERS>;

pub fn subscribe() -> Receiver {
    ATTITUDE.receiver().unwrap()
}

pub fn set(attitude: Attitude) {
    ATTITUDE.sender().send(attitude);
}
