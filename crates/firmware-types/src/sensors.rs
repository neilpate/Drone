use serde::{Deserialize, Serialize};

use crate::Temperature;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Sensors {
    pub temperature: Temperature,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postcard_round_trip() {
        let original = Sensors {
            temperature: Temperature::from_celsius(36.6),
        };
        let mut buf = [0u8; 16];
        let bytes = postcard::to_slice(&original, &mut buf).unwrap();
        let decoded: Sensors = postcard::from_bytes(bytes).unwrap();
        assert_eq!(original, decoded);
    }
}
