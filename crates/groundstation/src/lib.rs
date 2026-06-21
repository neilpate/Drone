//! Pure, host-testable helpers for the ground station.
//!
//! The GUI event loop, serial thread and gamepad polling live in `main.rs`.
//! Anything here is free of egui / serialport / gilrs I/O so it can be unit
//! tested without a window, a port or a gamepad attached.

use firmware_types::{DroneState, GroundstationCommand};

/// Numeric code for a drone state, used as the y-value of the drone-state
/// time series in the plot. Distinct, ordered values so the trace steps
/// cleanly between states.
pub fn drone_state_code(state: DroneState) -> f64 {
    match state {
        DroneState::Initialising => 0.0,
        DroneState::Armed => 1.0,
        DroneState::Degraded => 2.0,
        DroneState::Fault => 3.0,
    }
}

/// Map a raw gamepad trigger reading to a throttle fraction.
///
/// gilrs reports a trigger as roughly `0.0..=1.0`, but can momentarily report
/// slightly out-of-range values, so the reading is clamped before use.
pub fn trigger_to_throttle(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

/// Map a raw gamepad stick-axis reading to a normalised deflection.
///
/// gilrs reports a stick axis as roughly `-1.0..=1.0`, but can momentarily
/// overshoot, so the reading is clamped before use.
pub fn stick_to_deflection(value: f32) -> f32 {
    value.clamp(-1.0, 1.0)
}

/// Serialise a `GroundstationCommand` into a postcard + COBS framed buffer,
/// returning the framed bytes written into `buf`.
pub fn encode_command(command: GroundstationCommand, buf: &mut [u8]) -> postcard::Result<&[u8]> {
    postcard::to_slice_cobs(&command, buf).map(|framed| &framed[..])
}

#[cfg(test)]
mod tests {
    use super::*;
    use firmware_types::{Pitch, Roll, Throttle, Yaw};
    use postcard::accumulator::{CobsAccumulator, FeedResult};

    #[test]
    fn drone_state_codes_are_distinct_and_ordered() {
        assert_eq!(drone_state_code(DroneState::Initialising), 0.0);
        assert_eq!(drone_state_code(DroneState::Armed), 1.0);
        assert_eq!(drone_state_code(DroneState::Degraded), 2.0);
        assert_eq!(drone_state_code(DroneState::Fault), 3.0);
    }

    #[test]
    fn trigger_clamps_below_zero() {
        assert_eq!(trigger_to_throttle(-0.2), 0.0);
    }

    #[test]
    fn trigger_clamps_above_one() {
        assert_eq!(trigger_to_throttle(1.4), 1.0);
    }

    #[test]
    fn trigger_passes_through_in_range() {
        assert_eq!(trigger_to_throttle(0.5), 0.5);
    }

    #[test]
    fn stick_clamps_below_minus_one() {
        assert_eq!(stick_to_deflection(-1.4), -1.0);
    }

    #[test]
    fn stick_clamps_above_one() {
        assert_eq!(stick_to_deflection(1.4), 1.0);
    }

    #[test]
    fn stick_passes_through_in_range() {
        assert_eq!(stick_to_deflection(-0.5), -0.5);
    }

    #[test]
    fn encoded_command_round_trips_through_the_accumulator() {
        let mut buf = [0u8; 32];
        let command = GroundstationCommand {
            throttle: Throttle::from_normalised(0.5),
            roll: Roll::from_normalised(-0.5),
            pitch: Pitch::from_normalised(0.25),
            yaw: Yaw::from_normalised(-0.125),
        };
        let framed = encode_command(command, &mut buf).unwrap();

        // Decode the frame the same way the firmware does, to prove the
        // groundstation's framing matches the wire format on the other end.
        let mut cobs: CobsAccumulator<64> = CobsAccumulator::new();
        match cobs.feed::<GroundstationCommand>(framed) {
            FeedResult::Success { data, .. } => {
                assert_eq!(data, command);
            }
            _ => panic!("framed command did not decode"),
        }
    }
}
