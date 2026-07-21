use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Watch};
use firmware_types::RollCommand;

const MAX_SUBSCRIBERS: usize = 8;

static COMMAND: Watch<CriticalSectionRawMutex, RollCommand, MAX_SUBSCRIBERS> = Watch::new();

pub type Receiver =
    embassy_sync::watch::Receiver<'static, CriticalSectionRawMutex, RollCommand, MAX_SUBSCRIBERS>;

pub fn subscribe() -> Receiver {
    COMMAND.receiver().unwrap()
}

pub fn set(roll: RollCommand) {
    COMMAND.sender().send(roll);
}
