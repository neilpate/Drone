use embassy_time::Instant;

use firmware_drone_core::sensor_fusion::AttitudeEstimator;

use crate::signals::{attitude, imu_data};

#[embassy_executor::task]
pub async fn attitude_estimator() -> ! {
    defmt::info!("attitude estimator task: started");

    let mut imu_receiver = imu_data::subscribe();

    let mut attitude_estimator = AttitudeEstimator::new();

    let mut last = Instant::now();

    loop {
        let imu_data = imu_receiver.changed().await;

        let now = Instant::now();
        let loop_duration_s = (now - last).as_micros() as f32 / 1_000_000.0;
        last = now;

        let current_attitude = attitude_estimator.update(imu_data, loop_duration_s);

        attitude::set(current_attitude);
    }
}
