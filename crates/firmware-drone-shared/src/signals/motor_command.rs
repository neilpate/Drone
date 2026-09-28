use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Watch};
use firmware_types::MotorCommand;

const MAX_SUBSCRIBERS: usize = 8;

static MOTOR_COMMAND: Watch<CriticalSectionRawMutex, MotorCommand, MAX_SUBSCRIBERS> = Watch::new();

pub type Receiver =
    embassy_sync::watch::Receiver<'static, CriticalSectionRawMutex, MotorCommand, MAX_SUBSCRIBERS>;

pub fn subscribe() -> Receiver {
    MOTOR_COMMAND.receiver().unwrap()
}

pub fn set(motor_command: MotorCommand) {
    MOTOR_COMMAND.sender().send(motor_command);
}
