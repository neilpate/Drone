use embassy_futures::select::{Either, select};
use embassy_time::{Duration, Ticker};
use firmware_drone_core::supervisor_core::{Event, Supervisor, TICK_PERIOD_MS};
use firmware_types::{DroneState, MotorCommand};

use crate::signals::{controller_demand, motor_command, pilot_command, status};

const TIMEOUT_PERIOD: Duration = Duration::from_millis(TICK_PERIOD_MS);

#[embassy_executor::task]
pub async fn supervisor() -> ! {
    defmt::info!("supervisor task: started");
    status::set(DroneState::Initialising);

    let mut ticker = Ticker::every(TIMEOUT_PERIOD);

    let mut pilot_command_receiver = pilot_command::subscribe();

    let mut controller_demand_receiver = controller_demand::subscribe();

    let mut supervisor = Supervisor::new();

    let mut prev_superviser_state = supervisor.state;

    defmt::info!(
        "supervisor task: state changed: {:?}",
        prev_superviser_state
    );

    loop {
        // Wait either for a new PilotCommand to arrive, or for the timeout to elapse.
        let event = select(pilot_command_receiver.changed(), ticker.next()).await;

        let controller_demand = controller_demand_receiver.get().await;
        let output = match event {
            Either::First(cmd) => supervisor.step(Event::Command(cmd), controller_demand),
            Either::Second(_) => supervisor.step(Event::Tick, controller_demand),
        };

        // Just for safety, assert that the motor command is zero when disarmed. This should always be true, but if it ever fails, it's a serious safety issue.
        if output.state == DroneState::Disarmed {
            debug_assert!(
                output.motor_command == MotorCommand::ZERO,
                "supervisor task: motor command must be zero while disarmed"
            );
        }

        if supervisor.state != prev_superviser_state {
            prev_superviser_state = supervisor.state;
            defmt::info!(
                "supervisor task: state changed: {:?}",
                prev_superviser_state
            );
        }

        prev_superviser_state = supervisor.state;

        status::set(output.state);
        motor_command::set(output.motor_command);
    }
}
