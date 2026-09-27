use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Watch};
use firmware_types::ImuData;

const MAX_SUBSCRIBERS: usize = 8;

static IMU_DATA: Watch<CriticalSectionRawMutex, ImuData, MAX_SUBSCRIBERS> = Watch::new();
static IMU_DATA_PROCESSED: Watch<CriticalSectionRawMutex, ImuData, MAX_SUBSCRIBERS> = Watch::new();

pub type Receiver =
    embassy_sync::watch::Receiver<'static, CriticalSectionRawMutex, ImuData, MAX_SUBSCRIBERS>;

pub fn subscribe() -> Receiver {
    IMU_DATA.receiver().unwrap()
}

pub fn set(imu_data: ImuData) {
    IMU_DATA.sender().send(imu_data);
}

pub fn subscribe_processed() -> Receiver {
    IMU_DATA_PROCESSED.receiver().unwrap()
}

pub fn set_processed(imu_data: ImuData) {
    IMU_DATA_PROCESSED.sender().send(imu_data);
}
