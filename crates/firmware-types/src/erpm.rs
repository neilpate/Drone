use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, MaxSize, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ERPM(f32);

impl ERPM {
    /// Magnetic pole pairs (poles / 2). iFlight XING2 1404 is 9N12P: 12 poles
    /// = 6 pole pairs (see hardware/electrical/parts-list.md).
    const POLE_PAIRS: f32 = 6.0;

    pub fn from_bytes(bytes: &[u8]) -> Self {
        // The KISS/BLHeli/AM32 eRPM field carries electrical RPM / 100.
        let erpm = u16::from_be_bytes([bytes[0], bytes[1]]) as f32 * 100.0;
        Self(erpm)
    }

    pub fn as_motor_rpm(self) -> f32 {
        // Mechanical RPM = electrical RPM / pole pairs.
        self.0 / Self::POLE_PAIRS
    }
}
