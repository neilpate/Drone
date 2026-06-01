use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct TelemetryState {
    pub count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postcard_round_trip() {
        let original = TelemetryState { count: 999 };
        let mut buf = [0u8; 16];
        let bytes = postcard::to_slice(&original, &mut buf).unwrap();
        let decoded: TelemetryState = postcard::from_bytes(bytes).unwrap();
        assert_eq!(original, decoded);
    }
}
