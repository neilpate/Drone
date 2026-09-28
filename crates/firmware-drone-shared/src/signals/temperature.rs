use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Watch};
use firmware_types::Temperature;

const MAX_SUBSCRIBERS: usize = 8;

static TEMPERATURE: Watch<CriticalSectionRawMutex, Temperature, MAX_SUBSCRIBERS> = Watch::new();

pub type Receiver =
    embassy_sync::watch::Receiver<'static, CriticalSectionRawMutex, Temperature, MAX_SUBSCRIBERS>;

pub fn subscribe() -> Receiver {
    TEMPERATURE.receiver().unwrap()
}

pub fn set(temperature: Temperature) {
    TEMPERATURE.sender().send(temperature);
}
