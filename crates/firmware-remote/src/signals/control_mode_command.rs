use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Watch};
use firmware_types::ControlMode;

const MAX_SUBSCRIBERS: usize = 8;

static COMMAND: Watch<CriticalSectionRawMutex, ControlMode, MAX_SUBSCRIBERS> = Watch::new();

pub type Receiver =
    embassy_sync::watch::Receiver<'static, CriticalSectionRawMutex, ControlMode, MAX_SUBSCRIBERS>;

pub fn subscribe() -> Receiver {
    COMMAND.receiver().unwrap()
}

pub fn set(control_mode: ControlMode) {
    COMMAND.sender().send(control_mode);
}
