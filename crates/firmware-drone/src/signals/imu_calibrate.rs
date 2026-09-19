use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};

static IMU_CALIBRATE: Signal<CriticalSectionRawMutex, ()> = Signal::new();

pub fn signal() {
    IMU_CALIBRATE.signal(());
}

//Non-blocking check if the IMU calibration signal has been set
pub fn check() -> bool {
    IMU_CALIBRATE.try_take().is_some()
}
