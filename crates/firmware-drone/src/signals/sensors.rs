use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Watch};
use firmware_types::Sensors;

const MAX_SUBSCRIBERS: usize = 8;

static SENSORS: Watch<CriticalSectionRawMutex, Sensors, MAX_SUBSCRIBERS> = Watch::new();

pub type Receiver =
    embassy_sync::watch::Receiver<'static, CriticalSectionRawMutex, Sensors, MAX_SUBSCRIBERS>;

pub fn subscribe() -> Receiver {
    SENSORS.receiver().unwrap()
}

pub fn set(sensors: Sensors) {
    SENSORS.sender().send(sensors);
}
