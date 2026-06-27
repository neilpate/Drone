use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

use crate::{CpuLoad, DroneState, ImuData, PilotCommand, Sensors};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, MaxSize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Telemetry {
    pub sequence_number: u32,
    pub sensors: Sensors,
    pub drone_state: DroneState,
    pub pilot_command: PilotCommand,
    pub cpu_load: CpuLoad,
    pub imu: ImuData,
}

// The maximum size of a `Telemetry` frame, in bytes, when serialized with `postcard`.
pub const FRAME_MAX_SIZE_BYTES: usize =
    Telemetry::POSTCARD_MAX_SIZE + Telemetry::POSTCARD_MAX_SIZE / 254 + 2;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Acceleration, AngularRate, Pitch, Roll, Temperature, Throttle, Yaw};

    #[test]
    fn postcard_round_trip() {
        let original = Telemetry {
            sequence_number: 999,
            sensors: Sensors {
                temperature: Temperature::from_celsius(25.0),
            },
            drone_state: DroneState::Armed,
            pilot_command: PilotCommand {
                sequence_count: 7,
                throttle: Throttle::from_normalised(0.5),
                roll: Roll::from_normalised(-0.5),
                pitch: Pitch::from_normalised(0.25),
                yaw: Yaw::from_normalised(-0.125),
            },
            cpu_load: CpuLoad::from_percentage(50.0),
            imu: ImuData {
                acceleration_x: Acceleration::from_g(0.0),
                acceleration_y: Acceleration::from_g(0.0),
                acceleration_z: Acceleration::from_g(0.0),
                angular_rate_x: AngularRate::from_degrees_per_second(0.0),
                angular_rate_y: AngularRate::from_degrees_per_second(0.0),
                angular_rate_z: AngularRate::from_degrees_per_second(0.0),
            },
        };
        let mut buf = [0u8; Telemetry::POSTCARD_MAX_SIZE];
        let bytes = postcard::to_slice(&original, &mut buf).unwrap();
        let decoded: Telemetry = postcard::from_bytes(bytes).unwrap();
        assert_eq!(original, decoded);
    }
}
