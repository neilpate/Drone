use crate::{DroneState, PilotCommand, Temperature};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct TelemetryState {
    pub sequence_number: u32,
    pub temperature: Temperature,
    pub drone_state: DroneState,
    pub pilot_command: PilotCommand,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Throttle;

    #[test]
    fn postcard_round_trip() {
        let original = TelemetryState {
            sequence_number: 999,
            temperature: Temperature::from_celsius(25.0),
            drone_state: DroneState::Armed,
            pilot_command: PilotCommand {
                sequence_count: 7,
                throttle: Throttle::from_normalised(0.5),
            },
        };
        let mut buf = [0u8; 16];
        let bytes = postcard::to_slice(&original, &mut buf).unwrap();
        let decoded: TelemetryState = postcard::from_bytes(bytes).unwrap();
        assert_eq!(original, decoded);
    }
}
