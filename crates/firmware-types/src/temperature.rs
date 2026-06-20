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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn celsius_round_trips() {
        assert_eq!(Temperature::from_celsius(21.5).as_celsius(), 21.5);
    }

    #[test]
    fn preserves_negative() {
        assert_eq!(Temperature::from_celsius(-40.0).as_celsius(), -40.0);
    }

    #[test]
    fn postcard_round_trip() {
        let original = Temperature::from_celsius(36.6);
        let mut buf = [0u8; 16];
        let bytes = postcard::to_slice(&original, &mut buf).unwrap();
        let decoded: Temperature = postcard::from_bytes(bytes).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn deserialize_from_raw_f32() {
        // A bare f32 on the wire decodes straight into a Temperature.
        let mut buf = [0u8; 8];
        let bytes = postcard::to_slice(&12.25f32, &mut buf).unwrap();
        let decoded: Temperature = postcard::from_bytes(bytes).unwrap();
        assert_eq!(decoded.as_celsius(), 12.25);
    }
}
