use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

use crate::{ControlMode, ControlSystemParameters, PilotCommand};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, MaxSize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Command {
    PilotCommand(PilotCommand),
    ControlSystemParameterUpdate(ControlSystemParameters),
    ControlModeUpdate(ControlMode),
    ResetImuCalibration,
}

// The maximum size of a `Command` frame, in bytes, when serialized with `postcard`.
pub const FRAME_MAX_SIZE_BYTES: usize =
    Command::POSTCARD_MAX_SIZE + Command::POSTCARD_MAX_SIZE / 254 + 2;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PitchCommand, RollCommand, ThrottleCommand, YawCommand};

    #[test]
    fn postcard_round_trip() {
        let original = Command::PilotCommand(PilotCommand {
            throttle: ThrottleCommand::from_normalised(0.75),
            roll: RollCommand::from_normalised(-0.5),
            pitch: PitchCommand::from_normalised(0.25),
            yaw: YawCommand::from_normalised(-0.125),
        });
        let mut buf = [0u8; Command::POSTCARD_MAX_SIZE];
        let bytes = postcard::to_slice(&original, &mut buf).unwrap();
        let decoded: Command = postcard::from_bytes(bytes).unwrap();
        assert_eq!(original, decoded);
    }
}
