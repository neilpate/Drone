use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Watch};
use firmware_types::PilotCommand;

const MAX_SUBSCRIBERS: usize = 8;

static PILOT_COMMAND: Watch<CriticalSectionRawMutex, PilotCommand, MAX_SUBSCRIBERS> = Watch::new();

pub type Receiver =
    embassy_sync::watch::Receiver<'static, CriticalSectionRawMutex, PilotCommand, MAX_SUBSCRIBERS>;

pub fn subscribe() -> Receiver {
    PILOT_COMMAND.receiver().unwrap()
}

pub fn set(command: PilotCommand) {
    PILOT_COMMAND.sender().send(command);
}
