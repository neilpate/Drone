use firmware_drone_core::control_system;
use firmware_types::{ControlMode, ControllerDemand};

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
    let mut control_system_parameters_update_receiver =
        control_system_parameter_update::subscribe();

    loop {
        let attitude = attitude_receiver.changed().await;
        let pilot_command = pilot_command_receiver.get().await;
        let imu_data = imu_receiver.get().await;
        let control_mode = control_mode_receiver.get().await;
        let control_system_parameters = control_system_parameters_update_receiver.get().await;

        // defmt::info!("control system: mode={:?}", control_mode);

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
                // defmt::info!("control system: parameters={:?}", control_system_parameters);

                // In stabilised mode, we use the control system to generate a controller demand
                // from the pilot command and the current attitude.
                let demand = control_system::update(
                    pilot_command,
                    attitude,
                    imu_data,
                    control_system_parameters,
                );
                controller_demand::set(demand);
            }
        }
    }
}
