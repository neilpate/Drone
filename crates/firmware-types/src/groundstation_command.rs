use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

use crate::control_mode::ControlMode;
use crate::pitch_command::PitchCommand;
use crate::roll_command::RollCommand;
use crate::throttle_command::ThrottleCommand;
use crate::yaw_command::YawCommand;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, MaxSize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct GroundstationCommand {
    pub throttle: ThrottleCommand,
    pub roll: RollCommand,
    pub pitch: PitchCommand,
    pub yaw: YawCommand,
    pub control_mode: ControlMode,
}

// The maximum size of a `GroundstationCommand` frame, in bytes, when serialized with `postcard`.
pub const FRAME_MAX_SIZE_BYTES: usize =
    GroundstationCommand::POSTCARD_MAX_SIZE + GroundstationCommand::POSTCARD_MAX_SIZE / 254 + 2;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postcard_round_trip() {
        let original = GroundstationCommand {
            throttle: ThrottleCommand::from_normalised(0.75),
            roll: RollCommand::from_normalised(-0.5),
            pitch: PitchCommand::from_normalised(0.25),
            yaw: YawCommand::from_normalised(-0.125),
            control_mode: ControlMode::Manual,
        };
        let mut buf = [0u8; 32];
        let bytes = postcard::to_slice(&original, &mut buf).unwrap();
        let decoded: GroundstationCommand = postcard::from_bytes(bytes).unwrap();
        assert_eq!(original, decoded);
    }
}
