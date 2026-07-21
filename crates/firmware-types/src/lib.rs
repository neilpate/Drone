#![cfg_attr(not(test), no_std)]

mod acceleration;
mod angular_rate;
mod attitude;
mod controller_demand;
mod cpu_load;
mod drone_state;
mod groundstation_command;
mod imu_data;
mod motor_command;
mod pilot_command;
mod pitch_angle;
mod pitch_command;
mod remote_state;
mod roll_angle;
mod roll_command;
mod sensors;
mod telemetry;
mod temperature;
mod throttle_command;
mod yaw_angle;
mod yaw_command;

pub use acceleration::Acceleration;
pub use angular_rate::AngularRate;
pub use attitude::Attitude;
pub use controller_demand::ControllerDemand;
pub use cpu_load::CpuLoad;
pub use drone_state::DroneState;
pub use groundstation_command::GroundstationCommand;
pub use imu_data::ImuData;
pub use motor_command::MotorCommand;
pub use pilot_command::PilotCommand;
pub use pitch_angle::PitchAngle;
pub use pitch_command::PitchCommand;
pub use remote_state::RemoteState;
pub use roll_angle::RollAngle;
pub use roll_command::RollCommand;
pub use sensors::Sensors;
pub use telemetry::Telemetry;
pub use temperature::Temperature;
pub use throttle_command::ThrottleCommand;
pub use yaw_angle::YawAngle;
pub use yaw_command::YawCommand;

pub use groundstation_command::FRAME_MAX_SIZE_BYTES as GROUNDSTATION_COMMAND_FRAME_MAX_SIZE_BYTES;
pub use telemetry::FRAME_MAX_SIZE_BYTES as TELEMETRY_FRAME_MAX_SIZE_BYTES;
