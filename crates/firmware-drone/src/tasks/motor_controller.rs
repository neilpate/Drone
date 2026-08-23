use embassy_futures::select::{Either, select};
use embassy_time::{Duration, Ticker};

use crate::board;
use crate::signals::motor_command;

const RESEND_PERIOD: Duration = Duration::from_millis(10); // 10ms is the same as the remote link period, so that the motors are always being commanded at the same rate as the remote link is sending commands

#[embassy_executor::task]
pub async fn motor_controller(mut motors: board::Motors) -> ! {
    defmt::info!("motor_controller task: started");

    let mut motor_command_receiver = motor_command::subscribe();

    let mut ticker = Ticker::every(RESEND_PERIOD);

    let mut last_motor_command = motor_command_receiver.get().await;

    loop {
        // Wait either for a new MotorCommand to arrive, or for the timeout to elapse.
        let event = select(motor_command_receiver.changed(), ticker.next()).await;
        match event {
            Either::First(_) => {
                let motor_command = motor_command_receiver.get().await;
                motors.set_all_motors(motor_command);
                last_motor_command = motor_command;
            }
            Either::Second(_) => {
                motors.set_all_motors(last_motor_command); // re-send the last command to keep the motors running and also ensure telemetry is requested from the ESCs
            }
        }
    }
}
