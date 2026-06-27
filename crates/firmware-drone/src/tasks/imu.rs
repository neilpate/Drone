use embassy_time::{Duration, Timer};

use crate::board;
use crate::signals::imu_data;
use firmware_types::ImuData;

#[embassy_executor::task]
pub async fn imu(mut imu: board::Imu) -> ! {
    defmt::info!("imu task: started");

    let default_imu_data = ImuData {
        acceleration_x: firmware_types::Acceleration::from_g(0.0),
        acceleration_y: firmware_types::Acceleration::from_g(0.0),
        acceleration_z: firmware_types::Acceleration::from_g(0.0),
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

    loop {
        // Placeholder for future IMU reading and processing logic
        Timer::after(Duration::from_millis(100)).await; // Adjust the delay as needed
        let imu_data = imu.read_all().await; // Read all IMU data (accelerometer, gyroscope, etc.)

        match imu_data {
            Ok(data) => {
                imu_data::set(data); // Update the shared signal with the latest IMU data
                defmt::info!("imu data: {:?}", data); // Log the IMU data for debugging purposes
            }
            Err(e) => {
                defmt::error!("imu read failed: {:?}", e); // Log any errors encountered during IMU reading
            }
        }
    }
}
