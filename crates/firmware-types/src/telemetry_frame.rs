use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

use crate::{TelemetryFrameHighRate, TelemetryFrameLowRate};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, MaxSize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum TelemetryFrame {
    HighRate(TelemetryFrameHighRate),
    LowRate(TelemetryFrameLowRate),
}

// The maximum size of a `TelemetryFrame` frame, in bytes, when serialized with `postcard`.
pub const MAX_SIZE_BYTES: usize =
    TelemetryFrame::POSTCARD_MAX_SIZE + TelemetryFrame::POSTCARD_MAX_SIZE / 254 + 2;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Acceleration, AngularRate, Attitude, BatteryState, ControlMode, ControlSystemParameters,
        ControllerDemand, CpuLoad, DroneState, ESCTelemetry, ImuData, MotorCommand, PilotCommand,
        PitchCommand, RollCommand, Temperature, ThrottleCommand, YawCommand,
    };

    #[test]
    fn high_rate_round_trip() {
        let frame = TelemetryFrame::HighRate(TelemetryFrameHighRate {
            sequence_number: 1,
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
        });
        assert_eq!(frame, round_trip(&frame));
    }

    #[test]
    fn low_rate_round_trip() {
        let frame = TelemetryFrame::LowRate(TelemetryFrameLowRate {
            sequence_number: 2,
            cpu_load: CpuLoad::from_percentage(50.0),
            control_mode: ControlMode::Manual,
            temperature: Temperature::from_celsius(25.0),
            esc_telemetry: ESCTelemetry {
                temperature: Temperature::from_celsius(30.0),
                battery_state: BatteryState::default(),
                motor1_rpm: AngularRate::from_degrees_per_second(0.0),
                motor2_rpm: AngularRate::from_degrees_per_second(0.0),
                motor3_rpm: AngularRate::from_degrees_per_second(0.0),
                motor4_rpm: AngularRate::from_degrees_per_second(0.0),
            },
            control_parameters: ControlSystemParameters::default(),
        });
        assert_eq!(frame, round_trip(&frame));
    }

    fn round_trip(frame: &TelemetryFrame) -> TelemetryFrame {
        let mut buf = [0u8; TelemetryFrame::POSTCARD_MAX_SIZE];
        let bytes = postcard::to_slice(frame, &mut buf).unwrap();
        postcard::from_bytes(bytes).unwrap()
    }
}
