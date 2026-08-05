use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

use crate::{Command, PilotCommand};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, MaxSize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct RadioMessage {
    pub sequence_count: u32,
    pub command: Command,
}

impl RadioMessage {
    /// Neutral, fail-safe command: zero throttle, centred sticks, sequence 0.
    /// Publish this at startup and fall back to it when no command has been
    /// received, so a missing or dropped command holds the craft safe rather
    /// than acting on garbage.
    pub const ZERO: Self = Self {
        sequence_count: 0,
        command: Command::PilotCommand(PilotCommand::ZERO),
    };
}

impl Default for RadioMessage {
    fn default() -> Self {
        Self::ZERO
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PitchCommand, RollCommand, ThrottleCommand, YawCommand};

    #[test]
    fn postcard_round_trip() {
        let original = RadioMessage {
            sequence_count: 12_345,
            command: Command::PilotCommand(PilotCommand {
                throttle: ThrottleCommand::from_normalised(0.75),
                roll: RollCommand::from_normalised(-0.5),
                pitch: PitchCommand::from_normalised(0.25),
                yaw: YawCommand::from_normalised(-0.125),
            }),
        };
        let mut buf = [0u8; RadioMessage::POSTCARD_MAX_SIZE];
        let bytes = postcard::to_slice(&original, &mut buf).unwrap();
        let decoded: RadioMessage = postcard::from_bytes(bytes).unwrap();
        assert_eq!(original, decoded);
    }
}
