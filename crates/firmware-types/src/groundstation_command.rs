use serde::{Deserialize, Serialize};

use crate::pitch::Pitch;
use crate::roll::Roll;
use crate::throttle::Throttle;
use crate::yaw::Yaw;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct GroundstationCommand {
    pub throttle: Throttle,
    pub roll: Roll,
    pub pitch: Pitch,
    pub yaw: Yaw,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postcard_round_trip() {
        let original = GroundstationCommand {
            throttle: Throttle::from_normalised(0.75),
            roll: Roll::from_normalised(-0.5),
            pitch: Pitch::from_normalised(0.25),
            yaw: Yaw::from_normalised(-0.125),
        };
        let mut buf = [0u8; 32];
        let bytes = postcard::to_slice(&original, &mut buf).unwrap();
        let decoded: GroundstationCommand = postcard::from_bytes(bytes).unwrap();
        assert_eq!(original, decoded);
    }
}
