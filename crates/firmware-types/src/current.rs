use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, MaxSize, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Current(f32);

impl Current {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let current = u16::from_be_bytes([bytes[0], bytes[1]]) as f32 / 100.0;
        Self(current)
    }

    pub fn as_amps(self) -> f32 {
        self.0
    }

    pub fn zero() -> Self {
        Self(0.0)
    }
}
