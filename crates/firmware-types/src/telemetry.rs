use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

use crate::{Attitude, ControllerDemand, CpuLoad, DroneState, MotorCommand, PilotCommand, Sensors};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, MaxSize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Telemetry {
    pub sequence_number: u32,
    pub sensors: Sensors,
    pub drone_state: DroneState,
    pub pilot_command: PilotCommand,
    pub attitude: Attitude,
    pub cpu_load: CpuLoad,
    pub controller_demand: ControllerDemand,
    pub motor_command: MotorCommand,
}

// The maximum size of a `Telemetry` frame, in bytes, when serialized with `postcard`.
pub const FRAME_MAX_SIZE_BYTES: usize =
    Telemetry::POSTCARD_MAX_SIZE + Telemetry::POSTCARD_MAX_SIZE / 254 + 2;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Acceleration, AngularRate, ControlMode, ImuData, PitchCommand, RollCommand, Temperature,
        ThrottleCommand, YawCommand,
    };

    #[test]
    fn postcard_round_trip() {
        let original = Telemetry {
            sequence_number: 999,
            sensors: Sensors {
                temperature: Temperature::from_celsius(25.0),
                imu: ImuData {
                    acceleration_x: Acceleration::from_g(0.0),
                    acceleration_y: Acceleration::from_g(0.0),
                    acceleration_z: Acceleration::from_g(0.0),
                    angular_rate_x: AngularRate::from_degrees_per_second(0.0),
                    angular_rate_y: AngularRate::from_degrees_per_second(0.0),
                    angular_rate_z: AngularRate::from_degrees_per_second(0.0),
                },
            },
            drone_state: DroneState::Armed,
            pilot_command: PilotCommand {
                sequence_count: 7,
                throttle: ThrottleCommand::from_normalised(0.5),
                roll: RollCommand::from_normalised(-0.5),
                pitch: PitchCommand::from_normalised(0.25),
                yaw: YawCommand::from_normalised(-0.125),
                control_mode: ControlMode::Manual,
            },
            attitude: Attitude::from_degrees(0.0, 0.0),
            cpu_load: CpuLoad::from_percentage(50.0),
            controller_demand: ControllerDemand::ZERO,
            motor_command: MotorCommand::ZERO,
        };
        let mut buf = [0u8; Telemetry::POSTCARD_MAX_SIZE];
        let bytes = postcard::to_slice(&original, &mut buf).unwrap();
        let decoded: Telemetry = postcard::from_bytes(bytes).unwrap();
        assert_eq!(original, decoded);
    }
}
