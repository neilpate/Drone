use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Watch};
use firmware_types::YawCommand;

const MAX_SUBSCRIBERS: usize = 8;

static COMMAND: Watch<CriticalSectionRawMutex, YawCommand, MAX_SUBSCRIBERS> = Watch::new();

pub type Receiver =
    embassy_sync::watch::Receiver<'static, CriticalSectionRawMutex, YawCommand, MAX_SUBSCRIBERS>;

pub fn subscribe() -> Receiver {
    COMMAND.receiver().unwrap()
}

pub fn set(yaw: YawCommand) {
    COMMAND.sender().send(yaw);
}
