use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

use crate::{
    AngularRate, Attitude, ControllerDemand, DroneState, ImuData, MotorCommand, PilotCommand,
};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, MaxSize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct TelemetryFrameHighRate {
    pub sequence_number: u32,
    /// Raw IMU. The accelerometer is carried only here, not repeated on the low-rate frame.
    pub imu: ImuData,
    pub filtered_angular_rate_x: AngularRate,
    pub filtered_angular_rate_y: AngularRate,
    pub filtered_angular_rate_z: AngularRate,
    pub attitude: Attitude,
    /// Also the RTT echo target, so it must stay on the high-rate frame.
    pub pilot_command: PilotCommand,
    pub controller_demand: ControllerDemand,
    pub motor_command: MotorCommand,
    pub drone_state: DroneState,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Acceleration, PitchCommand, RollCommand, ThrottleCommand, YawCommand};

    #[test]
    fn postcard_round_trip() {
        let original = TelemetryFrameHighRate {
            sequence_number: 999,
            imu: ImuData {
                acceleration_x: Acceleration::from_g(0.0),
                acceleration_y: Acceleration::from_g(0.0),
                acceleration_z: Acceleration::from_g(1.0),
                angular_rate_x: AngularRate::from_degrees_per_second(0.0),
                angular_rate_y: AngularRate::from_degrees_per_second(0.0),
                angular_rate_z: AngularRate::from_degrees_per_second(0.0),
            },
            filtered_angular_rate_x: AngularRate::from_degrees_per_second(0.0),
            filtered_angular_rate_y: AngularRate::from_degrees_per_second(0.0),
            filtered_angular_rate_z: AngularRate::from_degrees_per_second(0.0),
            attitude: Attitude::from_degrees(0.0, 0.0),
            pilot_command: PilotCommand {
                throttle: ThrottleCommand::from_normalised(0.5),
                roll: RollCommand::from_normalised(-0.5),
                pitch: PitchCommand::from_normalised(0.25),
                yaw: YawCommand::from_normalised(-0.125),
            },
            controller_demand: ControllerDemand::ZERO,
            motor_command: MotorCommand::ZERO,
            drone_state: DroneState::Armed,
        };
        let mut buf = [0u8; TelemetryFrameHighRate::POSTCARD_MAX_SIZE];
        let bytes = postcard::to_slice(&original, &mut buf).unwrap();
        let decoded: TelemetryFrameHighRate = postcard::from_bytes(bytes).unwrap();
        assert_eq!(original, decoded);
    }
}
