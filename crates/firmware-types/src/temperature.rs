use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, MaxSize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Temperature(f32);

impl Temperature {
    pub fn from_celsius(c: f32) -> Self {
        Self(c)
    }

    pub fn as_celsius(self) -> f32 {
        self.0
    }
}
