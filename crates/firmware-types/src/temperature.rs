use serde::{Deserialize, Serialize};

#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Temperature(f32);

impl<'de> Deserialize<'de> for Temperature {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        f32::deserialize(deserializer).map(Self::from_celsius)
    }
}

impl Temperature {
    pub fn from_celsius(c: f32) -> Self {
        Self(c)
    }

    pub fn as_celsius(self) -> f32 {
        self.0
    }
}
