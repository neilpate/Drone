use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Watch};

const MAX_SUBSCRIBERS: usize = 8;

static IMU_CALIBRATE: Watch<CriticalSectionRawMutex, bool, MAX_SUBSCRIBERS> = Watch::new();

pub type Receiver =
    embassy_sync::watch::Receiver<'static, CriticalSectionRawMutex, bool, MAX_SUBSCRIBERS>;

pub fn subscribe() -> Receiver {
    IMU_CALIBRATE.receiver().unwrap()
}

pub fn set(calibrate: bool) {
    IMU_CALIBRATE.sender().send(calibrate);
}
