use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Clone, Copy, Debug, PartialEq, MaxSize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct CpuLoad(f32);

impl<'de> Deserialize<'de> for CpuLoad {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        f32::deserialize(deserializer).map(Self::from_percentage)
    }
}

impl CpuLoad {
    pub fn from_percentage(n: f32) -> Self {
        Self(if n.is_nan() { 0.0 } else { n.clamp(0.0, 100.0) })
    }

    pub fn as_percentage(self) -> f32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nan_becomes_zero() {
        assert_eq!(CpuLoad::from_percentage(f32::NAN).as_percentage(), 0.0);
    }

    #[test]
    fn postcard_round_trip() {
        let original = CpuLoad::from_percentage(42.0);
        let mut buf = [0u8; 16];
        let bytes = postcard::to_slice(&original, &mut buf).unwrap();
        let decoded: CpuLoad = postcard::from_bytes(bytes).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn deserialize_clamps_garbage() {
        // 4 bytes that decode as a huge f32 (the actual bug from the bench).
        let garbage_f32: f32 = 2_071_499_600_000.0;
        let mut buf = [0u8; 8];
        let bytes = postcard::to_slice(&garbage_f32, &mut buf).unwrap();
        let decoded: CpuLoad = postcard::from_bytes(bytes).unwrap();
        assert_eq!(decoded.as_percentage(), 100.0);
    }

    #[test]
    fn deserialize_scrubs_nan() {
        let mut buf = [0u8; 8];
        let bytes = postcard::to_slice(&f32::NAN, &mut buf).unwrap();
        let decoded: CpuLoad = postcard::from_bytes(bytes).unwrap();
        assert_eq!(decoded.as_percentage(), 0.0);
    }
}
