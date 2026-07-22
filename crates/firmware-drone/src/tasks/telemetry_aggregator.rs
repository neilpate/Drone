use embassy_time::{Duration, Ticker};
use firmware_types::Telemetry;

use crate::signals::{
    attitude, controller_demand, cpu_load, motor_command, pilot_command, sensors, status, telemetry,
};

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

        let state = Telemetry {
            drone_state,
            sequence_number: sequence_count,
            sensors,
            pilot_command,
            attitude,
            cpu_load,
            controller_demand,
            motor_command,
        };

        telemetry::set(state);
    }
}
