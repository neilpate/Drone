use crate::ImuData;
use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

use crate::Temperature;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, MaxSize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Sensors {
    pub temperature: Temperature,
    pub imu: ImuData,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postcard_round_trip() {
        let original = Sensors {
            temperature: Temperature::from_celsius(36.6),
            imu: ImuData::default(),
        };
        let mut buf = [0u8; Sensors::POSTCARD_MAX_SIZE];
        let bytes = postcard::to_slice(&original, &mut buf).unwrap();
        let decoded: Sensors = postcard::from_bytes(bytes).unwrap();
        assert_eq!(original, decoded);
    }
}
