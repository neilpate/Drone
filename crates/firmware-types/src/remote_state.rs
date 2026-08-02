use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, MaxSize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum RemoteState {
    Booting,
    Idle,
    Fault,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postcard_round_trip() {
        let original = RemoteState::Booting;
        let mut buf = [0u8; RemoteState::POSTCARD_MAX_SIZE];
        let bytes = postcard::to_slice(&original, &mut buf).unwrap();
        let decoded: RemoteState = postcard::from_bytes(bytes).unwrap();
        assert_eq!(original, decoded);
    }
}
