#![cfg_attr(not(test), no_std)]

mod acceleration;
mod angular_rate;
mod cpu_load;
mod drone_state;
mod groundstation_command;
mod imu_data;
mod motor_command;
mod pilot_command;
mod pitch;
mod remote_state;
mod roll;
mod sensors;
mod telemetry;
mod temperature;
mod throttle;
mod yaw;

pub use acceleration::Acceleration;
pub use angular_rate::AngularRate;
pub use cpu_load::CpuLoad;
pub use drone_state::DroneState;
pub use groundstation_command::GroundstationCommand;
pub use imu_data::ImuData;
pub use motor_command::MotorCommand;
pub use pilot_command::PilotCommand;
pub use pitch::Pitch;
pub use remote_state::RemoteState;
pub use roll::Roll;
pub use sensors::Sensors;
pub use telemetry::Telemetry;
pub use temperature::Temperature;
pub use throttle::Throttle;
pub use yaw::Yaw;

pub use groundstation_command::FRAME_MAX_SIZE_BYTES as GROUNDSTATION_COMMAND_FRAME_MAX_SIZE_BYTES;
pub use telemetry::FRAME_MAX_SIZE_BYTES as TELEMETRY_FRAME_MAX_SIZE_BYTES;
