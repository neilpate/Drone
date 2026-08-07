use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, MaxSize)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ControlSystemParameters {
    pub kp_roll: f32,
    pub kd_roll: f32,
    pub kp_pitch: f32,
    pub kd_pitch: f32,
    pub kp_yaw: f32,
    pub kd_yaw: f32,

    pub max_tilt_degrees: f32,
    pub max_tilt_rate_degrees_per_second: f32,
    pub max_yaw_rate_degrees_per_second: f32,
}

impl ControlSystemParameters {}

impl Default for ControlSystemParameters {
    fn default() -> Self {
        Self {
            kp_roll: 0.2,
            kd_roll: 0.01,
            kp_pitch: 0.2,
            kd_pitch: 0.01,
            kp_yaw: 0.3,
            kd_yaw: 0.0,
            max_tilt_degrees: 25.0,
            max_tilt_rate_degrees_per_second: 400.0,
            max_yaw_rate_degrees_per_second: 180.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postcard_round_trip() {
        let original = ControlSystemParameters {
            kp_roll: 0.4,
            kd_roll: 0.0,
            kp_pitch: 0.3,
            kd_pitch: 0.0,
            kp_yaw: 0.0,
            kd_yaw: 0.0,
            max_tilt_degrees: 30.0,
            max_tilt_rate_degrees_per_second: 20.0,
            max_yaw_rate_degrees_per_second: 20.0,
        };
        let mut buf = [0u8; ControlSystemParameters::POSTCARD_MAX_SIZE];
        let bytes = postcard::to_slice(&original, &mut buf).unwrap();
        let decoded: ControlSystemParameters = postcard::from_bytes(bytes).unwrap();
        assert_eq!(original, decoded);
    }
}
