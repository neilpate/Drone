use core::ops::Sub;
use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Copy, Debug, Default, PartialEq, MaxSize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Acceleration(f32);

impl Acceleration {
    pub fn from_g(g: f32) -> Self {
        Self(g)
    }

    pub fn as_g(self) -> f32 {
        self.0
    }
}

impl Sub<Acceleration> for Acceleration {
    type Output = Self;

    fn sub(self, other: Acceleration) -> Self::Output {
        Acceleration(self.0 - other.0)
    }
}

#[test]
fn sub_subtracts_g() {
    assert_eq!(
        (Acceleration::from_g(0.5) - Acceleration::from_g(0.2)).as_g(),
        0.3
    );
}
