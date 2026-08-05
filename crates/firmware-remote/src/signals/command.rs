use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Watch};
use firmware_types::Command;

const MAX_SUBSCRIBERS: usize = 8;

static COMMAND: Watch<CriticalSectionRawMutex, Command, MAX_SUBSCRIBERS> = Watch::new();

pub type Receiver =
    embassy_sync::watch::Receiver<'static, CriticalSectionRawMutex, Command, MAX_SUBSCRIBERS>;

pub fn subscribe() -> Receiver {
    COMMAND.receiver().unwrap()
}

pub fn set(command: Command) {
    COMMAND.sender().send(command);
}
