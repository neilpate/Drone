use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Watch};
use firmware_types::ControllerDemand;

const MAX_SUBSCRIBERS: usize = 8;

static CONTROLLER_DEMAND: Watch<CriticalSectionRawMutex, ControllerDemand, MAX_SUBSCRIBERS> =
    Watch::new();

pub type Receiver = embassy_sync::watch::Receiver<
    'static,
    CriticalSectionRawMutex,
    ControllerDemand,
    MAX_SUBSCRIBERS,
>;

pub fn subscribe() -> Receiver {
    CONTROLLER_DEMAND.receiver().unwrap()
}

pub fn set(command: ControllerDemand) {
    CONTROLLER_DEMAND.sender().send(command);
}
