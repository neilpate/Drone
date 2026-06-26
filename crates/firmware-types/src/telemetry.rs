use crate::{DroneState, PilotCommand, Sensors};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Telemetry {
    pub sequence_number: u32,
    pub sensors: Sensors,
    pub drone_state: DroneState,
    pub pilot_command: PilotCommand,
    pub cpu_load: crate::CpuLoad,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Pitch, Roll, Temperature, Throttle, Yaw};

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
            cpu_load: crate::CpuLoad::from_percentage(50.0),
        };
        let mut buf = [0u8; 32];
        let bytes = postcard::to_slice(&original, &mut buf).unwrap();
        let decoded: Telemetry = postcard::from_bytes(bytes).unwrap();
        assert_eq!(original, decoded);
    }
}
