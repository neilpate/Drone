use crate::board;
use crate::tasks::pilot_command;

#[embassy_executor::task]
pub async fn motor_controller(mut motors: board::Motors) -> ! {
    defmt::info!("motor_controller task: started");

    let mut pilot_command_receiver = pilot_command::subscribe();

    motors.enable();

    loop {
        let pilot_command = pilot_command_receiver.changed().await;

        defmt::debug!("received pilot command: {}", pilot_command.throttle);

        motors.set_throttle(0, pilot_command.throttle);
    }
}
