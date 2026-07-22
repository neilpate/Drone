use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

use crate::throttle_command::ThrottleCommand;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, MaxSize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct MotorCommand {
    pub motor1: ThrottleCommand,
    pub motor2: ThrottleCommand,
    pub motor3: ThrottleCommand,
    pub motor4: ThrottleCommand,
}

impl MotorCommand {
    /// All four motors stopped.
    pub const ZERO: Self = Self {
        motor1: ThrottleCommand::ZERO,
        motor2: ThrottleCommand::ZERO,
        motor3: ThrottleCommand::ZERO,
        motor4: ThrottleCommand::ZERO,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postcard_round_trip() {
        let original = MotorCommand {
            motor1: ThrottleCommand::from_normalised(0.75),
            motor2: ThrottleCommand::from_normalised(0.75),
            motor3: ThrottleCommand::from_normalised(0.75),
            motor4: ThrottleCommand::from_normalised(0.75),
        };
        let mut buf = [0u8; 32];
        let bytes = postcard::to_slice(&original, &mut buf).unwrap();
        let decoded: MotorCommand = postcard::from_bytes(bytes).unwrap();
        assert_eq!(original, decoded);
    }
}
