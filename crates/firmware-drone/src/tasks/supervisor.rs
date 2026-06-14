use embassy_futures::select::{Either, select};
use embassy_time::{Duration, Ticker};

pub use firmware_drone_core::supervisor_core::SystemState;
use firmware_drone_core::supervisor_core::{Event, Supervisor};

use firmware_types::{MotorCommand, PilotCommand};

use crate::signals::{motor_command, pilot_command};

use crate::signals::status;

const TIMEOUT_PERIOD: Duration = Duration::from_millis(10);

#[embassy_executor::task]
pub async fn supervisor() -> ! {
    defmt::info!("supervisor task: started");
    status::set(SystemState::Initialising);

    let mut ticker = Ticker::every(TIMEOUT_PERIOD);

    let mut pilot_command_receiver = pilot_command::subscribe();

    let mut supervisor = Supervisor::new();

    loop {
        // Wait either for a new PilotCommand to arrive, or for the timeout to elapse.

        let event = select(pilot_command_receiver.changed(), ticker.next()).await;

        let output = match event {
            Either::First(cmd) => supervisor.step(Event::Command(cmd)),
            Either::Second(_) => supervisor.step(Event::Tick),
        };

        status::set(output.state);
        motor_command::set(MotorCommand {
            throttle: output.throttle,
        });
    }
}
