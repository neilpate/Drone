use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

use crate::{PitchCommand, RollCommand, ThrottleCommand, YawCommand};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, MaxSize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ControllerDemand {
    pub throttle: ThrottleCommand,
    pub roll: RollCommand,
    pub pitch: PitchCommand,
    pub yaw: YawCommand,
}

impl ControllerDemand {
    /// Neutral, fail-safe command: zero throttle, centred sticks. Publish this
    /// at startup and fall back to it when no command has been received, so a
    /// missing or dropped command holds the craft safe rather than acting on
    /// garbage.
    pub const ZERO: Self = Self {
        throttle: ThrottleCommand::ZERO,
        roll: RollCommand::ZERO,
        pitch: PitchCommand::ZERO,
        yaw: YawCommand::ZERO,
    };
}

impl Default for ControllerDemand {
    fn default() -> Self {
        Self::ZERO
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postcard_round_trip() {
        let original = ControllerDemand {
            throttle: ThrottleCommand::from_normalised(0.75),
            roll: RollCommand::from_normalised(-0.5),
            pitch: PitchCommand::from_normalised(0.25),
            yaw: YawCommand::from_normalised(-0.125),
        };
        let mut buf = [0u8; 32];
        let bytes = postcard::to_slice(&original, &mut buf).unwrap();
        let decoded: ControllerDemand = postcard::from_bytes(bytes).unwrap();
        assert_eq!(original, decoded);
    }
}
