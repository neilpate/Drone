use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Copy, Debug, Default, PartialEq, MaxSize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct PitchAngle(f32);

impl PitchAngle {
    pub fn from_degrees(deg: f32) -> Self {
        Self(deg)
    }

    pub fn as_degrees(self) -> f32 {
        self.0
    }
}
