use crate::board;
use crate::signals::motor_command;

#[embassy_executor::task]
pub async fn motor_controller(mut motors: board::Motors) -> ! {
    defmt::info!("motor_controller task: started");

    let mut motor_command_receiver = motor_command::subscribe();

    motors.enable();

    loop {
        let motor_command = motor_command_receiver.changed().await;

        defmt::debug!("received motor command: {}", motor_command.throttle);

        motors.set_throttle(0, motor_command.throttle);
    }
}
