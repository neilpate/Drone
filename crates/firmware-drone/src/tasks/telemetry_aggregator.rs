use crate::signals::{
    attitude, control_mode_update, controller_demand, cpu_load, imu_data, motor_command,
    pilot_command, sensors, status, telemetry,
};
use embassy_time::{Duration, Ticker};
use firmware_types::Telemetry;
use firmware_types::{ImuData, Sensors, Temperature};

#[embassy_executor::task]
pub async fn telemetry_aggregator() -> ! {
    defmt::info!("telemetry aggregator task: started");

    let mut sequence_count: u32 = 0;

    let mut status_receiver = status::subscribe();
    let mut sensors_receiver = sensors::subscribe();
    let mut pilot_command_receiver = pilot_command::subscribe();
    let mut cpu_load_receiver = cpu_load::subscribe();
    let mut attitude_receiver = attitude::subscribe();
    let mut controller_demand_receiver = controller_demand::subscribe();
    let mut motor_command_receiver = motor_command::subscribe();
    let mut imu_processed_receiver = imu_data::subscribe_processed();
    let mut control_mode_receiver = control_mode_update::subscribe();

    let mut ticker = Ticker::every(Duration::from_millis(10));

    loop {
        ticker.next().await;

        sequence_count = sequence_count.wrapping_add(1);

        let drone_state = status_receiver.get().await;
        let sensors = sensors_receiver.get().await;
        let pilot_command = pilot_command_receiver.get().await;
        let cpu_load = cpu_load_receiver.get().await;
        let attitude = attitude_receiver.get().await;
        let controller_demand = controller_demand_receiver.get().await;
        let motor_command = motor_command_receiver.get().await;
        let imu_processed = imu_processed_receiver.get().await;
        let control_mode = control_mode_receiver.get().await;

        let sensors_processed = Sensors {
            temperature: Temperature::from_celsius(25.0),
            imu: ImuData {
                acceleration_x: sensors.imu.acceleration_x,
                acceleration_y: sensors.imu.acceleration_y,
                acceleration_z: sensors.imu.acceleration_z,
                angular_rate_x: imu_processed.angular_rate_x,
                angular_rate_y: imu_processed.angular_rate_y,
                angular_rate_z: imu_processed.angular_rate_z,
            },
        };

        let state = Telemetry {
            drone_state,
            sequence_number: sequence_count,
            sensors,
            sensors_processed,
            pilot_command,
            attitude,
            cpu_load,
            controller_demand,
            motor_command,
            control_mode,
        };

        telemetry::set(state);
    }
}
