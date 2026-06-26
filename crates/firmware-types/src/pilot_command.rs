use serde::{Deserialize, Serialize};

use crate::{Pitch, Roll, Throttle, Yaw};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct PilotCommand {
    pub sequence_count: u32,
    pub throttle: Throttle,
    pub roll: Roll,
    pub pitch: Pitch,
    pub yaw: Yaw,
}

impl Default for PilotCommand {
    /// Neutral, fail-safe command: zero throttle, centred sticks. This is the
    /// value to publish at startup and to fall back to if no command has been
    /// received, so a missing or dropped command holds the craft safe rather
    /// than acting on garbage.
    fn default() -> Self {
        Self {
            sequence_count: 0,
            throttle: Throttle::from_normalised(0.0),
            roll: Roll::from_normalised(0.0),
            pitch: Pitch::from_normalised(0.0),
            yaw: Yaw::from_normalised(0.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postcard_round_trip() {
        let original = PilotCommand {
            sequence_count: 12_345,
            throttle: Throttle::from_normalised(0.75),
            roll: Roll::from_normalised(-0.5),
            pitch: Pitch::from_normalised(0.25),
            yaw: Yaw::from_normalised(-0.125),
        };
        let mut buf = [0u8; 32];
        let bytes = postcard::to_slice(&original, &mut buf).unwrap();
        let decoded: PilotCommand = postcard::from_bytes(bytes).unwrap();
        assert_eq!(original, decoded);
    }
}
