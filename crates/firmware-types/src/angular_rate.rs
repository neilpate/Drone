use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Copy, Debug, Default, PartialEq, MaxSize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct AngularRate(f32);

impl AngularRate {
    pub fn from_degrees_per_second(deg: f32) -> Self {
        Self(deg)
    }

    pub fn as_degrees_per_second(self) -> f32 {
        self.0
    }
}
