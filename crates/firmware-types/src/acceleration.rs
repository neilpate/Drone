use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, MaxSize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Acceleration(f32);

impl Acceleration {
    pub fn from_g(g: f32) -> Self {
        Self(g)
    }

    pub fn as_g(self) -> f32 {
        self.0
    }
}
