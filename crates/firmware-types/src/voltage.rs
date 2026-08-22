use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, MaxSize, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Voltage(f32);

impl Voltage {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let voltage = u16::from_be_bytes([bytes[0], bytes[1]]) as f32 / 100.0;
        Self(voltage)
    }

    pub fn as_volts(self) -> f32 {
        self.0
    }
}
