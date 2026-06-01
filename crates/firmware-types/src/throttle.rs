use serde::{Deserialize, Serialize};

#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Throttle(f32);

impl<'de> Deserialize<'de> for Throttle {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        f32::deserialize(deserializer).map(Self::from_normalised)
    }
}

impl Throttle {
    pub const ZERO: Self = Self(0.0);
    pub const MAX: Self = Self(1.0);

    pub fn from_normalised(n: f32) -> Self {
        Self(if n.is_nan() { 0.0 } else { n.clamp(0.0, 1.0) })
    }

    pub fn as_normalised(self) -> f32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_above_one() {
        assert_eq!(Throttle::from_normalised(1.5).as_normalised(), 1.0);
    }

    #[test]
    fn clamps_below_zero() {
        assert_eq!(Throttle::from_normalised(-0.5).as_normalised(), 0.0);
    }

    #[test]
    fn nan_becomes_zero() {
        assert_eq!(Throttle::from_normalised(f32::NAN).as_normalised(), 0.0);
    }

    #[test]
    fn constants() {
        assert_eq!(Throttle::ZERO.as_normalised(), 0.0);
        assert_eq!(Throttle::MAX.as_normalised(), 1.0);
    }

    #[test]
    fn postcard_round_trip() {
        let original = Throttle::from_normalised(0.42);
        let mut buf = [0u8; 16];
        let bytes = postcard::to_slice(&original, &mut buf).unwrap();
        let decoded: Throttle = postcard::from_bytes(bytes).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn deserialize_clamps_garbage() {
        // 4 bytes that decode as a huge f32 (the actual bug from the bench).
        let garbage_f32: f32 = 2_071_499_600_000.0;
        let mut buf = [0u8; 8];
        let bytes = postcard::to_slice(&garbage_f32, &mut buf).unwrap();
        let decoded: Throttle = postcard::from_bytes(bytes).unwrap();
        assert_eq!(decoded.as_normalised(), 1.0);
    }

    #[test]
    fn deserialize_scrubs_nan() {
        let mut buf = [0u8; 8];
        let bytes = postcard::to_slice(&f32::NAN, &mut buf).unwrap();
        let decoded: Throttle = postcard::from_bytes(bytes).unwrap();
        assert_eq!(decoded.as_normalised(), 0.0);
    }
}
