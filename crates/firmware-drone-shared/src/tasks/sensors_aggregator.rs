use embassy_time::{Duration, Ticker};
use firmware_types::Sensors;

use crate::signals::{imu_data, sensors, temperature};

#[embassy_executor::task]
pub async fn sensors_aggregator() -> ! {
    defmt::info!("sensors aggregator task: started");

    let mut temperature_receiver = temperature::subscribe();
    let mut imu_receiver = imu_data::subscribe();

    let mut ticker = Ticker::every(Duration::from_millis(10));

    loop {
        ticker.next().await;

        let temperature = temperature_receiver.get().await;
        let imu_data = imu_receiver.get().await;

        let sensors = Sensors {
            temperature,
            imu: imu_data,
        };

        sensors::set(sensors);
    }
}
