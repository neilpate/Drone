use core::ops::Mul;

use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

/// Normalised roll stick deflection, `-1.0..=1.0` (centre `0.0`), dimensionless.
///
/// Intentionally parallel with [`Pitch`](crate::Pitch) and [`Yaw`](crate::Yaw):
/// the three are structurally identical hand-written newtypes (ADR 0016 allows a
/// macro here, but we keep them separate for grep-ability). Keep all three in
/// sync — a change to one usually means the same change to the other two.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, MaxSize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct RollCommand(f32);

impl<'de> Deserialize<'de> for RollCommand {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        f32::deserialize(deserializer).map(Self::from_normalised)
    }
}

impl RollCommand {
    pub const ZERO: Self = Self(0.0);
    pub const MAX: Self = Self(1.0);
    pub const MIN: Self = Self(-1.0);

    pub fn from_normalised(n: f32) -> Self {
        Self(if n.is_nan() { 0.0 } else { n.clamp(-1.0, 1.0) })
    }

    pub fn as_normalised(self) -> f32 {
        self.0
    }
}

impl Mul<f32> for RollCommand {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::from_normalised(self.as_normalised() * rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_above_one() {
        assert_eq!(RollCommand::from_normalised(1.5).as_normalised(), 1.0);
    }

    #[test]
    fn no_clamp_within_bounds() {
        assert_eq!(RollCommand::from_normalised(0.5).as_normalised(), 0.5);
        assert_eq!(RollCommand::from_normalised(-0.5).as_normalised(), -0.5);
    }

    #[test]
    fn clamps_below_minus_one() {
        assert_eq!(RollCommand::from_normalised(-1.5).as_normalised(), -1.0);
    }

    #[test]
    fn nan_becomes_zero() {
        assert_eq!(RollCommand::from_normalised(f32::NAN).as_normalised(), 0.0);
    }

    #[test]
    fn constants() {
        assert_eq!(RollCommand::ZERO.as_normalised(), 0.0);
        assert_eq!(RollCommand::MAX.as_normalised(), 1.0);
        assert_eq!(RollCommand::MIN.as_normalised(), -1.0);
    }

    #[test]
    fn postcard_round_trip() {
        let original = RollCommand::from_normalised(0.42);
        let mut buf = [0u8; 16];
        let bytes = postcard::to_slice(&original, &mut buf).unwrap();
        let decoded: RollCommand = postcard::from_bytes(bytes).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn deserialize_clamps_garbage() {
        // 4 bytes that decode as a huge f32 (the actual bug from the bench).
        let garbage_f32: f32 = 2_071_499_600_000.0;
        let mut buf = [0u8; 8];
        let bytes = postcard::to_slice(&garbage_f32, &mut buf).unwrap();
        let decoded: RollCommand = postcard::from_bytes(bytes).unwrap();
        assert_eq!(decoded.as_normalised(), 1.0);
    }

    #[test]
    fn deserialize_scrubs_nan() {
        let mut buf = [0u8; 8];
        let bytes = postcard::to_slice(&f32::NAN, &mut buf).unwrap();
        let decoded: RollCommand = postcard::from_bytes(bytes).unwrap();
        assert_eq!(decoded.as_normalised(), 0.0);
    }

    #[test]
    fn multiply() {
        let roll = RollCommand::from_normalised(0.5);
        let result = roll * 0.5;
        assert_eq!(result.as_normalised(), 0.25);
    }

    #[test]
    fn multiply_clamps_when_overshoot() {
        // 0.6 * 2.0 = 1.2 -> clamped to MAX
        assert_eq!(
            (RollCommand::from_normalised(0.6) * 2.0).as_normalised(),
            1.0
        );
    }

    #[test]
    fn multiply_by_zero_is_zero() {
        assert_eq!((RollCommand::MAX * 0.0).as_normalised(), 0.0);
    }
}
