use crate::{PitchAngle, RollAngle};
use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Copy, Debug, Default, PartialEq, MaxSize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Attitude {
    pub pitch: PitchAngle,
    pub roll: RollAngle,
}

impl Attitude {
    pub fn from_degrees(pitch_deg: f32, roll_deg: f32) -> Self {
        Self {
            pitch: PitchAngle::from_degrees(pitch_deg),
            roll: RollAngle::from_degrees(roll_deg),
        }
    }

    pub fn as_degrees(self) -> (f32, f32) {
        (self.pitch.as_degrees(), self.roll.as_degrees())
    }
}
