use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, MaxSize, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ConsumedCurrent(u16);

impl ConsumedCurrent {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let consumed_current = u16::from_be_bytes([bytes[0], bytes[1]]);
        Self(consumed_current)
    }

    pub fn as_milliamphours(self) -> u16 {
        self.0
    }
}
