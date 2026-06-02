use embassy_futures::select::{Either, select};
use embassy_time::{Duration, Ticker};

use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Watch};

pub use firmware_drone_core::supervisor_core::SystemState;
use firmware_types::Throttle;

use crate::tasks::pilot_command;

const TIMEOUT_PERIOD: Duration = Duration::from_millis(10);

const MAX_SUBSCRIBERS: usize = 8;

static STATUS: Watch<CriticalSectionRawMutex, SystemState, MAX_SUBSCRIBERS> = Watch::new();

pub type StatusReceiver =
    embassy_sync::watch::Receiver<'static, CriticalSectionRawMutex, SystemState, MAX_SUBSCRIBERS>;

pub fn subscribe() -> StatusReceiver {
    STATUS.receiver().unwrap()
}

fn set(s: SystemState) {
    STATUS.sender().send(s);
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct MotorCommand {
    pub throttle: Throttle,
}

static MOTOR_COMMAND: Watch<CriticalSectionRawMutex, MotorCommand, MAX_SUBSCRIBERS> = Watch::new();

pub type MotorCommandReceiver =
    embassy_sync::watch::Receiver<'static, CriticalSectionRawMutex, MotorCommand, MAX_SUBSCRIBERS>;

pub fn subscribe_motor_command() -> MotorCommandReceiver {
    MOTOR_COMMAND.receiver().unwrap()
}

pub fn set_motor_command(motor_command: MotorCommand) {
    MOTOR_COMMAND.sender().send(motor_command);
}

#[embassy_executor::task]
pub async fn supervise() -> ! {
    defmt::info!("supervisor task: started");
    set(SystemState::Initialising);

    let mut ticker = Ticker::every(TIMEOUT_PERIOD);

    let mut pilot_command_receiver = pilot_command::subscribe();

    let mut motor_command = MotorCommand {
        throttle: Throttle::ZERO,
    };

    let mut ticks_without_command = 0;

    loop {
        // Wait either for a new PilotCommand to arrive, or for the timeout to elapse.

        let event = select(pilot_command_receiver.changed(), ticker.next()).await;

        match event {
            Either::First(pilot_command) => {
                motor_command.throttle = pilot_command.throttle;
                set_motor_command(motor_command);
                ticks_without_command = 0;
            }

            Either::Second(_) => {
                ticks_without_command += 1;

                if ticks_without_command > 10 {
                    defmt::warn!(
                        "supervisor: no pilot command received for 100ms, entering degraded mode"
                    );
                    motor_command.throttle = Throttle::ZERO;
                    set_motor_command(motor_command);
                }
            }
        }
    }
}
