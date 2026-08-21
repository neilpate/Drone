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

// ---------------------------------------------------------------------------
// Bidirectional DShot (eRPM telemetry returned on the signal wire).
// ---------------------------------------------------------------------------
//
// The outgoing frame is the same as `frame` but with an *inverted* CRC, and on
// the wire the signal *level* is inverted (idle high) -- that level inversion is
// the board driver's job, not this value. After sending, the FC releases the
// line and the ESC replies with a GCR-encoded 16-bit `[eRPM:CRC]` value.
// Capturing that reply (line turnaround + sampling ~21 bits) is board-specific;
// the pure decode of an already-captured bit value lives here.
//
// Encoding reference: BrushlessWhoop "DSHOT - the missing Handbook".

/// Build a bidirectional (inverted-CRC) DShot frame. Same bits as [`frame`]
/// except the 4-bit CRC is inverted, which -- with the ESC in bidirectional mode
/// -- asks it to reply with eRPM.
pub fn bidir_frame(value: u16, request_telemetry: bool) -> u16 {
    let payload = ((value & VALUE_MASK) << 1) | request_telemetry as u16;
    let crc = !(payload ^ (payload >> 4) ^ (payload >> 8)) & 0xF;
    (payload << 4) | crc
}

/// GCR 4-bit -> 5-bit quintet map the ESC uses to encode its eRPM reply.
const GCR_QUINTET: [u8; 16] = [
    0x19, 0x1B, 0x12, 0x13, 0x1D, 0x15, 0x16, 0x17, 0x1A, 0x09, 0x0A, 0x0B, 0x1E, 0x0D, 0x0E, 0x0F,
];

/// Inverse GCR map: a 5-bit quintet back to its nibble, or `None` if the quintet
/// is not a valid code (a corrupt capture).
fn quintet_to_nibble(quintet: u8) -> Option<u8> {
    GCR_QUINTET
        .iter()
        .position(|&q| q == quintet)
        .map(|i| i as u8)
}

/// Decode a captured bidirectional-DShot reply (`raw21`: the 21 sampled line
/// bits, MSB first) into the raw 16-bit `[eRPM:CRC]` value: reverse the
/// transition ("run-length") encoding, then un-map the four GCR quintets.
/// `None` if any quintet is invalid.
pub fn gcr_decode(raw21: u32) -> Option<u16> {
    // Undo the transition encoding: gcr = raw ^ (raw >> 1) (low 20 bits).
    let gcr = (raw21 ^ (raw21 >> 1)) & 0xF_FFFF;
    let mut value = 0u16;
    for quintet_index in (0..4u32).rev() {
        let quintet = ((gcr >> (quintet_index * 5)) & 0x1F) as u8;
        value = (value << 4) | u16::from(quintet_to_nibble(quintet)?);
    }
    Some(value)
}

/// Validate an eRPM reply's CRC and decode it to **electrical** RPM (divide by
/// the motor's pole-pair count for shaft RPM). `None` on CRC failure; a zero
/// period (motor stopped / no signal) yields `Some(0)`.
///
/// `value16` is `[3-bit exponent][9-bit mantissa][4-bit CRC]`; the period in µs
/// is `mantissa << exponent`, and eRPM is `60_000_000 / period_us`.
pub fn erpm(value16: u16) -> Option<u32> {
    let data = value16 >> 4;
    let crc = value16 & 0xF;
    // The eRPM CRC is the *uninverted* checksum over the 12-bit data field.
    if crc != (data ^ (data >> 4) ^ (data >> 8)) & 0xF {
        return None;
    }
    let exponent = (data >> 9) & 0x7;
    let mantissa = data & 0x1FF;
    let period_us = u32::from(mantissa) << exponent;
    // A zero period (motor stopped / no signal) yields 0 eRPM.
    Some(60_000_000u32.checked_div(period_us).unwrap_or(0))
}

/// Full receive path: decode a captured 21-bit reply straight to electrical RPM.
pub fn decode_erpm(raw21: u32) -> Option<u32> {
    erpm(gcr_decode(raw21)?)
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

    // ---- Bidirectional DShot ----

    /// Reference vector (BrushlessWhoop handbook): bidir frame for throttle 1046.
    #[test]
    fn bidir_frame_matches_reference() {
        assert_eq!(bidir_frame(1046, false), 0x82C9);
    }

    /// The GCR quintet map is a bijection: every code decodes back to its nibble.
    #[test]
    fn gcr_quintet_map_round_trips() {
        for nibble in 0u8..16 {
            assert_eq!(
                quintet_to_nibble(GCR_QUINTET[nibble as usize]),
                Some(nibble)
            );
        }
    }

    /// Test-only: map a 16-bit value to its 20-bit GCR (reference table).
    fn quintet_map(value16: u16) -> u32 {
        let mut gcr = 0u32;
        for i in (0..4u32).rev() {
            let nibble = ((value16 >> (i * 4)) & 0xF) as usize;
            gcr = (gcr << 5) | u32::from(GCR_QUINTET[nibble]);
        }
        gcr
    }

    /// Test-only: inverse of `gcr_decode`'s transition step (NRZI encode).
    fn transition_encode(gcr20: u32) -> u32 {
        let mut raw = 0u32;
        let mut higher = 0u32; // the leading start bit is 0
        for i in (0..20u32).rev() {
            let bit = ((gcr20 >> i) & 1) ^ higher;
            raw |= bit << i;
            higher = bit;
        }
        raw
    }

    /// Reference vector: 0x82C6 maps to GCR 0xD4BD6.
    #[test]
    fn quintet_map_matches_reference() {
        assert_eq!(quintet_map(0x82C6), 0xD4BD6);
    }

    /// A value GCR- and transition-encoded, then decoded, returns unchanged.
    #[test]
    fn gcr_decode_round_trips() {
        for value in [0x0000u16, 0x2640, 0x82C6, 0xFFFF, 0x1234, 0xABCD] {
            let raw = transition_encode(quintet_map(value));
            assert_eq!(
                gcr_decode(raw),
                Some(value),
                "round trip failed for {value:#06X}"
            );
        }
    }

    /// eRPM decode: reference-anchored period/rpm plus CRC and stopped handling.
    #[test]
    fn erpm_decodes_and_checks_crc() {
        // data=0x264 (exp=1, mant=100 -> 200 us) => 300000 eRPM.
        assert_eq!(erpm(0x2640), Some(300_000));
        // Corrupt CRC is rejected.
        assert_eq!(erpm(0x2641), None);
        // Zero period (stopped) reads as 0 eRPM.
        assert_eq!(erpm(0x0000), Some(0));
    }

    /// Full receive path from a captured 21-bit reply.
    #[test]
    fn decode_erpm_end_to_end() {
        let raw = transition_encode(quintet_map(0x2640));
        assert_eq!(decode_erpm(raw), Some(300_000));
    }
}
