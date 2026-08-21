//! DShot digital ESC protocol framing (outgoing, no telemetry readback).
//!
//! Pure frame construction, host-tested per [ADR 0007] / [ADR 0015]. Only the
//! 16-bit frame is built here; expanding each bit into a timing-critical PWM
//! pulse-width sequence is board-specific and lives in the board's motor driver.
//!
//! Frame layout (16 bits, transmitted MSB first):
//!
//! ```text
//! [ 11-bit value ][ telemetry ][ 4-bit CRC ]
//! ```
//!
//! `value` is `0` = stop, `1..=47` = special commands, `48..=2047` = throttle.
//! The CRC is the standard (non-inverted) DShot checksum used by Betaflight /
//! BLHeli / AM32: the XOR-fold of the three nibbles of the 12-bit
//! `[value:telemetry]` field. Bidirectional DShot (which inverts the CRC) is a
//! later, separate concern.
//!
//! [ADR 0007]: ../../../doc/decisions/0007-testing-and-ci-strategy.md
//! [ADR 0015]: ../../../doc/decisions/0015-host-testing-no-std-crates.md

/// Motor-stop / disarm command; also the neutral idle frame.
pub const STOP: u16 = 0;

/// Lowest value in the throttle band. Values `1..=47` are special commands.
pub const THROTTLE_MIN: u16 = 48;

/// Highest throttle value (the top of the 11-bit field).
pub const THROTTLE_MAX: u16 = 2047;

/// Mask for the 11-bit value field.
const VALUE_MASK: u16 = 0x7FF;

/// Build a 16-bit DShot frame from an 11-bit `value` (a throttle level or a
/// special command) and the telemetry-request bit. `value` is masked to 11 bits.
pub fn frame(value: u16, request_telemetry: bool) -> u16 {
    // 12-bit payload: the 11-bit value with the telemetry-request bit appended.
    let payload = ((value & VALUE_MASK) << 1) | request_telemetry as u16;
    // CRC: XOR-fold the three nibbles of the payload (non-inverted DShot).
    let crc = (payload ^ (payload >> 4) ^ (payload >> 8)) & 0xF;
    (payload << 4) | crc
}

/// Map a normalised throttle (`0.0..=1.0`) to a throttle-band DShot frame,
/// clamping out-of-range input.
///
/// Note: `0.0` maps to [`THROTTLE_MIN`] — the slowest *spinning* throttle, not a
/// stop. Use `frame(STOP, _)` to actually stop the motor (the disarmed path).
pub fn throttle_frame(normalised: f32, request_telemetry: bool) -> u16 {
    let clamped = normalised.clamp(0.0, 1.0);
    let span = f32::from(THROTTLE_MAX - THROTTLE_MIN);
    // + 0.5 then truncate = round to nearest.
    let value = THROTTLE_MIN + (clamped * span + 0.5) as u16;
    frame(value, request_telemetry)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Known-answer vectors, computed independently from the Betaflight CRC
    /// algorithm (not from this implementation).
    #[test]
    fn known_frames() {
        assert_eq!(frame(0, false), 0x0000);
        assert_eq!(frame(48, false), 0x0606);
        assert_eq!(frame(1046, false), 0x82C6);
        assert_eq!(frame(1046, true), 0x82D7);
        assert_eq!(frame(2047, false), 0xFFEE);
        assert_eq!(frame(2047, true), 0xFFFF);
        assert_eq!(frame(7, false), 0x00EE);
        assert_eq!(frame(1, false), 0x0022);
    }

    /// Bits above the 11-bit value field are ignored.
    #[test]
    fn value_is_masked_to_11_bits() {
        assert_eq!(frame(2047 | 0xF800, false), frame(2047, false));
    }

    /// The telemetry-request bit is frame bit 4 (just above the 4-bit CRC).
    #[test]
    fn telemetry_bit_is_frame_bit_4() {
        assert_eq!(frame(1046, true) & 0x10, 0x10);
        assert_eq!(frame(1046, false) & 0x10, 0x00);
    }

    /// For every value/telemetry combination, the embedded CRC must validate and
    /// the value/telemetry fields must decode back out.
    #[test]
    fn crc_and_fields_round_trip() {
        for request_telemetry in [false, true] {
            for value in 0u16..=THROTTLE_MAX {
                let f = frame(value, request_telemetry);
                let payload = f >> 4;
                let crc = f & 0xF;
                let expected_crc = (payload ^ (payload >> 4) ^ (payload >> 8)) & 0xF;
                assert_eq!(crc, expected_crc, "bad CRC for value={value}");
                assert_eq!(payload >> 1, value & VALUE_MASK, "value mismatch");
                assert_eq!(payload & 1, request_telemetry as u16, "telem mismatch");
            }
        }
    }

    #[test]
    fn throttle_frame_clamps_to_endpoints() {
        assert_eq!(throttle_frame(0.0, false), frame(THROTTLE_MIN, false));
        assert_eq!(throttle_frame(1.0, false), frame(THROTTLE_MAX, false));
        assert_eq!(throttle_frame(-0.5, false), frame(THROTTLE_MIN, false));
        assert_eq!(throttle_frame(2.0, false), frame(THROTTLE_MAX, false));
    }

    /// A normalised sweep must produce monotonically non-decreasing throttle
    /// values landing near the linear midpoint at 0.5.
    #[test]
    fn throttle_frame_is_monotonic() {
        let mut last = 0u16;
        for i in 0..=100 {
            let value = throttle_frame(i as f32 / 100.0, false) >> 5;
            assert!(value >= last, "throttle not monotonic at {i}");
            assert!((THROTTLE_MIN..=THROTTLE_MAX).contains(&value));
            last = value;
        }
        let mid = throttle_frame(0.5, false) >> 5;
        assert!((i32::from(mid) - 1048).abs() <= 1, "midpoint {mid} off");
    }
}
