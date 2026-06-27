use embassy_time::{Duration, Timer};

use crate::board;

#[embassy_executor::task]
pub async fn imu(mut imu: board::Imu) -> ! {
    defmt::info!("imu task: started");

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
        imu.read_all().await; // Read all IMU data (accelerometer, gyroscope, etc.)
    }
}
