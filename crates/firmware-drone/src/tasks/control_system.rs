use firmware_drone_core::{control_system::Controller, filter};
use firmware_types::{AngularRate, ControlMode, ControllerDemand};

use crate::signals::{
    attitude, control_mode_update, control_system_parameter_update, controller_demand, imu_data,
    pilot_command,
};

#[embassy_executor::task]
pub async fn control_system() -> ! {
    defmt::info!("control system task: started");

    let mut attitude_receiver = attitude::subscribe();
    let mut pilot_command_receiver = pilot_command::subscribe();
    let mut imu_receiver = imu_data::subscribe();
    let mut control_mode_receiver = control_mode_update::subscribe();
    let mut parameters_update_receiver = control_system_parameter_update::subscribe();

    const CUTOFF_FREQUENCY_HZ: f32 = 100.0; // Cutoff frequency for the low-pass filter
    const SAMPLE_PERIOD_S: f32 = 0.001; // Sample period in seconds (1 ms)

    let mut gx_filtered = filter::Filter::new(CUTOFF_FREQUENCY_HZ, SAMPLE_PERIOD_S);
    let mut gy_filtered = filter::Filter::new(CUTOFF_FREQUENCY_HZ, SAMPLE_PERIOD_S);
    let mut gz_filtered = filter::Filter::new(CUTOFF_FREQUENCY_HZ, SAMPLE_PERIOD_S);

    let mut control_system = Controller::new();

    loop {
        let attitude = attitude_receiver.changed().await;
        let pilot_command = pilot_command_receiver.get().await;
        let mut imu_data = imu_receiver.get().await;
        let control_mode = control_mode_receiver.get().await;

        let parameters = parameters_update_receiver.get().await;

        // defmt::info!("control system: mode={:?}", control_mode);

        // Filter the gyro signals
        let gx = gx_filtered.update(imu_data.angular_rate_x.as_degrees_per_second());
        let gy = gy_filtered.update(imu_data.angular_rate_y.as_degrees_per_second());
        let gz = gz_filtered.update(imu_data.angular_rate_z.as_degrees_per_second());
        imu_data.angular_rate_x = AngularRate::from_degrees_per_second(gx);
        imu_data.angular_rate_y = AngularRate::from_degrees_per_second(gy);
        imu_data.angular_rate_z = AngularRate::from_degrees_per_second(gz);

        imu_data::set_processed(imu_data);

        match control_mode {
            ControlMode::Manual => {
                // In manual mode, the controller demand is simply the pilot command.

                let controller_demand_manual = ControllerDemand {
                    throttle: pilot_command.throttle,
                    roll: pilot_command.roll,
                    pitch: pilot_command.pitch,
                    yaw: pilot_command.yaw,
                };

                controller_demand::set(controller_demand_manual);
                continue;
            }
            ControlMode::Stabilized => {
                // In stabilised mode, we use the control system to generate a controller demand
                // from the pilot command and the current attitude.

                let demand = control_system.update(
                    parameters,
                    pilot_command,
                    attitude,
                    imu_data,
                    SAMPLE_PERIOD_S,
                );
                controller_demand::set(demand);
            }
        }
    }
}
