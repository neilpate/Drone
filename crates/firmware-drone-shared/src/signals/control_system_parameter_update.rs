use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Watch};
use firmware_types::ControlSystemParameters;

const MAX_SUBSCRIBERS: usize = 8;

static CONTROL_SYSTEM_PARAMETER_UPDATE: Watch<
    CriticalSectionRawMutex,
    ControlSystemParameters,
    MAX_SUBSCRIBERS,
> = Watch::new();

pub type Receiver = embassy_sync::watch::Receiver<
    'static,
    CriticalSectionRawMutex,
    ControlSystemParameters,
    MAX_SUBSCRIBERS,
>;

pub fn subscribe() -> Receiver {
    CONTROL_SYSTEM_PARAMETER_UPDATE.receiver().unwrap()
}

pub fn set(update: ControlSystemParameters) {
    CONTROL_SYSTEM_PARAMETER_UPDATE.sender().send(update);
}
