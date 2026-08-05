use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Watch};
use firmware_types::ControlMode;

const MAX_SUBSCRIBERS: usize = 8;

static CONTROL_MODE_UPDATE: Watch<CriticalSectionRawMutex, ControlMode, MAX_SUBSCRIBERS> =
    Watch::new();

pub type Receiver =
    embassy_sync::watch::Receiver<'static, CriticalSectionRawMutex, ControlMode, MAX_SUBSCRIBERS>;

pub fn subscribe() -> Receiver {
    CONTROL_MODE_UPDATE.receiver().unwrap()
}

pub fn set(mode: ControlMode) {
    CONTROL_MODE_UPDATE.sender().send(mode);
}
