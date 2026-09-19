use crate::board;
use crate::signals::{control_system_parameter_update, save_config, status};

use firmware_types::DroneState;

#[embassy_executor::task]
pub async fn config_manager(mut config_storage: board::ConfigStorage) -> ! {
    defmt::info!("config manager task: started");

    let mut parameters_update_receiver = control_system_parameter_update::subscribe();
    let mut status_receiver = status::subscribe();

    let initial_value = config_storage.load().await;
    defmt::info!("initials parameters: {:?}", &initial_value);

    control_system_parameter_update::set(initial_value.unwrap_or_default()); // On a fresh board there will not be any parameters in the flash, so fall back to default

    loop {
        // wait for save config signal
        save_config::wait_on_signal().await;
        defmt::info!("save config requested");

        // signal received, ok to proceed as long as the drone is disarmed
        let status = status_receiver.get().await;

        match status {
            DroneState::Disarmed => {
                // Safe to proceed with saving the configuration

                // Retrieve the latest control system parameters before saving
                let live_parameters = parameters_update_receiver.get().await;

                // Attempt to save the configuration
                match config_storage.save(&live_parameters).await {
                    Ok(_) => {
                        defmt::info!("config saved successfully");
                        let newly_saved_parameters =
                            config_storage.load().await.unwrap_or_default();
                        defmt::info!("newly saved parameters: {:?}", &newly_saved_parameters);
                    }
                    Err(_) => defmt::error!("failed to save config"),
                }
            }

            _ => {
                defmt::info!(
                    "cannot save config: status needs to be disarmed (was {:?}",
                    status
                );
            }
        }
    }
}
