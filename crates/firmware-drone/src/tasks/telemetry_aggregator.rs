use embassy_time::{Duration, Ticker};
use firmware_types::Telemetry;

use crate::signals::{cpu_load, imu_data, pilot_command, sensors, status, telemetry};

#[embassy_executor::task]
pub async fn telemetry_aggregator() -> ! {
    defmt::info!("telemetry aggregator task: started");

    let mut sequence_count: u32 = 0;

    let mut status_receiver = status::subscribe();
    let mut sensors_receiver = sensors::subscribe();
    let mut pilot_command_receiver = pilot_command::subscribe();
    let mut cpu_load_receiver = cpu_load::subscribe();
    let mut imu_receiver = imu_data::subscribe();

    let mut ticker = Ticker::every(Duration::from_millis(10));

    loop {
        ticker.next().await;

        sequence_count = sequence_count.wrapping_add(1);

        let drone_state = status_receiver.get().await;
        let sensors = sensors_receiver.get().await;
        let pilot_command = pilot_command_receiver.get().await;
        let cpu_load = cpu_load_receiver.get().await;
        let imu = imu_receiver.get().await;

        let state = Telemetry {
            drone_state,
            sequence_number: sequence_count,
            sensors,
            pilot_command,
            cpu_load,
            imu,
        };

        telemetry::set(state);
    }
}
