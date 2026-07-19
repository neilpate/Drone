use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

use crate::{Pitch, Roll, Throttle, Yaw};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, MaxSize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct PilotCommand {
    pub sequence_count: u32,
    pub throttle: Throttle,
    pub roll: Roll,
    pub pitch: Pitch,
    pub yaw: Yaw,
}

impl PilotCommand {
    /// Neutral, fail-safe command: zero throttle, centred sticks, sequence 0.
    /// Publish this at startup and fall back to it when no command has been
    /// received, so a missing or dropped command holds the craft safe rather
    /// than acting on garbage.
    pub const ZERO: Self = Self {
        sequence_count: 0,
        throttle: Throttle::ZERO,
        roll: Roll::ZERO,
        pitch: Pitch::ZERO,
        yaw: Yaw::ZERO,
    };
}

impl Default for PilotCommand {
    fn default() -> Self {
        Self::ZERO
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
