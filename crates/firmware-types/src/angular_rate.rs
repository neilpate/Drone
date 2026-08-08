use core::ops::Sub;

use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Copy, Debug, Default, PartialEq, MaxSize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct AngularRate(f32);

impl AngularRate {
    pub fn from_degrees_per_second(deg: f32) -> Self {
        Self(deg)
    }

    pub fn as_degrees_per_second(self) -> f32 {
        self.0
    }
}

impl Sub<AngularRate> for AngularRate {
    type Output = Self;

    fn sub(self, other: AngularRate) -> Self::Output {
        AngularRate(self.0 - other.0)
    }
}

#[test]
fn sub_subtracts_degrees_per_second() {
    assert_eq!(
        (AngularRate::from_degrees_per_second(5.0) - AngularRate::from_degrees_per_second(2.0))
            .as_degrees_per_second(),
        3.0
    );
}
