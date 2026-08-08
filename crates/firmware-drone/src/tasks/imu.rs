use embassy_time::{Duration, Ticker, Timer};

use crate::board;
use crate::signals::imu_data;
use firmware_types::{Acceleration, AngularRate, ImuData};

const LOOP_PERIOD_MS: u64 = 1; // Loop period in milliseconds

struct ImuCalibration {
    accel_x_offset: Acceleration,
    accel_y_offset: Acceleration,
    gyro_x_offset: AngularRate,
    gyro_y_offset: AngularRate,
    gyro_z_offset: AngularRate,
}

async fn calibrate_imu(imu: &mut board::Imu) -> ImuCalibration {
    const CALIBRATION_SAMPLES: usize = 100;
    let mut accel_x_sum = 0.0;
    let mut accel_y_sum = 0.0;

    let mut gyro_x_sum = 0.0;
    let mut gyro_y_sum = 0.0;
    let mut gyro_z_sum = 0.0;

    let mut collected = 0;
    while collected < CALIBRATION_SAMPLES {
        match imu.read_all().await {
            Ok(d) => {
                accel_x_sum += d.acceleration_x.as_g();
                accel_y_sum += d.acceleration_y.as_g();

                gyro_x_sum += d.angular_rate_x.as_degrees_per_second();
                gyro_y_sum += d.angular_rate_y.as_degrees_per_second();
                gyro_z_sum += d.angular_rate_z.as_degrees_per_second();

                collected += 1;
            }
            Err(e) => defmt::warn!("imu calibration read failed, retrying: {:?}", e),
        }
        Timer::after(Duration::from_millis(1)).await;
    }

    ImuCalibration {
        accel_x_offset: Acceleration::from_g(accel_x_sum / CALIBRATION_SAMPLES as f32),
        accel_y_offset: Acceleration::from_g(accel_y_sum / CALIBRATION_SAMPLES as f32),
        gyro_x_offset: AngularRate::from_degrees_per_second(
            gyro_x_sum / CALIBRATION_SAMPLES as f32,
        ),
        gyro_y_offset: AngularRate::from_degrees_per_second(
            gyro_y_sum / CALIBRATION_SAMPLES as f32,
        ),
        gyro_z_offset: AngularRate::from_degrees_per_second(
            gyro_z_sum / CALIBRATION_SAMPLES as f32,
        ),
    }
}

#[embassy_executor::task]
pub async fn imu(mut imu: board::Imu) -> ! {
    defmt::info!("imu task: started");

    // Startup default = "level at rest": 1 g down the -Z (FRD) axis, no rotation.
    // Must be a physically valid gravity vector (magnitude ~1 g), NOT all-zeros:
    // the estimator seeds its attitude from the first sample via atan2, and a
    // zero-length vector makes atan2 degenerate (NaN on micromath), which then
    // sticks in the filter forever.
    let default_imu_data = ImuData {
        acceleration_x: firmware_types::Acceleration::from_g(0.0),
        acceleration_y: firmware_types::Acceleration::from_g(0.0),
        acceleration_z: firmware_types::Acceleration::from_g(-1.0),
        angular_rate_x: firmware_types::AngularRate::from_degrees_per_second(0.0),
        angular_rate_y: firmware_types::AngularRate::from_degrees_per_second(0.0),
        angular_rate_z: firmware_types::AngularRate::from_degrees_per_second(0.0),
    };

    imu_data::set(default_imu_data); // Initialize the shared signal with default IMU data

    let result = imu.check_identity().await;

    match result {
        Ok(()) => defmt::info!("imu detected over SPI"),
        Err(e) => defmt::error!("imu detection failed: {:?}", e),
    }

    let result = imu.configure().await;

    match result {
        Ok(_) => defmt::info!("imu configured successfully"),
        Err(e) => defmt::error!("imu configuration failed: {:?}", e),
    }

    Timer::after(Duration::from_millis(100)).await; // Give the IMU some time to stabilize after configuration

    let calibration = calibrate_imu(&mut imu).await;

    let mut ticker = Ticker::every(Duration::from_millis(LOOP_PERIOD_MS));

    loop {
        ticker.next().await; // Adjust the delay as needed
        let raw_imu_data = imu.read_all().await; // Read all IMU data (accelerometer, gyroscope, etc.)

        match raw_imu_data {
            Ok(data) => {
                let corrected_imu_data = ImuData {
                    acceleration_x: data.acceleration_x - calibration.accel_x_offset,
                    acceleration_y: data.acceleration_y - calibration.accel_y_offset,
                    acceleration_z: data.acceleration_z,
                    angular_rate_x: data.angular_rate_x - calibration.gyro_x_offset,
                    angular_rate_y: data.angular_rate_y - calibration.gyro_y_offset,
                    angular_rate_z: data.angular_rate_z - calibration.gyro_z_offset,
                };

                imu_data::set(corrected_imu_data); // Update the shared signal with the latest IMU data
            }
            Err(e) => {
                defmt::error!("imu read failed: {:?}", e); // Log any errors encountered during IMU reading
            }
        }
    }
}
