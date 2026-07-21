use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Copy, Debug, Default, PartialEq, MaxSize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct YawAngle(f32);

impl YawAngle {
    /// Construct from degrees, scrubbing NaN to 0.0 so a degenerate estimate
    /// can never propagate into the control path. Mirrors the NaN-scrub on the
    /// command newtypes (ADR 0016).
    pub fn from_degrees(deg: f32) -> Self {
        Self(if deg.is_nan() { 0.0 } else { deg })
    }

    pub fn as_degrees(self) -> f32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nan_scrubs_to_zero() {
        assert_eq!(YawAngle::from_degrees(f32::NAN).as_degrees(), 0.0);
    }

    #[test]
    fn finite_passes_through() {
        assert_eq!(YawAngle::from_degrees(-30.0).as_degrees(), -30.0);
    }
}
