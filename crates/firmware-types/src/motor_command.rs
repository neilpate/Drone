use serde::{Deserialize, Serialize};

use crate::throttle::Throttle;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct MotorCommand {
    pub motor1: Throttle,
    pub motor2: Throttle,
    pub motor3: Throttle,
    pub motor4: Throttle,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postcard_round_trip() {
        let original = MotorCommand {
            motor1: Throttle::from_normalised(0.75),
            motor2: Throttle::from_normalised(0.75),
            motor3: Throttle::from_normalised(0.75),
            motor4: Throttle::from_normalised(0.75),
        };
        let mut buf = [0u8; 32];
        let bytes = postcard::to_slice(&original, &mut buf).unwrap();
        let decoded: MotorCommand = postcard::from_bytes(bytes).unwrap();
        assert_eq!(original, decoded);
    }
}
